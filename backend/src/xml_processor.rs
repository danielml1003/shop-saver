use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{path::Path, sync::mpsc, thread, time::Duration};
use tokio::fs;
use tracing::{error, info};
use sqlx::Row;

use crate::database::{DatabaseManager, is_ean13};
use crate::models::{XmlRoot, Item, StoresFullRoot};

pub struct XmlFileProcessor {
    db_manager: DatabaseManager,
    watch_directory: String,
}

impl XmlFileProcessor {
    pub fn new(db_manager: DatabaseManager, watch_directory: String) -> Self {
        Self {
            db_manager,
            watch_directory,
        }
    }

    pub async fn process_xml_file(&self, file_path: &Path) -> Result<()> {
        let file_path_str = file_path.to_string_lossy();
        info!("Processing XML file: {}", file_path_str);

        let filename = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        // Size cap on untrusted retailer XML so one huge/malicious file can't
        // exhaust memory (ARCHITECTURE.md §5.2). Largest real PriceFull files
        // are tens of MB uncompressed.
        let max_bytes: u64 = std::env::var("XML_MAX_BYTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(256 * 1024 * 1024);
        let size = fs::metadata(file_path).await?.len();
        if size > max_bytes {
            return Err(anyhow::anyhow!(
                "XML parsing error: file {} is {} bytes, over the {} byte cap",
                filename, size, max_bytes
            ));
        }

        let content = fs::read_to_string(file_path).await?;

        if filename.contains("storesfull") || filename.contains("stores") {
            match serde_xml_rs::from_str::<StoresFullRoot>(&content) {
                Ok(stores_data) => {
                    self.process_stores_full(stores_data).await?;
                    info!("Successfully processed StoresFull: {}", file_path_str);
                }
                Err(e) => {
                    error!("Failed to parse StoresFull XML {}: {}", file_path_str, e);
                    return Err(anyhow::anyhow!("StoresFull XML parsing error: {}", e));
                }
            }
        } else {
            match serde_xml_rs::from_str::<XmlRoot>(&content) {
                Ok(xml_data) => {
                    self.process_xml_data(xml_data, &file_path_str).await?;
                    info!("Successfully processed: {}", file_path_str);
                }
                Err(e) => {
                    error!("Failed to parse XML file {}: {}", file_path_str, e);
                    return Err(anyhow::anyhow!("XML parsing error: {}", e));
                }
            }
        }

        Ok(())
    }

    pub async fn scan_existing_files(&self) -> Result<()> {
        info!("Scanning existing XML files in: {}", self.watch_directory);

        let mut dir = fs::read_dir(&self.watch_directory).await?;
        let mut skipped = 0usize;
        let mut processed = 0usize;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("xml") {
                continue;
            }

            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let file_size = entry.metadata().await.map(|m| m.len() as i64).unwrap_or(0);

            // Skip promotional files — they have a different XML structure (Promotions, not Items)
            // and we don't need promo data. Mark immediately so they're not re-checked.
            let lower_fname = filename.to_lowercase();
            if lower_fname.contains("promo") || lower_fname.starts_with("nullpromo") {
                if let Err(e) = self.db_manager.mark_file_processed(&filename, file_size).await {
                    error!("Error marking promo file as skipped {}: {}", filename, e);
                }
                skipped += 1;
                continue;
            }

            // Skip files that were already processed with the same size
            match self.db_manager.is_file_processed(&filename, file_size).await {
                Ok(true) => { skipped += 1; continue; }
                Ok(false) => {}
                Err(e) => error!("Error checking processed status for {}: {}", filename, e),
            }

            match self.process_xml_file(&path).await {
                Ok(_) => {
                    processed += 1;
                    if let Err(e) = self.db_manager.mark_file_processed(&filename, file_size).await {
                        error!("Error marking file as processed {}: {}", filename, e);
                    }
                }
                Err(e) => {
                    error!("Error processing existing file {:?}: {}", path, e);
                    // Parse errors are permanent — the file will never succeed, so mark it
                    // done to avoid retry loops. Transient failures (DB connectivity etc.)
                    // stay unmarked so the next scan retries them.
                    if e.to_string().contains("parsing error") {
                        if let Err(e2) = self.db_manager.mark_file_processed(&filename, file_size).await {
                            error!("Error marking unparseable file {}: {}", filename, e2);
                        }
                    }
                }
            }
        }

        info!(
            "Scan complete: {} files processed, {} already-done files skipped",
            processed, skipped
        );
        Ok(())
    }

    pub fn start_file_watcher(&self) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        
        let mut watcher: RecommendedWatcher = Watcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    if let Err(e) = tx.send(event) {
                        error!("Error sending file event: {}", e);
                    }
                }
            },
            notify::Config::default(),
        )?;
        
        watcher.watch(Path::new(&self.watch_directory), RecursiveMode::NonRecursive)?;
        info!("Started watching directory: {}", self.watch_directory);
        
        let rt = tokio::runtime::Handle::current();
        
        let db_clone = self.db_manager.clone();
        let watch_dir = self.watch_directory.clone();

        thread::spawn(move || {
            loop {
                match rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(event) => {
                        if let EventKind::Create(_) | EventKind::Modify(_) = event.kind {
                            for path in event.paths {
                                if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                                    // Promo files have a different XML schema and are not ingested —
                                    // skip them here just like the startup scan does.
                                    let fname_lower = path.file_name()
                                        .map(|n| n.to_string_lossy().to_lowercase())
                                        .unwrap_or_default();
                                    if fname_lower.contains("promo") {
                                        continue;
                                    }
                                    info!("New/modified XML file detected: {:?}", path);

                                    let path_clone = path.clone();
                                    let db_for_task = db_clone.clone();
                                    let dir_for_task = watch_dir.clone();
                                    rt.spawn(async move {
                                        tokio::time::sleep(Duration::from_secs(2)).await;
                                        info!("File ready for processing: {:?}", path_clone);
                                        if let Ok(metadata) = tokio::fs::metadata(&path_clone).await {
                                            if metadata.is_file() {
                                                let processor = XmlFileProcessor { db_manager: db_for_task, watch_directory: dir_for_task };
                                                if let Err(e) = processor.process_xml_file(&path_clone).await {
                                                    error!("Error processing new file {:?}: {}", path_clone, e);
                                                } else {
                                                    let filename = path_clone.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                                                    let file_size = metadata.len() as i64;
                                                    if let Err(e) = processor.db_manager.mark_file_processed(&filename, file_size).await {
                                                        error!("Error marking {} as processed: {}", filename, e);
                                                    }
                                                }
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Normal timeout, continue loop
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        error!("File watcher channel disconnected");
                        break;
                    }
                }
            }
        });
        
        std::mem::forget(watcher);
        Ok(())
    }

    async fn process_stores_full(&self, stores_data: StoresFullRoot) -> Result<()> {
        let chain_id = &stores_data.chain_id;
        let mut updated = 0usize;

        for sub_chain in &stores_data.sub_chains.sub_chains {
            let sub_chain_id = sub_chain.sub_chain_id;
            for store in &sub_chain.stores.stores {
                match self
                    .db_manager
                    .update_store_from_stores_full(chain_id, sub_chain_id, store)
                    .await
                {
                    Ok(_) => updated += 1,
                    Err(e) => error!(
                        "Error upserting store {} for chain {}: {}",
                        store.store_id, chain_id, e
                    ),
                }
            }
        }

        info!("StoresFull: upserted {} stores for chain {}", updated, chain_id);
        Ok(())
    }

    async fn process_xml_data(&self, xml_data: XmlRoot, file_path: &str) -> Result<()> {
        info!("Processing XML data from file: {}", file_path);

        let store_id = self.insert_or_get_store(&xml_data).await?;

        let mut inserted = 0u64;
        let mut skipped = 0u64;
        let total = xml_data.items.items.len();

        for item in xml_data.items.items {
            match self.insert_item(store_id, &item, file_path).await {
                Ok(rows_affected) => {
                    if rows_affected > 0 {
                        inserted += 1;
                        // Populate the product catalog for barcode items
                        if is_ean13(&item.item_code) {
                            if let Err(e) = self.db_manager.upsert_product(&item).await {
                                error!("Error upserting product {}: {}", item.item_code, e);
                            }
                        }
                    } else {
                        skipped += 1;
                    }
                }
                Err(e) => error!("Error inserting item {}: {}", item.item_code, e),
            }
        }

        info!(
            "Inserted {}/{} items ({} already existed) from {}",
            inserted, total, skipped, file_path
        );

        Ok(())
    }

    async fn insert_or_get_store(&self, xml_data: &XmlRoot) -> Result<i32> {
        let result = sqlx::query(
            r#"
            INSERT INTO stores (chain_id, sub_chain_id, store_id, bikoret_no)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (chain_id, sub_chain_id, store_id)
            DO UPDATE SET bikoret_no = EXCLUDED.bikoret_no
            RETURNING id
            "#,
        )
        .bind(&xml_data.chain_id)
        .bind(xml_data.sub_chain_id)
        .bind(xml_data.store_id)
        .bind(xml_data.bikoret_no)
        .fetch_one(&self.db_manager.pool)
        .await?;

        Ok(result.get("id"))
    }

    async fn insert_item(&self, store_pk: i32, item: &Item, file_source: &str) -> Result<u64> {
        let price_update_date = self.db_manager.parse_datetime(&item.price_update_date)?;
        
        let item_price: f64 = item.item_price.parse()
            .map_err(|_| anyhow::anyhow!("Invalid item price: {}", item.item_price))?;
        
        let unit_of_measure_price: Option<f64> = item.unit_of_measure_price
            .as_ref()
            .and_then(|price| price.parse().ok());

        sqlx::query(
            r#"
            INSERT INTO items (
                store_pk, item_code, item_type, item_name, manufacturer_name,
                manufacture_country, manufacturer_item_description, unit_qty,
                quantity, unit_of_measure, is_weighted, qty_in_package,
                item_price, unit_of_measure_price, allow_discount, item_status,
                price_update_date, file_source
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            ON CONFLICT (store_pk, item_code, price_update_date) DO NOTHING
            "#,
        )
        .bind(store_pk)
        .bind(&item.item_code)
        .bind(item.item_type)
        .bind(&item.item_name)
        .bind(&item.manufacturer_name)
        .bind(&item.manufacture_country)
        .bind(&item.manufacturer_item_description)
        .bind(&item.unit_qty)
        .bind(&item.quantity)
        .bind(&item.unit_of_measure)
        .bind(item.is_weighted)
        .bind(&item.qty_in_package)
        .bind(item_price)
        .bind(unit_of_measure_price)
        .bind(item.allow_discount)
        .bind(item.item_status)
        .bind(price_update_date)
        .bind(file_source)
        .execute(&self.db_manager.pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(anyhow::Error::from)
    }
}

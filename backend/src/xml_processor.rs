use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{path::Path, sync::mpsc, thread, time::Duration};
use tokio::fs;
use tracing::{error, info, warn};

use crate::database::DatabaseManager;
use crate::models::{XmlRoot, Item};

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

        let content = fs::read_to_string(file_path).await?;
        
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

        Ok(())
    }

    pub async fn scan_existing_files(&self) -> Result<()> {
        info!("Scanning existing XML files in: {}", self.watch_directory);
        
        let mut dir = fs::read_dir(&self.watch_directory).await?;
        
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                if let Err(e) = self.process_xml_file(&path).await {
                    error!("Error processing existing file {:?}: {}", path, e);
                }
            }
        }
        
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
        
        thread::spawn(move || {
            loop {
                match rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(event) => {
                        if let EventKind::Create(_) | EventKind::Modify(_) = event.kind {
                            for path in event.paths {
                                if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                                    info!("New/modified XML file detected: {:?}", path);
                                    
                                    let path_clone = path.clone();
                                    rt.spawn(async move {
                                        tokio::time::sleep(Duration::from_secs(2)).await;
                                        info!("File ready for processing: {:?}", path_clone);
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

    async fn process_xml_data(&self, xml_data: XmlRoot, file_path: &str) -> Result<()> {
        info!("Processing XML data from file: {}", file_path);
        
        let store_id = self.insert_or_get_store(&xml_data).await?;
        
        let mut processed_count = 0;
        let mut skipped_count = 0;
        
        for item in xml_data.items.items {
            match self.insert_item(store_id, &item, file_path).await {
                Ok(_) => processed_count += 1,
                Err(e) => {
                    if e.to_string().contains("duplicate key") {
                        skipped_count += 1;
                    } else {
                        error!("Error inserting item {}: {}", item.item_code, e);
                    }
                }
            }
        }
        
        info!(
            "Processed {} items, skipped {} duplicates from {}",
            processed_count, skipped_count, file_path
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

    async fn insert_item(&self, store_pk: i32, item: &Item, file_source: &str) -> Result<()> {
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
        .await?;

        Ok(())
    }
}

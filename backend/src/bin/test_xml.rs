use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct XmlRoot {
    #[serde(rename = "ChainId")]
    chain_id: String,
    #[serde(rename = "SubChainId")]
    sub_chain_id: i32,
    #[serde(rename = "StoreId")]
    store_id: i32,
    #[serde(rename = "BikoretNo")]
    bikoret_no: Option<i32>,
    #[serde(rename = "Items")]
    items: Items,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Items {
    #[serde(rename = "Item")]
    items: Vec<Item>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Item {
    #[serde(rename = "PriceUpdateDate")]
    price_update_date: String,
    #[serde(rename = "ItemCode")]
    item_code: String,
    #[serde(rename = "ItemType")]
    item_type: i32,
    #[serde(rename = "ItemNm")]
    item_name: String,
    #[serde(rename = "ManufacturerName")]
    manufacturer_name: Option<String>,
    #[serde(rename = "ManufactureCountry")]
    manufacture_country: Option<String>,
    #[serde(rename = "ManufacturerItemDescription")]
    manufacturer_item_description: Option<String>,
    #[serde(rename = "UnitQty")]
    unit_qty: Option<String>,
    #[serde(rename = "Quantity")]
    quantity: Option<String>,
    #[serde(rename = "UnitOfMeasure")]
    unit_of_measure: Option<String>,
    #[serde(rename = "bIsWeighted")]
    is_weighted: Option<i32>,
    #[serde(rename = "QtyInPackage")]
    qty_in_package: Option<String>,
    #[serde(rename = "ItemPrice")]
    item_price: String,
    #[serde(rename = "UnitOfMeasurePrice")]
    unit_of_measure_price: Option<String>,
    #[serde(rename = "AllowDiscount")]
    allow_discount: Option<i32>,
    #[serde(rename = "ItemStatus")]
    item_status: Option<i32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xml_file = "../service/downloads/7290058108879-001-Price-202506031024.xml";
    
    println!("Testing XML parsing with file: {}", xml_file);
    
    match fs::read_to_string(xml_file) {
        Ok(content) => {
            match serde_xml_rs::from_str::<XmlRoot>(&content) {
                Ok(xml_data) => {
                    println!("✅ Successfully parsed XML!");
                    println!("Store: Chain {} - Sub {} - Store {}", 
                        xml_data.chain_id, 
                        xml_data.sub_chain_id, 
                        xml_data.store_id
                    );
                    println!("Found {} items", xml_data.items.items.len());
                    
                    // Show first few items
                    for (i, item) in xml_data.items.items.iter().take(3).enumerate() {
                        println!("Item {}: {} - {} ({})", 
                            i + 1, 
                            item.item_code, 
                            item.item_name, 
                            item.item_price
                        );
                    }
                    
                    println!("✅ XML parsing test completed successfully!");
                }
                Err(e) => {
                    println!("❌ Failed to parse XML: {}", e);
                    return Err(Box::new(e));
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to read XML file: {}", e);
            println!("Make sure the XML file exists at: {}", xml_file);
            return Err(Box::new(e));
        }
    }
    
    Ok(())
}

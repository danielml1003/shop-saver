use serde::{Deserialize, Serialize};

// XML Data Structures (for parsing price files)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct XmlRoot {
    #[serde(rename = "ChainId")]
    pub chain_id: String,
    #[serde(rename = "SubChainId")]
    pub sub_chain_id: i32,
    #[serde(rename = "StoreId")]
    pub store_id: i32,
    #[serde(rename = "BikoretNo")]
    pub bikoret_no: Option<i32>,
    #[serde(rename = "Items")]
    pub items: Items,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Items {
    #[serde(rename = "Item")]
    pub items: Vec<Item>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Item {
    #[serde(rename = "PriceUpdateDate")]
    pub price_update_date: String,
    #[serde(rename = "ItemCode")]
    pub item_code: String,
    #[serde(rename = "ItemType")]
    pub item_type: i32,
    #[serde(rename = "ItemNm")]
    pub item_name: String,
    #[serde(rename = "ManufacturerName")]
    pub manufacturer_name: Option<String>,
    #[serde(rename = "ManufactureCountry")]
    pub manufacture_country: Option<String>,
    #[serde(rename = "ManufacturerItemDescription")]
    pub manufacturer_item_description: Option<String>,
    #[serde(rename = "UnitQty")]
    pub unit_qty: Option<String>,
    #[serde(rename = "Quantity")]
    pub quantity: Option<String>,
    #[serde(rename = "UnitOfMeasure")]
    pub unit_of_measure: Option<String>,
    #[serde(rename = "bIsWeighted")]
    pub is_weighted: Option<i32>,
    #[serde(rename = "QtyInPackage")]
    pub qty_in_package: Option<String>,
    #[serde(rename = "ItemPrice")]
    pub item_price: String,
    #[serde(rename = "UnitOfMeasurePrice")]
    pub unit_of_measure_price: Option<String>,
    #[serde(rename = "AllowDiscount")]
    pub allow_discount: Option<i32>,
    #[serde(rename = "ItemStatus")]
    pub item_status: Option<i32>,
}

// API Request/Response Structures
#[derive(Debug, Deserialize)]
pub struct LocationQuery {
    pub latitude: f64,
    pub longitude: f64,
    pub radius_km: Option<f64>, // Default to 10km if not provided
}

#[derive(Debug, Deserialize)]
pub struct PriceComparisonRequest {
    pub user_location: LocationQuery,
    pub grocery_list: Vec<String>, // List of item names to search for
}

#[derive(Debug, Serialize, Clone)]
pub struct StoreInfo {
    pub id: i32,
    pub chain_id: String,
    pub sub_chain_id: i32,
    pub store_id: i32,
    pub address: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub distance_km: Option<f64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ItemPrice {
    pub item_code: String,
    pub item_name: String,
    pub price: f64,
    pub unit_of_measure: Option<String>,
    pub manufacturer_name: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct StoreComparison {
    pub store: StoreInfo,
    pub items: Vec<ItemPrice>,
    pub total_price: f64,
    pub items_found: usize,
    pub items_missing: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PriceComparisonResponse {
    pub stores: Vec<StoreComparison>,
    pub best_store: Option<StoreComparison>,
    pub requested_items: Vec<String>,
}

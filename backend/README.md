# Shop Saver XML Processor

A Rust server that monitors XML files containing shop item data and automatically processes them into a PostgreSQL database.

## Features

- **Real-time XML file monitoring** - Watches a directory for new/modified XML files
- **Automatic data processing** - Parses XML structure and extracts item information
- **PostgreSQL integration** - Stores processed data in structured database tables
- **Duplicate prevention** - Handles duplicate entries gracefully
- **Logging and monitoring** - Comprehensive logging for debugging and monitoring
- **Configurable** - Environment-based configuration

## Database Schema

### Stores Table
- `id` - Primary key (serial)
- `chain_id` - Store chain identifier
- `sub_chain_id` - Sub-chain identifier  
- `store_id` - Individual store identifier
- `bikoret_no` - Bikoret number (optional)
- `created_at` - Timestamp when record was created

### Items Table
- `id` - Primary key (UUID)
- `store_pk` - Foreign key to stores table
- `item_code` - Unique item code per store
- `item_type` - Type of item
- `item_name` - Item name (supports Hebrew)
- `manufacturer_name` - Manufacturer name
- `manufacture_country` - Country of manufacture
- `manufacturer_item_description` - Item description
- `unit_qty` - Unit quantity type
- `quantity` - Quantity value
- `unit_of_measure` - Unit of measurement
- `is_weighted` - Whether item is sold by weight
- `qty_in_package` - Quantity in package
- `item_price` - Item price (decimal)
- `unit_of_measure_price` - Price per unit of measure
- `allow_discount` - Whether discounts are allowed
- `item_status` - Item status
- `price_update_date` - When price was last updated
- `processed_at` - When record was processed
- `file_source` - Source XML file path

## Setup

### Prerequisites

1. **Rust** (latest stable version)
2. **PostgreSQL** database server
3. **XML files** to process

### Database Setup

1. Create a PostgreSQL database:
```sql
CREATE DATABASE shop_saver;
```

2. The application will automatically create the required tables on first run.

### Environment Configuration

1. Copy the example environment file:
```bash
cp .env.example .env
```

2. Edit `.env` with your configuration:
```env
DATABASE_URL=postgresql://username:password@localhost:5432/shop_saver
WATCH_DIRECTORY=../service/downloads
RUST_LOG=info
```

### Running the Server

1. Install dependencies and build:
```bash
cargo build --release
```

2. Run the server:
```bash
cargo run
```

Or run with custom environment variables:
```bash
DATABASE_URL="postgresql://user:pass@localhost:5432/db" WATCH_DIRECTORY="/path/to/xml/files" cargo run
```

## XML File Format

The server expects XML files with the following structure:
```xml
<Root>
  <ChainId>7290058108879</ChainId>
  <SubChainId>1</SubChainId>
  <StoreId>1</StoreId>
  <BikoretNo>6</BikoretNo>
  <Items>
    <Item>
      <PriceUpdateDate>2025-06-03 10:20:00</PriceUpdateDate>
      <ItemCode>6454</ItemCode>
      <ItemType>1</ItemType>
      <ItemNm>בלוק צ'דר אדום אירי  GRAND OR</ItemNm>
      <ManufacturerName>לא ידוע</ManufacturerName>
      <ManufactureCountry>IE</ManufactureCountry>
      <ManufacturerItemDescription>בלוק צ'דר אדום אירי  GRAND OR</ManufacturerItemDescription>
      <UnitQty>גרם</UnitQty>
      <Quantity>1000</Quantity>
      <UnitOfMeasure>100 גרם</UnitOfMeasure>
      <bIsWeighted>1</bIsWeighted>
      <QtyInPackage>8.0000</QtyInPackage>
      <ItemPrice>91</ItemPrice>
      <UnitOfMeasurePrice>91.0000</UnitOfMeasurePrice>
      <AllowDiscount>1</AllowDiscount>
      <ItemStatus>1</ItemStatus>
    </Item>
    <!-- More items... -->
  </Items>
</Root>
```

## Usage

1. **Start the server** - The server will begin monitoring the configured directory
2. **Add XML files** - Place XML files in the monitored directory
3. **Automatic processing** - Files are automatically detected and processed
4. **View logs** - Monitor the console for processing status and any errors
5. **Query database** - Use your preferred PostgreSQL client to query the processed data

## Querying the Data

Example queries for your frontend:

```sql
-- Get all items for a specific store
SELECT i.*, s.chain_id, s.store_id 
FROM items i 
JOIN stores s ON i.store_pk = s.id 
WHERE s.chain_id = '7290058108879' AND s.store_id = 1;

-- Get items by price range
SELECT item_name, item_price, manufacturer_name 
FROM items 
WHERE item_price BETWEEN 50 AND 100 
ORDER BY item_price;

-- Get latest prices for items
SELECT DISTINCT ON (item_code) 
    item_code, item_name, item_price, price_update_date
FROM items 
ORDER BY item_code, price_update_date DESC;
```

## API Integration

This server focuses on data processing. For frontend integration, you can:

1. **Direct database queries** - Connect your frontend directly to PostgreSQL
2. **Add REST API** - Extend this server with HTTP endpoints using `axum` or `warp`
3. **GraphQL API** - Add GraphQL support for flexible queries

## Troubleshooting

- **Database connection errors** - Check DATABASE_URL and ensure PostgreSQL is running
- **File permission errors** - Ensure the server has read access to the XML directory
- **XML parsing errors** - Check that XML files match the expected format
- **Duplicate key errors** - These are normal and indicate the item already exists

## Development

To extend the server:

1. **Add new XML fields** - Update the `Item` struct and database schema
2. **Add HTTP API** - Include `axum` dependency and add REST endpoints
3. **Add data validation** - Implement validation logic for item data
4. **Add metrics** - Include Prometheus metrics for monitoring

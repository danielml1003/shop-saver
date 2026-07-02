-- Initial schema — consolidated from the runtime CREATE TABLE code that used to
-- live in DatabaseManager::new (ARCHITECTURE.md §3.1). Every statement is
-- IF NOT EXISTS / ON CONFLICT so this migration is a no-op on databases that
-- were created by older versions of the app.

CREATE TABLE IF NOT EXISTS stores (
    id SERIAL PRIMARY KEY,
    chain_id VARCHAR NOT NULL,
    sub_chain_id INTEGER NOT NULL,
    store_id INTEGER NOT NULL,
    bikoret_no INTEGER,
    latitude DECIMAL(10, 8),
    longitude DECIMAL(11, 8),
    address TEXT,
    city VARCHAR(100),
    country VARCHAR(100),
    store_name VARCHAR(200),
    zip_code VARCHAR(20),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(chain_id, sub_chain_id, store_id)
);

-- Columns added over time (no-ops on fresh databases, needed for old ones)
ALTER TABLE stores ADD COLUMN IF NOT EXISTS store_name VARCHAR(200);
ALTER TABLE stores ADD COLUMN IF NOT EXISTS zip_code VARCHAR(20);

CREATE INDEX IF NOT EXISTS idx_stores_location ON stores(latitude, longitude);

CREATE TABLE IF NOT EXISTS items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    store_pk INTEGER REFERENCES stores(id),
    item_code VARCHAR NOT NULL,
    item_type INTEGER NOT NULL,
    item_name VARCHAR NOT NULL,
    manufacturer_name VARCHAR,
    manufacture_country VARCHAR,
    manufacturer_item_description VARCHAR,
    unit_qty VARCHAR,
    quantity VARCHAR,
    unit_of_measure VARCHAR,
    is_weighted INTEGER,
    qty_in_package VARCHAR,
    item_price DECIMAL(10,4) NOT NULL,
    unit_of_measure_price DECIMAL(10,4),
    allow_discount INTEGER,
    item_status INTEGER,
    price_update_date TIMESTAMP,
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    file_source VARCHAR,
    UNIQUE(store_pk, item_code, price_update_date)
);

-- Canonical product catalog — one row per unique EAN-13 barcode.
CREATE TABLE IF NOT EXISTS products (
    barcode VARCHAR(13) PRIMARY KEY,
    canonical_name VARCHAR NOT NULL,
    manufacturer VARCHAR,
    quantity VARCHAR,
    unit_of_measure VARCHAR,
    first_seen_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Trigram indexes for LIKE '%…%' substring search
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS idx_items_lower_name_trgm
    ON items USING gin (LOWER(item_name) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_products_lower_name_trgm
    ON products USING gin (LOWER(canonical_name) gin_trgm_ops);

-- Dedup log for ingested XML files
CREATE TABLE IF NOT EXISTS processed_files (
    filename VARCHAR PRIMARY KEY,
    file_size BIGINT NOT NULL,
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Chain ID → Hebrew display name fallback (used until StoresFull is ingested)
CREATE TABLE IF NOT EXISTS chain_names (
    chain_id VARCHAR PRIMARY KEY,
    display_name VARCHAR NOT NULL
);

INSERT INTO chain_names (chain_id, display_name) VALUES
    ('7290027600007', 'שופרסל'),
    ('7290058140886', 'רמי לוי'),
    ('7290055700007', 'קרפור'),
    ('7290058108879', 'קינג סטור'),
    ('7290058159628', 'מעיין 2000'),
    ('7290058197699', 'גוד פארם'),
    ('7290492000005', 'דור אלון'),
    ('7290873255550', 'טיב טעם'),
    ('7290696200003', 'ויקטורי'),
    ('7290058173198', 'זול ובגדול'),
    ('7290803800003', 'יוחננוף'),
    ('7290695900006', 'אושר עד'),
    ('7290633800006', 'AM:PM'),
    ('7290876100000', 'חצי חינם'),
    ('7290011900477', 'סופר-פארם')
ON CONFLICT (chain_id) DO UPDATE SET display_name = EXCLUDED.display_name;

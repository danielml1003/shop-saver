-- Retention policy (ARCHITECTURE.md §3.2 item 8):
--   items         = current price per (store, item)   — bounded size
--   price_history = one row per observed price change — powers the roadmap's
--                   price-history sparkline feature.

CREATE TABLE IF NOT EXISTS price_history (
    id BIGSERIAL PRIMARY KEY,
    store_pk INTEGER REFERENCES stores(id),
    item_code VARCHAR NOT NULL,
    item_price DECIMAL(10,4) NOT NULL,
    price_update_date TIMESTAMP NOT NULL,
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(store_pk, item_code, price_update_date)
);

CREATE INDEX IF NOT EXISTS idx_price_history_item
    ON price_history(store_pk, item_code, price_update_date DESC);

-- Preserve the history that has accumulated in items so far
INSERT INTO price_history (store_pk, item_code, item_price, price_update_date)
SELECT store_pk, item_code, item_price, price_update_date
FROM items
WHERE price_update_date IS NOT NULL
ON CONFLICT (store_pk, item_code, price_update_date) DO NOTHING;

-- Shrink items to the latest price per (store, item)
DELETE FROM items i
USING items j
WHERE i.store_pk = j.store_pk
  AND i.item_code = j.item_code
  AND (i.price_update_date < j.price_update_date
       OR (i.price_update_date = j.price_update_date AND i.id < j.id));

-- Replace the 3-column uniqueness with (store, item) — one current row each
ALTER TABLE items DROP CONSTRAINT IF EXISTS items_store_pk_item_code_price_update_date_key;
CREATE UNIQUE INDEX IF NOT EXISTS idx_items_store_item ON items(store_pk, item_code);

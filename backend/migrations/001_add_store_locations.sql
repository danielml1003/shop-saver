-- Add location columns to stores table
ALTER TABLE stores 
ADD COLUMN IF NOT EXISTS latitude DECIMAL(10, 8),
ADD COLUMN IF NOT EXISTS longitude DECIMAL(11, 8),
ADD COLUMN IF NOT EXISTS address TEXT,
ADD COLUMN IF NOT EXISTS city VARCHAR(100),
ADD COLUMN IF NOT EXISTS country VARCHAR(100);

-- Create index for location-based queries
CREATE INDEX IF NOT EXISTS idx_stores_location ON stores(latitude, longitude);

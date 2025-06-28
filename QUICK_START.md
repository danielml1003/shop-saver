# 🚀 Quick Start Guide - Shop Saver XML Processor

## What You Just Got

A powerful Rust server that:
- **Monitors XML files** in real-time
- **Parses Israeli shop data** (Hebrew support included!)
- **Stores everything in PostgreSQL** with proper schema
- **Handles 2,500+ items** per file (tested with your data!)
- **Prevents duplicates** automatically
- **Logs everything** for easy debugging

## 📋 Prerequisites

1. **PostgreSQL** - Download from https://www.postgresql.org/download/
2. **Rust** - Should already be installed

## ⚡ 5-Minute Setup

### 1. Setup Database
```sql
-- Connect to PostgreSQL and run:
CREATE DATABASE shop_saver;
```

### 2. Configure Environment
```powershell
cd backend
cp .env.example .env
# Edit .env with your PostgreSQL credentials
```

### 3. Test & Run
```powershell
# Test XML parsing (we verified this works!)
cargo run --bin test_xml

# Run the full server
cargo run
```

## 📊 What Happens Next

1. **Server starts** and creates database tables automatically
2. **Scans existing XML files** in `../service/downloads/`
3. **Processes your 2,588 items** from the test file
4. **Monitors for new files** in real-time
5. **Stores everything** in PostgreSQL

## 📈 Example Queries for Your Frontend

```sql
-- Get all items from a specific store
SELECT item_name, item_price, manufacturer_name 
FROM items i 
JOIN stores s ON i.store_pk = s.id 
WHERE s.chain_id = '7290058108879';

-- Find items by price range
SELECT item_name, item_price 
FROM items 
WHERE item_price BETWEEN 50 AND 100 
ORDER BY item_price;

-- Latest prices for each item
SELECT DISTINCT ON (item_code) 
    item_code, item_name, item_price, price_update_date
FROM items 
ORDER BY item_code, price_update_date DESC;
```

## 🔧 Configuration Options

Set these environment variables:

```env
DATABASE_URL=postgresql://username:password@localhost:5432/shop_saver
WATCH_DIRECTORY=../service/downloads
RUST_LOG=info
```

## 📱 Frontend Integration

Your frontend can:
1. **Connect directly** to PostgreSQL
2. **Query the items table** for your shop data
3. **Filter by store, price, category** etc.
4. **Get real-time updates** as new XML files are processed

## 🛠️ What's Inside

- **XML to PostgreSQL pipeline** ✅
- **Hebrew text support** ✅
- **Real-time file monitoring** ✅
- **Duplicate handling** ✅
- **Comprehensive logging** ✅
- **2,588 items tested and working** ✅

## 🎯 Next Steps

1. **Run the server** with your XML files
2. **Connect your frontend** to the PostgreSQL database
3. **Query the processed data** for your shop app
4. **Add more XML files** - they'll be processed automatically!

## 🔍 Monitoring

The server logs everything:
- ✅ Successfully processed files
- 📊 Number of items processed
- ⚠️ Skipped duplicates
- ❌ Any errors encountered

Your Israeli shop data with Hebrew names like "בלוק צ'דר אדום אירי" is fully supported! 🇮🇱

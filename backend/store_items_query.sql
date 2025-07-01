-- Query to show all stores and their items summary
SELECT 
    s.chain_id,
    s.sub_chain_id,
    s.store_id,
    s.bikoret_no,
    COUNT(i.id) as total_items,
    MIN(i.item_price) as cheapest_item,
    MAX(i.item_price) as most_expensive_item,
    ROUND(AVG(i.item_price)::NUMERIC, 2) as average_price,
    s.created_at
FROM stores s
LEFT JOIN items i ON s.id = i.store_pk
GROUP BY s.id, s.chain_id, s.sub_chain_id, s.store_id, s.bikoret_no, s.created_at
ORDER BY s.chain_id, s.sub_chain_id, s.store_id;

-- Query to show stores with sample items
SELECT 
    CONCAT(s.chain_id, '-', s.sub_chain_id, '-', s.store_id) as store_identifier,
    i.item_code,
    i.item_price,
    CASE 
        WHEN i.item_price > 100 THEN 'Expensive'
        WHEN i.item_price > 10 THEN 'Medium'
        ELSE 'Cheap'
    END as price_category,
    i.price_update_date
FROM stores s
JOIN items i ON s.id = i.store_pk
ORDER BY s.chain_id, s.sub_chain_id, s.store_id, i.item_price DESC;

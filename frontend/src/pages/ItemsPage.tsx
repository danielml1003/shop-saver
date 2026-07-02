import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Alert, Box, Button, CircularProgress, Container,
  Slider, TextField, Typography,
} from '@mui/material';
import AddShoppingCartIcon from '@mui/icons-material/AddShoppingCart';
import { apiService } from '../services/api';
import { StoreItemRow } from '../types';
import { useCart } from '../context/CartContext';

const PAGE_SIZE = 30;
const PRICE_MAX = 500;

const ItemsPage: React.FC = () => {
  const { addItem, contains } = useCart();
  const [items, setItems] = useState<StoreItemRow[]>([]);
  const [query, setQuery] = useState('');
  const [priceRange, setPriceRange] = useState<[number, number]>([0, PRICE_MAX]);
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searched, setSearched] = useState(false);

  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pageRef = useRef(page);
  const hasMoreRef = useRef(hasMore);
  const loadingMoreRef = useRef(loadingMore);
  const queryRef = useRef(query);
  const priceRangeRef = useRef(priceRange);

  useEffect(() => { pageRef.current = page; }, [page]);
  useEffect(() => { hasMoreRef.current = hasMore; }, [hasMore]);
  useEffect(() => { loadingMoreRef.current = loadingMore; }, [loadingMore]);
  useEffect(() => { queryRef.current = query; }, [query]);
  useEffect(() => { priceRangeRef.current = priceRange; }, [priceRange]);

  const fetchPage = useCallback(async (pageNum: number, append: boolean) => {
    const q = queryRef.current;
    const [min, max] = priceRangeRef.current;

    if (pageNum === 1) { setLoading(true); setError(null); }
    else setLoadingMore(true);

    try {
      const data = await apiService.searchItemsPaginated(
        q, min > 0 ? min : undefined, max < PRICE_MAX ? max : undefined, pageNum, PAGE_SIZE
      );
      setItems(prev => append ? [...prev, ...data.items] : data.items);
      setTotal(data.total);
      setHasMore(data.has_more);
      setPage(pageNum);
      setSearched(true);
    } catch {
      setError('שגיאה בחיפוש מוצרים');
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  }, []);

  const triggerSearch = useCallback(() => {
    setItems([]);
    setPage(1);
    fetchPage(1, false);
  }, [fetchPage]);

  // Initial load
  useEffect(() => { triggerSearch(); }, [triggerSearch]);

  const handleQueryChange = (val: string) => {
    setQuery(val);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      setItems([]);
      setPage(1);
      fetchPage(1, false);
    }, 400);
  };

  // Infinite scroll
  useEffect(() => {
    const el = sentinelRef.current;
    if (!el) return;
    const observer = new IntersectionObserver(entries => {
      if (entries[0].isIntersecting && hasMoreRef.current && !loadingMoreRef.current) {
        fetchPage(pageRef.current + 1, true);
      }
    }, { threshold: 0.1 });
    observer.observe(el);
    return () => observer.disconnect();
  }, [fetchPage, searched]);

  return (
    <Container maxWidth="md" sx={{ mt: 3, mb: 8 }}>
      <Typography variant="h5" fontWeight={700} sx={{ mb: 2 }}>חיפוש מוצרים</Typography>

      {/* Search bar */}
      <TextField
        fullWidth
        size="small"
        label="חפש מוצר"
        value={query}
        onChange={e => handleQueryChange(e.target.value)}
        onKeyDown={e => { if (e.key === 'Enter') triggerSearch(); }}
        sx={{ mb: 2 }}
      />

      {/* Price range */}
      <Box sx={{ px: 1, mb: 3 }}>
        <Typography variant="body2" color="text.secondary" gutterBottom>
          טווח מחיר: ₪{priceRange[0]} – ₪{priceRange[1] < PRICE_MAX ? priceRange[1] : `${PRICE_MAX}+`}
        </Typography>
        <Slider
          value={priceRange}
          onChange={(_, val) => setPriceRange(val as [number, number])}
          onChangeCommitted={() => triggerSearch()}
          min={0} max={PRICE_MAX} step={5}
          valueLabelDisplay="auto"
          valueLabelFormat={v => `₪${v}`}
        />
      </Box>

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', mt: 4 }}><CircularProgress /></Box>
      ) : (
        <>
          {searched && (
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {total.toLocaleString()} מוצרים · מציג {items.length}
            </Typography>
          )}

          {items.map(item => (
            <Box
              key={`${item.item_code}-${item.item_name}`}
              sx={{
                display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                py: 1.5, borderBottom: '1px solid', borderColor: 'divider',
              }}
            >
              <Box>
                <Typography variant="body1">{item.item_name}</Typography>
                {item.manufacturer_name && (
                  <Typography variant="caption" color="text.secondary">{item.manufacturer_name}</Typography>
                )}
              </Box>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flexShrink: 0 }}>
                <Typography variant="subtitle1" fontWeight={700}>₪{item.item_price.toFixed(2)}</Typography>
                <Button
                  size="small"
                  variant={contains(item.item_name) ? 'contained' : 'outlined'}
                  onClick={() => addItem({ barcode: null, name: item.item_name })}
                  startIcon={<AddShoppingCartIcon fontSize="small" />}
                  sx={{ minWidth: 0, px: 1 }}
                >
                  {contains(item.item_name) ? 'בסל' : 'הוסף'}
                </Button>
              </Box>
            </Box>
          ))}

          <div ref={sentinelRef} style={{ height: 1 }} />

          {loadingMore && (
            <Box sx={{ display: 'flex', justifyContent: 'center', py: 3 }}>
              <CircularProgress size={24} />
            </Box>
          )}

          {searched && !hasMore && items.length > 0 && !loadingMore && (
            <Typography variant="body2" color="text.secondary" textAlign="center" sx={{ mt: 2 }}>
              הוצגו כל {items.length} הפריטים
            </Typography>
          )}

          {searched && items.length === 0 && !loading && (
            <Typography variant="body1" color="text.secondary" textAlign="center" sx={{ mt: 4 }}>
              לא נמצאו מוצרים. נסה מילת חיפוש אחרת.
            </Typography>
          )}
        </>
      )}
    </Container>
  );
};

export default ItemsPage;

import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Alert, Box, Button, CircularProgress, Container,
  Divider, TextField, Typography,
} from '@mui/material';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import AddShoppingCartIcon from '@mui/icons-material/AddShoppingCart';
import { useNavigate, useParams } from 'react-router-dom';
import { apiService } from '../services/api';
import { BackendStoreInfo, StoreItemRow } from '../types';
import { useCart } from '../context/CartContext';

const PAGE_SIZE = 30;

const StoreDetailPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { addItem, contains } = useCart();

  const [store, setStore] = useState<BackendStoreInfo | null>(null);
  const [items, setItems] = useState<StoreItemRow[]>([]);
  const [query, setQuery] = useState('');
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Load store info from the stores list (no dedicated /api/stores/:id endpoint needed)
  useEffect(() => {
    if (!id) return;
    apiService.getAllStores().then(all => {
      const found = all.find(s => s.id === Number(id));
      setStore(found ?? null);
    }).catch(() => { /* header falls back to the store id */ });
  }, [id]);

  const loadItems = useCallback(async (pageNum: number, append: boolean, q: string) => {
    if (!id) return;
    if (pageNum === 1) setLoading(true); else setLoadingMore(true);
    try {
      const data = await apiService.getStoreItems(Number(id), q, pageNum, PAGE_SIZE);
      setItems(prev => append ? [...prev, ...data.items] : data.items);
      setTotal(data.total);
      setHasMore(data.has_more);
      setPage(pageNum);
      setError(null);
    } catch {
      setError('שגיאה בטעינת פריטים');
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  }, [id]);

  useEffect(() => { loadItems(1, false, ''); }, [loadItems]);

  // Debounced search
  const handleSearch = (val: string) => {
    setQuery(val);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => loadItems(1, false, val), 300);
  };

  // Infinite scroll sentinel
  useEffect(() => {
    const el = sentinelRef.current;
    if (!el) return;
    const observer = new IntersectionObserver(entries => {
      if (entries[0].isIntersecting && hasMore && !loadingMore) {
        loadItems(page + 1, true, query);
      }
    }, { threshold: 0.1 });
    observer.observe(el);
    return () => observer.disconnect();
  }, [hasMore, loadingMore, page, query, loadItems]);

  if (loading) {
    return <Box sx={{ display: 'flex', justifyContent: 'center', mt: 8 }}><CircularProgress /></Box>;
  }

  const storeLabel = store ? (store.store_name || store.chain_id) : `חנות ${id}`;
  const storeLocation = store ? [store.city, store.address].filter(Boolean).join(' · ') : '';

  return (
    <Container maxWidth="md" sx={{ mt: 3, mb: 8 }}>
      <Button startIcon={<ArrowBackIcon />} onClick={() => navigate('/stores')} sx={{ mb: 2 }}>
        חזרה לחנויות
      </Button>

      <Typography variant="h5" fontWeight={700}>{storeLabel}</Typography>
      {storeLocation && (
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>{storeLocation}</Typography>
      )}
      <Divider sx={{ mb: 2 }} />

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      <TextField
        fullWidth
        size="small"
        label="חפש מוצר בחנות זו"
        value={query}
        onChange={e => handleSearch(e.target.value)}
        sx={{ mb: 2 }}
      />

      <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
        {total.toLocaleString()} מוצרים · מציג {items.length}
      </Typography>

      {items.map(item => (
        <Box
          key={item.item_code}
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

      {!hasMore && items.length > 0 && !loadingMore && (
        <Typography variant="body2" color="text.secondary" textAlign="center" sx={{ mt: 2 }}>
          הוצגו כל {items.length} הפריטים
        </Typography>
      )}
    </Container>
  );
};

export default StoreDetailPage;

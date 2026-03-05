import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Autocomplete,
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  CircularProgress,
  Container,
  Divider,
  Stack,
  TextField,
  Typography,
} from '@mui/material';
import ShoppingCartIcon from '@mui/icons-material/ShoppingCart';
import LocationOnIcon from '@mui/icons-material/LocationOn';
import StarIcon from '@mui/icons-material/Star';
import { apiService } from '../services/api';
import { PriceComparisonRequest, PriceComparisonResponse, StoreComparison, UserLocation } from '../types';

const PAGE_SIZE = 10;

const ComparePage: React.FC = () => {
  // --- grocery bag state ---
  const [inputItem, setInputItem] = useState('');
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [items, setItems] = useState<string[]>([]);

  // --- location (optional) ---
  const [location, setLocation] = useState<UserLocation | null>(null);

  // --- results state ---
  const [results, setResults] = useState<StoreComparison[]>([]);
  const [bestStore, setBestStore] = useState<StoreComparison | null>(null);
  const [totalStores, setTotalStores] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [currentPage, setCurrentPage] = useState(1);
  const [searched, setSearched] = useState(false);

  // --- loading state ---
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // --- refs ---
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // keep latest items/page in refs so the IntersectionObserver closure stays fresh
  const itemsRef = useRef(items);
  const pageRef = useRef(currentPage);
  const hasMoreRef = useRef(hasMore);
  const loadingMoreRef = useRef(loadingMore);

  useEffect(() => { itemsRef.current = items; }, [items]);
  useEffect(() => { pageRef.current = currentPage; }, [currentPage]);
  useEffect(() => { hasMoreRef.current = hasMore; }, [hasMore]);
  useEffect(() => { loadingMoreRef.current = loadingMore; }, [loadingMore]);

  // --- autocomplete: debounce query ---
  const handleInputChange = (_: React.SyntheticEvent, value: string) => {
    setInputItem(value);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (value.length < 2) { setSuggestions([]); return; }
    debounceRef.current = setTimeout(async () => {
      try {
        const names = await apiService.searchItemNames(value);
        setSuggestions(names);
      } catch { /* ignore */ }
    }, 300);
  };

  const addItem = (name: string) => {
    const trimmed = name.trim();
    if (!trimmed || items.includes(trimmed)) return;
    setItems(prev => [...prev, trimmed]);
    setInputItem('');
    setSuggestions([]);
  };

  const removeItem = (name: string) => setItems(prev => prev.filter(i => i !== name));

  // --- geolocation ---
  const useGeolocation = () => {
    if (!navigator.geolocation) return;
    navigator.geolocation.getCurrentPosition(pos => {
      setLocation({ latitude: pos.coords.latitude, longitude: pos.coords.longitude, radius_km: 10 });
    });
  };

  // --- fetch a page of results ---
  const fetchPage = useCallback(async (page: number, append: boolean) => {
    const grocery = itemsRef.current;
    if (grocery.length === 0) return;

    if (page === 1) { setLoading(true); setError(null); }
    else setLoadingMore(true);

    try {
      const payload: PriceComparisonRequest = {
        grocery_list: grocery,
        page,
        page_size: PAGE_SIZE,
        ...(location ? { user_location: location } : {}),
      };
      const data: PriceComparisonResponse = await apiService.comparePrices(payload);

      if (append) {
        setResults(prev => [...prev, ...data.stores]);
      } else {
        setResults(data.stores);
        setBestStore(data.best_store ?? null);
        setSearched(true);
      }
      setTotalStores(data.total_stores);
      setHasMore(data.has_more);
      setCurrentPage(page);
    } catch (e: any) {
      const isNetwork = !e?.response;
      setError(isNetwork ? 'השרת לא זמין. ודא שהשרת רץ.' : 'שגיאה בהרצת ההשוואה.');
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  }, [location]);

  const runSearch = () => {
    setResults([]);
    setBestStore(null);
    setCurrentPage(1);
    setHasMore(false);
    fetchPage(1, false);
  };

  // --- IntersectionObserver for infinite scroll ---
  useEffect(() => {
    const el = sentinelRef.current;
    if (!el) return;

    const observer = new IntersectionObserver(entries => {
      if (
        entries[0].isIntersecting &&
        hasMoreRef.current &&
        !loadingMoreRef.current
      ) {
        fetchPage(pageRef.current + 1, true);
      }
    }, { threshold: 0.1 });

    observer.observe(el);
    return () => observer.disconnect();
  }, [fetchPage, searched]);

  // --- render a store card ---
  const renderCard = (sc: StoreComparison, isBest: boolean) => {
    const s = sc.store;
    const label = s.store_name || `${s.chain_id}-${s.store_id}`;
    const location_str = [s.city, s.address].filter(Boolean).join(' · ');

    return (
      <Card
        key={s.id}
        variant="outlined"
        sx={{
          mb: 2,
          borderColor: isBest ? 'success.main' : 'divider',
          borderWidth: isBest ? 2 : 1,
        }}
      >
        <CardContent>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', flexWrap: 'wrap', gap: 1 }}>
            <Box>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                {isBest && <StarIcon fontSize="small" color="success" />}
                <Typography variant="h6" component="span">{label}</Typography>
              </Box>
              {location_str && (
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mt: 0.5 }}>
                  <LocationOnIcon fontSize="small" color="action" />
                  <Typography variant="body2" color="text.secondary">{location_str}</Typography>
                </Box>
              )}
              {s.distance_km != null && (
                <Typography variant="body2" color="text.secondary">
                  {s.distance_km.toFixed(1)} ק"מ
                </Typography>
              )}
            </Box>
            <Box sx={{ textAlign: 'left' }}>
              <Typography variant="h5" color={isBest ? 'success.main' : 'text.primary'} fontWeight={700}>
                ₪{sc.total_price.toFixed(2)}
              </Typography>
              <Typography variant="caption" color="text.secondary">
                {sc.items_found} / {sc.items_found + sc.items_missing.length} פריטים
              </Typography>
            </Box>
          </Box>

          <Divider sx={{ my: 1.5 }} />

          <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap">
            {sc.items.map(it => (
              <Chip
                key={it.item_code}
                label={`${it.item_name} · ₪${it.price.toFixed(2)}`}
                size="small"
                color="default"
              />
            ))}
            {sc.items_missing.map(name => (
              <Chip key={name} label={name} size="small" color="warning" variant="outlined" />
            ))}
          </Stack>
        </CardContent>
      </Card>
    );
  };

  return (
    <Container maxWidth="md" sx={{ mt: 4, mb: 8 }}>
      {/* Title */}
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 3 }}>
        <ShoppingCartIcon color="primary" fontSize="large" />
        <Typography variant="h4" fontWeight={700}>סל הקניות שלי</Typography>
      </Box>

      {/* Item input */}
      <Box sx={{ display: 'flex', gap: 1, mb: 1 }}>
        <Autocomplete
          freeSolo
          options={suggestions}
          inputValue={inputItem}
          onInputChange={handleInputChange}
          onChange={(_, value) => { if (value) addItem(value as string); }}
          sx={{ flexGrow: 1 }}
          renderInput={(params) => (
            <TextField
              {...params}
              label="הוסף מוצר לסל"
              size="small"
              onKeyDown={(e) => { if (e.key === 'Enter' && inputItem.trim()) addItem(inputItem); }}
            />
          )}
          noOptionsText="לא נמצאו תוצאות"
        />
        <Button variant="contained" onClick={() => addItem(inputItem)} disabled={!inputItem.trim()}>
          הוסף
        </Button>
      </Box>

      {/* Item chips */}
      {items.length > 0 && (
        <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap" sx={{ mb: 2 }}>
          {items.map(name => (
            <Chip key={name} label={name} onDelete={() => removeItem(name)} color="primary" variant="outlined" />
          ))}
        </Stack>
      )}

      {/* Actions */}
      <Box sx={{ display: 'flex', gap: 2, alignItems: 'center', mb: 3, flexWrap: 'wrap' }}>
        <Button
          variant="contained"
          size="large"
          disabled={items.length === 0 || loading}
          onClick={runSearch}
          startIcon={loading ? <CircularProgress size={18} color="inherit" /> : undefined}
        >
          {loading ? 'מחפש...' : 'השווה מחירים'}
        </Button>
        <Button
          variant="outlined"
          size="small"
          onClick={useGeolocation}
          startIcon={<LocationOnIcon />}
          color={location ? 'success' : 'inherit'}
        >
          {location ? 'מיקום נבחר' : 'המיקום שלי (אופציונלי)'}
        </Button>
        {location && (
          <Button size="small" color="inherit" onClick={() => setLocation(null)}>הסר מיקום</Button>
        )}
      </Box>

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      {/* Results */}
      {searched && !loading && (
        <>
          {results.length === 0 ? (
            <Alert severity="info">לא נמצאו חנויות עם הפריטים המבוקשים.</Alert>
          ) : (
            <>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                נמצאו {totalStores} חנויות · מציג {results.length}
              </Typography>

              {bestStore && (
                <Alert severity="success" icon={<StarIcon />} sx={{ mb: 2 }}>
                  הכי משתלם: <strong>{bestStore.store.store_name || `${bestStore.store.chain_id}-${bestStore.store.store_id}`}</strong>
                  {bestStore.store.city ? ` · ${bestStore.store.city}` : ''}
                  {' · '}₪{bestStore.total_price.toFixed(2)}
                </Alert>
              )}

              {results.map((sc) => renderCard(sc, sc.store.id === bestStore?.store.id))}
            </>
          )}
        </>
      )}

      {/* Sentinel for infinite scroll */}
      {searched && <div ref={sentinelRef} style={{ height: 1 }} />}

      {/* Load more spinner */}
      {loadingMore && (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 3 }}>
          <CircularProgress />
        </Box>
      )}

      {/* End of results */}
      {searched && !hasMore && results.length > 0 && !loadingMore && (
        <Typography variant="body2" color="text.secondary" textAlign="center" sx={{ mt: 2 }}>
          הוצגו כל {results.length} החנויות
        </Typography>
      )}
    </Container>
  );
};

export default ComparePage;

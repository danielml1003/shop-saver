import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  CircularProgress,
  Container,
  Dialog,
  DialogContent,
  DialogTitle,
  Divider,
  IconButton,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableRow,
  TextField,
  Tooltip,
  Typography,
} from '@mui/material';
import ShoppingCartIcon from '@mui/icons-material/ShoppingCart';
import LocationOnIcon from '@mui/icons-material/LocationOn';
import StarIcon from '@mui/icons-material/Star';
import CameraAltIcon from '@mui/icons-material/CameraAlt';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import CheckIcon from '@mui/icons-material/Check';
import CloseIcon from '@mui/icons-material/Close';
import { apiService } from '../services/api';
import { useCart } from '../context/CartContext';
import {
  GroceryItem,
  PriceComparisonRequest,
  PriceComparisonResponse,
  ProductSearchResult,
  StoreComparison,
  UserLocation,
} from '../types';

// BarcodeDetector is a Web API not yet in the default TS lib — declare it inline.
declare class BarcodeDetector {
  constructor(options?: { formats?: string[] });
  detect(image: ImageBitmapSource): Promise<Array<{ rawValue: string; format: string }>>;
  static getSupportedFormats(): Promise<string[]>;
}

const PAGE_SIZE = 10;

// ---------------------------------------------------------------------------
// URL encoding helpers for share-list feature
// ---------------------------------------------------------------------------
function encodeList(items: GroceryItem[]): string {
  return btoa(encodeURIComponent(JSON.stringify(items)));
}

function decodeList(encoded: string): GroceryItem[] | null {
  try {
    return JSON.parse(decodeURIComponent(atob(encoded)));
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
const ComparePage: React.FC = () => {
  // --- grocery bag state (shared app-wide via CartContext) ---
  const [inputItem, setInputItem] = useState('');
  const [suggestions, setSuggestions] = useState<ProductSearchResult[]>([]);
  const { items, addItem: cartAddItem, removeItem: cartRemoveItem, setItems } = useCart();

  // --- location (optional) ---
  const [location, setLocation] = useState<UserLocation | null>(null);
  const [city, setCity] = useState('');
  const [gpsStatus, setGpsStatus] = useState<'idle' | 'active' | 'denied'>('idle');

  // --- results state ---
  const [results, setResults] = useState<StoreComparison[]>([]);
  const [bestStore, setBestStore] = useState<StoreComparison | null>(null);
  const [totalStores, setTotalStores] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [currentPage, setCurrentPage] = useState(1);
  const [searched, setSearched] = useState(false);

  // --- loading ---
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // --- share ---
  const [shareCopied, setShareCopied] = useState(false);

  // --- barcode scanner ---
  const [scanOpen, setScanOpen] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const [scannerSupported, setScannerSupported] = useState(false);
  const videoRef = useRef<HTMLVideoElement>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const scanFrameRef = useRef<number>(0);

  // --- refs to keep closures fresh ---
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const itemsRef = useRef<GroceryItem[]>(items);
  const pageRef = useRef(currentPage);
  const hasMoreRef = useRef(hasMore);
  const loadingMoreRef = useRef(loadingMore);

  useEffect(() => { itemsRef.current = items; }, [items]);
  useEffect(() => { pageRef.current = currentPage; }, [currentPage]);
  useEffect(() => { hasMoreRef.current = hasMore; }, [hasMore]);
  useEffect(() => { loadingMoreRef.current = loadingMore; }, [loadingMore]);

  // ---------------------------------------------------------------------------
  // On mount: restore list from URL (?q=...) if present
  // ---------------------------------------------------------------------------
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const q = params.get('q');
    if (q) {
      const decoded = decodeList(q);
      if (decoded && decoded.length > 0) setItems(decoded);
    }
    // Check barcode scanner support
    if (typeof BarcodeDetector !== 'undefined') {
      setScannerSupported(true);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Auto-detect GPS on mount — falls back to the city text input when denied
  useEffect(() => {
    if (!navigator.geolocation) { setGpsStatus('denied'); return; }
    navigator.geolocation.getCurrentPosition(
      pos => {
        setLocation({ latitude: pos.coords.latitude, longitude: pos.coords.longitude, radius_km: 10 });
        setGpsStatus('active');
      },
      () => setGpsStatus('denied')
    );
  }, []);

  // ---------------------------------------------------------------------------
  // Autocomplete
  // ---------------------------------------------------------------------------
  const handleInputChange = (_: React.SyntheticEvent, value: string) => {
    setInputItem(value);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (value.length < 2) { setSuggestions([]); return; }
    debounceRef.current = setTimeout(async () => {
      try {
        const res = await apiService.searchItemNames(value);
        setSuggestions(res);
      } catch { /* ignore */ }
    }, 300);
  };

  const addItem = (input: ProductSearchResult | string) => {
    const grocery: GroceryItem = typeof input === 'string'
      ? { barcode: null, name: input.trim() }
      : { barcode: input.barcode, name: input.name };

    if (!grocery.name || items.some(i =>
      (i.barcode && i.barcode === grocery.barcode) || i.name === grocery.name
    )) return;
    cartAddItem(grocery);
    setInputItem('');
    setSuggestions([]);
  };

  const removeItem = (name: string) => cartRemoveItem(name);

  // Resolve a grocery list term (may be a barcode) back to a display name.
  const resolveTermName = (term: string): string => {
    const found = items.find(i => i.barcode === term || i.name === term);
    return found ? found.name : term;
  };

  // ---------------------------------------------------------------------------
  // Share list
  // ---------------------------------------------------------------------------
  const shareList = async () => {
    const encoded = encodeList(items);
    const url = `${window.location.origin}${window.location.pathname}?q=${encoded}`;
    try {
      await navigator.clipboard.writeText(url);
      setShareCopied(true);
      setTimeout(() => setShareCopied(false), 2500);
    } catch {
      // Clipboard API blocked — fall back to selecting the URL manually
      window.prompt('העתק את הקישור:', url);
    }
  };

  // ---------------------------------------------------------------------------
  // Barcode scanner
  // ---------------------------------------------------------------------------
  const stopScan = useCallback(() => {
    cancelAnimationFrame(scanFrameRef.current);
    streamRef.current?.getTracks().forEach(t => t.stop());
    streamRef.current = null;
    setScanOpen(false);
    setScanError(null);
  }, []);

  const startScan = async () => {
    setScanError(null);
    setScanOpen(true);
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: 'environment' },
      });
      streamRef.current = stream;
      if (videoRef.current) {
        videoRef.current.srcObject = stream;
        await videoRef.current.play();
        runScanLoop();
      }
    } catch (e: any) {
      setScanError('לא ניתן לגשת למצלמה. בדוק הרשאות.');
    }
  };

  const runScanLoop = () => {
    const detector = new BarcodeDetector({
      formats: ['ean_13', 'ean_8', 'upc_a', 'upc_e', 'qr_code'],
    });

    const loop = async () => {
      if (!videoRef.current || videoRef.current.readyState < 2) {
        scanFrameRef.current = requestAnimationFrame(loop);
        return;
      }
      try {
        const barcodes = await detector.detect(videoRef.current);
        if (barcodes.length > 0) {
          const raw = barcodes[0].rawValue;
          stopScan();
          // Try to resolve barcode to a product name via the search API
          try {
            const found = await apiService.searchItemNames(raw);
            if (found.length > 0) {
              addItem(found[0]);
            } else {
              addItem({ barcode: raw, name: raw });
            }
          } catch {
            addItem({ barcode: raw, name: raw });
          }
          return;
        }
      } catch { /* detect can throw on some frames */ }
      scanFrameRef.current = requestAnimationFrame(loop);
    };

    scanFrameRef.current = requestAnimationFrame(loop);
  };

  // Clean up camera if dialog is closed externally
  useEffect(() => {
    if (!scanOpen) stopScan();
  }, [scanOpen, stopScan]);

  // ---------------------------------------------------------------------------
  // Geolocation
  // ---------------------------------------------------------------------------
  const useGeolocation = () => {
    if (!navigator.geolocation) return;
    navigator.geolocation.getCurrentPosition(
      pos => {
        setLocation({ latitude: pos.coords.latitude, longitude: pos.coords.longitude, radius_km: 10 });
        setGpsStatus('active');
      },
      () => setGpsStatus('denied')
    );
  };

  // ---------------------------------------------------------------------------
  // Fetch results (paginated)
  // ---------------------------------------------------------------------------
  const fetchPage = useCallback(async (page: number, append: boolean) => {
    const grocery = itemsRef.current;
    if (grocery.length === 0) return;

    if (page === 1) { setLoading(true); setError(null); }
    else setLoadingMore(true);

    try {
      const payload: PriceComparisonRequest = {
        grocery_list: grocery.map(i => i.barcode ?? i.name),
        page,
        page_size: PAGE_SIZE,
        ...(location ? { user_location: location } : {}),
        ...(!location && city.trim() ? { city: city.trim() } : {}),
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
  }, [location, city]);

  const runSearch = () => {
    setResults([]);
    setBestStore(null);
    setCurrentPage(1);
    setHasMore(false);
    fetchPage(1, false);
  };

  // Infinite scroll sentinel
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

  // ---------------------------------------------------------------------------
  // Render a store comparison card
  // ---------------------------------------------------------------------------
  const renderCard = (sc: StoreComparison, isBest: boolean) => {
    const s = sc.store;
    const label = s.store_name || `${s.chain_id} — סניף ${s.store_id}`;
    const locationStr = [s.city, s.address].filter(Boolean).join(' · ');

    return (
      <Card
        key={s.id}
        variant="outlined"
        sx={{ mb: 2, borderColor: isBest ? 'success.main' : 'divider', borderWidth: isBest ? 2 : 1 }}
      >
        <CardContent>
          {/* Store header row */}
          <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', flexWrap: 'wrap', gap: 1 }}>
            <Box>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                {isBest && <StarIcon fontSize="small" color="success" />}
                <Typography variant="h6" component="span">{label}</Typography>
              </Box>
              {locationStr && (
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mt: 0.25 }}>
                  <LocationOnIcon fontSize="small" color="action" />
                  <Typography variant="body2" color="text.secondary">{locationStr}</Typography>
                </Box>
              )}
              {s.distance_km != null && (
                <Typography variant="body2" color="text.secondary">
                  {s.distance_km.toFixed(1)} ק"מ
                </Typography>
              )}
            </Box>

            {/* Total price */}
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

          {/* Per-item price breakdown */}
          <Table size="small" sx={{ '& td': { border: 'none', py: 0.4, px: 0.5 } }}>
            <TableBody>
              {sc.items.map(it => (
                <TableRow key={it.item_code}>
                  <TableCell sx={{ color: 'text.primary' }}>
                    <Typography variant="body2" noWrap sx={{ maxWidth: 260 }}>{it.item_name}</Typography>
                    {it.manufacturer_name && (
                      <Typography variant="caption" color="text.secondary">{it.manufacturer_name}</Typography>
                    )}
                  </TableCell>
                  <TableCell align="left" sx={{ whiteSpace: 'nowrap' }}>
                    <Typography variant="body2" fontWeight={600} color={isBest ? 'success.main' : 'text.primary'}>
                      ₪{it.price.toFixed(2)}
                    </Typography>
                  </TableCell>
                </TableRow>
              ))}
              {sc.items_missing.map(term => (
                <TableRow key={term}>
                  <TableCell colSpan={2}>
                    <Typography variant="body2" color="warning.main">
                      {resolveTermName(term)} — לא נמצא בסניף זה
                    </Typography>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    );
  };

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------
  return (
    <Container maxWidth="md" sx={{ mt: 4, mb: 8 }}>

      {/* Title */}
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 3 }}>
        <ShoppingCartIcon color="primary" fontSize="large" />
        <Typography variant="h4" fontWeight={700}>סל הקניות שלי</Typography>
      </Box>

      {/* Input row */}
      <Box sx={{ display: 'flex', gap: 1, mb: 1 }}>
        <Autocomplete
          freeSolo
          options={suggestions}
          getOptionLabel={(option) => typeof option === 'string' ? option : option.name}
          inputValue={inputItem}
          onInputChange={handleInputChange}
          onChange={(_, value) => { if (value) addItem(value as ProductSearchResult | string); }}
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
        {scannerSupported && (
          <Tooltip title="סרוק ברקוד">
            <IconButton onClick={startScan} color="primary" aria-label="סרוק ברקוד">
              <CameraAltIcon />
            </IconButton>
          </Tooltip>
        )}
      </Box>

      {/* Item chips */}
      {items.length > 0 && (
        <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap" sx={{ mb: 2 }}>
          {items.map(item => (
            <Chip
              key={item.barcode ?? item.name}
              label={item.name}
              onDelete={() => removeItem(item.name)}
              color="primary"
              variant="outlined"
            />
          ))}
        </Stack>
      )}

      {/* Action row */}
      <Box sx={{ display: 'flex', gap: 1.5, alignItems: 'center', mb: 3, flexWrap: 'wrap' }}>
        <Button
          variant="contained"
          size="large"
          disabled={items.length === 0 || loading}
          onClick={runSearch}
          startIcon={loading ? <CircularProgress size={18} color="inherit" /> : undefined}
        >
          {loading ? 'מחפש...' : 'השווה מחירים'}
        </Button>

        {gpsStatus === 'active' && location ? (
          <Chip
            icon={<LocationOnIcon />}
            label={`GPS פעיל · ${location.radius_km ?? 10} ק"מ`}
            color="success"
            variant="outlined"
            onDelete={() => { setLocation(null); setGpsStatus('denied'); }}
          />
        ) : (
          <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
            <TextField
              label="עיר (אופציונלי)"
              size="small"
              value={city}
              onChange={e => setCity(e.target.value)}
              placeholder="לדוגמה: תל אביב"
              sx={{ width: 180 }}
              InputProps={{
                startAdornment: <LocationOnIcon sx={{ color: 'action.active', mr: 0.5 }} fontSize="small" />,
              }}
            />
            <Button
              variant="outlined"
              size="small"
              onClick={useGeolocation}
              startIcon={<LocationOnIcon />}
              color="inherit"
            >
              המיקום שלי
            </Button>
          </Box>
        )}

        {items.length > 0 && (
          <Tooltip title={shareCopied ? 'הקישור הועתק!' : 'שתף רשימה'}>
            <Button
              variant="outlined"
              size="small"
              onClick={shareList}
              startIcon={shareCopied ? <CheckIcon /> : <ContentCopyIcon />}
              color={shareCopied ? 'success' : 'inherit'}
            >
              שתף רשימה
            </Button>
          </Tooltip>
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
                  הכי משתלם:{' '}
                  <strong>
                    {bestStore.store.store_name || `${bestStore.store.chain_id}-${bestStore.store.store_id}`}
                  </strong>
                  {bestStore.store.city ? ` · ${bestStore.store.city}` : ''}
                  {' · '}₪{bestStore.total_price.toFixed(2)}
                </Alert>
              )}

              {results.map(sc => renderCard(sc, sc.store.id === bestStore?.store.id))}
            </>
          )}
        </>
      )}

      {/* Infinite scroll sentinel */}
      {searched && <div ref={sentinelRef} style={{ height: 1 }} />}

      {loadingMore && (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 3 }}>
          <CircularProgress />
        </Box>
      )}

      {searched && !hasMore && results.length > 0 && !loadingMore && (
        <Typography variant="body2" color="text.secondary" textAlign="center" sx={{ mt: 2 }}>
          הוצגו כל {results.length} החנויות
        </Typography>
      )}

      {/* Barcode scanner dialog */}
      <Dialog open={scanOpen} onClose={stopScan} maxWidth="sm" fullWidth>
        <DialogTitle sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          סרוק ברקוד מוצר
          <IconButton onClick={stopScan} size="small">
            <CloseIcon />
          </IconButton>
        </DialogTitle>
        <DialogContent sx={{ p: 0, display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
          {scanError ? (
            <Alert severity="error" sx={{ m: 2, width: '100%' }}>{scanError}</Alert>
          ) : (
            <>
              <video
                ref={videoRef}
                style={{ width: '100%', maxHeight: 360, objectFit: 'cover', background: '#000' }}
                playsInline
                muted
              />
              <Typography variant="caption" color="text.secondary" sx={{ py: 1.5 }}>
                כוון את המצלמה לברקוד — הסריקה תתבצע אוטומטית
              </Typography>
            </>
          )}
        </DialogContent>
      </Dialog>
    </Container>
  );
};

export default ComparePage;

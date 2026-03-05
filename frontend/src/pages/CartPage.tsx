import React, { useEffect, useMemo, useState } from 'react';
import { Container, Typography, Box, Chip, Stack, Button, GridLegacy as Grid, Card, CardContent, Divider, Alert } from '@mui/material';
import { useCart } from '../context/CartContext';
import { apiService } from '../services/api';
import { PriceComparisonRequest, PriceComparisonResponse, StoreComparison, UserLocation } from '../types';

const defaultLocation: UserLocation = {
  latitude: 32.0853,
  longitude: 34.7818,
  radius_km: 15,
};

const CartPage: React.FC = () => {
  const { items, removeItem, clearCart } = useCart();
  const [location, setLocation] = useState<UserLocation>(defaultLocation);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<PriceComparisonResponse | null>(null);

  const groceryList = useMemo(() => items.map(i => i.item_name), [items]);
  const canCompare = useMemo(() => groceryList.length > 0 && !loading, [groceryList.length, loading]);

  const useGeolocation = () => {
    if (!navigator.geolocation) return;
    navigator.geolocation.getCurrentPosition(pos => {
      setLocation(prev => ({ ...prev, latitude: pos.coords.latitude, longitude: pos.coords.longitude }));
    });
  };

  const runCompare = async () => {
    if (groceryList.length === 0) return;
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const payload: PriceComparisonRequest = { user_location: location, grocery_list: groceryList };
      const data = await apiService.comparePrices(payload);
      setResult(data);
    } catch (e: any) {
      console.error(e);
      const isNetwork = !e?.response;
      setError(isNetwork ? 'השרת לא זמין. ודא שהשרת רץ ושכתובת ה-API נכונה.' : 'שגיאה בהרצת ההשוואה.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (groceryList.length > 0) {
      runCompare();
    } else {
      setResult(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [groceryList.join('|'), location.latitude, location.longitude, location.radius_km]);

  const renderStoreCard = (sc: StoreComparison) => (
    <Card key={`${sc.store.id}`} sx={{ height: '100%' }}>
      <CardContent>
        <Typography variant="h6" gutterBottom>
          {sc.store.chain_id}-{sc.store.store_id} {sc.store.city ? `· ${sc.store.city}` : ''}
        </Typography>
        <Typography variant="body1" sx={{ mt: 1, fontWeight: 600 }}>
          מחיר כולל לסל: ₪{sc.total_price.toFixed(2)} · נמצאו {sc.items_found}
        </Typography>
        <Divider sx={{ my: 1.5 }} />
        <Typography variant="subtitle2">פריטים בסל</Typography>
        <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap" sx={{ my: 1 }}>
          {sc.items.map(it => (
            <Chip key={it.item_code} label={`${it.item_name} · ₪${it.price.toFixed(2)}`} size="small" />
          ))}
        </Stack>
        {sc.items_missing.length > 0 && (
          <>
            <Typography variant="subtitle2">פריטים שלא נמצאו</Typography>
            <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap" sx={{ mt: 1 }}>
              {sc.items_missing.map(name => (
                <Chip key={name} label={name} color="warning" size="small" />
              ))}
            </Stack>
          </>
        )}
      </CardContent>
    </Card>
  );

  return (
    <Container maxWidth="lg" sx={{ mt: 4, mb: 6 }}>
      <Typography variant="h4" gutterBottom>
        סל הקניות שלי
      </Typography>

      {items.length === 0 ? (
        <Alert severity="info">הסל ריק. לחץ על מוצר כדי להוסיף אותו.</Alert>
      ) : (
        <>
          <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap" sx={{ mb: 2 }}>
            {items.map(i => (
              <Chip key={i.item_code} label={i.item_name} onDelete={() => removeItem(i.item_code)} />
            ))}
          </Stack>

          <Box sx={{ display: 'flex', gap: 1, mb: 2 }}>
            <Button variant="outlined" onClick={useGeolocation}>המיקום שלי</Button>
            <Button variant="contained" disabled={!canCompare} onClick={runCompare}>
              מצא את החנות הזולה
            </Button>
            <Button color="error" onClick={clearCart}>נקה סל</Button>
          </Box>

          {error && <Alert severity="error" sx={{ mt: 1 }}>{error}</Alert>}

          {result && (
            <Box sx={{ mt: 3 }}>
              {result.best_store && (
                <Alert severity="success" sx={{ mb: 2 }}>
                  החנות המשתלמת: רשת {result.best_store.store.chain_id}-{result.best_store.store.store_id} · מחיר כולל ₪{result.best_store.total_price.toFixed(2)}
                </Alert>
              )}
              <Grid container spacing={2}>
                {result.stores.map(renderStoreCard)}
              </Grid>
            </Box>
          )}
        </>
      )}
    </Container>
  );
};

export default CartPage;



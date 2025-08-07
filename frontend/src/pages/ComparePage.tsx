import React, { useMemo, useState } from 'react';
import { Container, Box, Typography, TextField, Button, Chip, Stack, GridLegacy as Grid, Card, CardContent, Divider, Alert } from '@mui/material';
import { apiService } from '../services/api';
import { PriceComparisonRequest, PriceComparisonResponse, UserLocation, StoreComparison } from '../types';

const defaultLocation: UserLocation = {
  latitude: 32.0853,
  longitude: 34.7818,
  radius_km: 15,
};

const ComparePage: React.FC = () => {
  const [location, setLocation] = useState<UserLocation>(defaultLocation);
  const [inputItem, setInputItem] = useState('');
  const [items, setItems] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<PriceComparisonResponse | null>(null);

  const canCompare = useMemo(() => items.length > 0 && !loading, [items, loading]);

  const addItem = () => {
    const trimmed = inputItem.trim();
    if (!trimmed) return;
    if (!items.includes(trimmed)) setItems(prev => [...prev, trimmed]);
    setInputItem('');
  };

  const removeItem = (name: string) => {
    setItems(prev => prev.filter(i => i !== name));
  };

  const useGeolocation = () => {
    if (!navigator.geolocation) return;
    navigator.geolocation.getCurrentPosition(pos => {
      setLocation(prev => ({ ...prev, latitude: pos.coords.latitude, longitude: pos.coords.longitude }));
    });
  };

  const runCompare = async () => {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const payload: PriceComparisonRequest = {
        user_location: location,
        grocery_list: items,
      };
      const data = await apiService.comparePrices(payload);
      setResult(data);
    } catch (e: any) {
      console.error(e);
      setError('שגיאה בהרצת ההשוואה.');
    } finally {
      setLoading(false);
    }
  };

  const renderStoreCard = (sc: StoreComparison) => (
    <Card key={`${sc.store.id}`} sx={{ height: '100%' }}>
      <CardContent>
        <Typography variant="h6" gutterBottom>
          {sc.store.chain_id}-{sc.store.store_id} {sc.store.city ? `· ${sc.store.city}` : ''}
        </Typography>
        <Typography variant="body2" color="text.secondary">
          מרחק: {sc.store.distance_km?.toFixed(2) ?? '-'} ק"מ
        </Typography>
        <Typography variant="body1" sx={{ mt: 1, fontWeight: 600 }}>
          מחיר כולל: ₪{sc.total_price.toFixed(2)} · נמצאו {sc.items_found} פריטים
        </Typography>
        <Divider sx={{ my: 1.5 }} />
        <Typography variant="subtitle2">פריטים שנמצאו</Typography>
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
        השוואת סל קניות
      </Typography>

      <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap', alignItems: 'center', mb: 2 }}>
        <TextField
          label="קו רוחב"
          type="number"
          size="small"
          value={location.latitude}
          onChange={(e) => setLocation(prev => ({ ...prev, latitude: Number(e.target.value) }))}
        />
        <TextField
          label="קו אורך"
          type="number"
          size="small"
          value={location.longitude}
          onChange={(e) => setLocation(prev => ({ ...prev, longitude: Number(e.target.value) }))}
        />
        <TextField
          label={'רדיוס (ק"מ)'}
          type="number"
          size="small"
          value={location.radius_km ?? ''}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => setLocation(prev => ({ ...prev, radius_km: Number(e.target.value) }))}
        />
        <Button variant="outlined" onClick={useGeolocation}>המיקום שלי</Button>
      </Box>

      <Box sx={{ display: 'flex', gap: 1, alignItems: 'center', mb: 1 }}>
        <TextField
          label="הוסף מוצר לסל"
          size="small"
          fullWidth
          value={inputItem}
          onChange={(e) => setInputItem(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') addItem(); }}
        />
        <Button variant="contained" onClick={addItem}>הוסף</Button>
      </Box>

      {items.length > 0 && (
        <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap" sx={{ mb: 2 }}>
          {items.map(name => (
            <Chip key={name} label={name} onDelete={() => removeItem(name)} />
          ))}
        </Stack>
      )}

      <Button variant="contained" disabled={!canCompare} onClick={runCompare}>
        השווה מחירים
      </Button>

      {error && (
        <Alert severity="error" sx={{ mt: 2 }}>{error}</Alert>
      )}

      {result && (
        <Box sx={{ mt: 4 }}>
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
    </Container>
  );
};

export default ComparePage;



import React, { useEffect, useMemo, useState } from 'react';
import {
  Alert, Box, Card, CardActionArea, CardContent,
  CircularProgress, Container, TextField, Typography,
} from '@mui/material';
import LocationOnIcon from '@mui/icons-material/LocationOn';
import { MapContainer, TileLayer, Marker, Popup } from 'react-leaflet';
import { useNavigate } from 'react-router-dom';
import { apiService } from '../services/api';
import { BackendStoreInfo } from '../types';

const StoresPage: React.FC = () => {
  const navigate = useNavigate();
  const [stores, setStores] = useState<BackendStoreInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');

  useEffect(() => {
    apiService.getAllStores()
      .then(setStores)
      .catch(() => setError('שגיאה בטעינת החנויות'))
      .finally(() => setLoading(false));
  }, []);

  const filtered = useMemo(() => {
    if (!search.trim()) return stores;
    const q = search.toLowerCase();
    return stores.filter(s =>
      (s.store_name ?? '').toLowerCase().includes(q) ||
      (s.city ?? '').toLowerCase().includes(q)
    );
  }, [stores, search]);

  const mapStores = useMemo(
    () => filtered.filter(s => s.latitude != null && s.longitude != null),
    [filtered]
  );

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', mt: 8 }}>
        <CircularProgress />
      </Box>
    );
  }

  return (
    <Container maxWidth="lg" sx={{ mt: 3, mb: 8 }}>
      <Typography variant="h5" fontWeight={700} sx={{ mb: 2 }}>חנויות ורשתות</Typography>

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      {/* Map */}
      {mapStores.length > 0 && (
        <Box sx={{ height: 400, borderRadius: 2, overflow: 'hidden', mb: 3, border: '1px solid', borderColor: 'divider' }}>
          <MapContainer
            center={[mapStores[0].latitude!, mapStores[0].longitude!]}
            zoom={8}
            style={{ height: '100%', width: '100%' }}
          >
            <TileLayer
              attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
              url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
            />
            {mapStores.map(s => (
              <Marker key={s.id} position={[s.latitude!, s.longitude!]}>
                <Popup>
                  <strong>{s.store_name || s.chain_id}</strong>
                  <br />
                  {[s.address, s.city].filter(Boolean).join(', ')}
                  <br />
                  <a href={`/stores/${s.id}`} onClick={e => { e.preventDefault(); navigate(`/stores/${s.id}`); }}>
                    פרטים ומחירים
                  </a>
                </Popup>
              </Marker>
            ))}
          </MapContainer>
        </Box>
      )}

      {/* Search */}
      <TextField
        fullWidth
        size="small"
        label="חפש לפי שם חנות או עיר"
        value={search}
        onChange={e => setSearch(e.target.value)}
        sx={{ mb: 2 }}
        InputProps={{ startAdornment: <LocationOnIcon sx={{ color: 'action.active', mr: 0.5 }} fontSize="small" /> }}
      />

      <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
        {filtered.length} חנויות
      </Typography>

      {/* List */}
      <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', sm: '1fr 1fr', md: '1fr 1fr 1fr' }, gap: 2 }}>
        {filtered.map(store => (
          <Card key={store.id} variant="outlined">
            <CardActionArea onClick={() => navigate(`/stores/${store.id}`)}>
              <CardContent>
                <Typography variant="subtitle1" fontWeight={600}>
                  {store.store_name || store.chain_id}
                </Typography>
                {(store.city || store.address) && (
                  <Typography variant="body2" color="text.secondary">
                    {[store.city, store.address].filter(Boolean).join(' · ')}
                  </Typography>
                )}
              </CardContent>
            </CardActionArea>
          </Card>
        ))}
      </Box>
    </Container>
  );
};

export default StoresPage;

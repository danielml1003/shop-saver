import React, { useState, useEffect } from 'react';
import {
  Container,
  GridLegacy as Grid,
  Typography,
  Box,
  CircularProgress,
  Alert,
  Card,
  CardContent,
  Chip,
} from '@mui/material';
import {
  Store as StoreIcon,
  LocationOn as LocationIcon,
  AccessTime as TimeIcon,
  Business as ChainIcon,
} from '@mui/icons-material';
import { Store } from '../types';
import { apiService } from '../services/api';

const StoresPage: React.FC = () => {
  const [stores, setStores] = useState<Store[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadStores();
  }, []);

  const loadStores = async () => {
    try {
      setLoading(true);
      setError(null);
      const storesData = await apiService.getStores();
      setStores(storesData);
    } catch (err) {
      setError('שגיאה בטעינת החנויות. אנא נסה שוב.');
      console.error('Error loading stores:', err);
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('he-IL');
  };

  const formatTime = (timeString: string) => {
    return timeString.slice(0, 5); // HH:MM format
  };

  if (loading) {
    return (
      <Container maxWidth="lg" sx={{ mt: 4, mb: 4 }}>
        <Box sx={{ display: 'flex', justifyContent: 'center', my: 4 }}>
          <CircularProgress />
        </Box>
      </Container>
    );
  }

  if (error) {
    return (
      <Container maxWidth="lg" sx={{ mt: 4, mb: 4 }}>
        <Alert severity="error">{error}</Alert>
      </Container>
    );
  }

  return (
    <Container maxWidth="lg" sx={{ mt: 4, mb: 4 }}>
      <Typography variant="h4" component="h1" gutterBottom>
        חנויות ורשתות
      </Typography>

      <Typography variant="body1" color="text.secondary" sx={{ mb: 4 }}>
        רשימת החנויות הזמינות במערכת עם פרטי העדכון האחרון
      </Typography>

      {stores.length === 0 ? (
        <Box sx={{ textAlign: 'center', my: 4 }}>
          <Typography variant="h6" color="text.secondary">
            לא נמצאו חנויות
          </Typography>
        </Box>
      ) : (
        <Grid container spacing={3}>
          {stores.map((store) => (
            <Grid item xs={12} md={6} lg={4} key={store.id}>
              <Card sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
                <CardContent sx={{ flexGrow: 1 }}>
                  {/* Store Name */}
                  <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                    <StoreIcon sx={{ mr: 1, color: 'primary.main' }} />
                    <Typography variant="h6" component="h3">
                      {store.store_name}
                    </Typography>
                  </Box>

                  {/* Chain Information */}
                  <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                    <ChainIcon sx={{ mr: 1, fontSize: 16, color: 'text.secondary' }} />
                    <Typography variant="body2" color="text.secondary">
                      קוד רשת: {store.chain_id}
                    </Typography>
                  </Box>

                  {/* Address */}
                  <Box sx={{ display: 'flex', alignItems: 'flex-start', mb: 2 }}>
                    <LocationIcon sx={{ mr: 1, fontSize: 16, color: 'text.secondary', mt: 0.2 }} />
                    <Box>
                      <Typography variant="body2" color="text.secondary">
                        {store.address}
                      </Typography>
                      <Typography variant="body2" color="text.secondary">
                        {store.city}
                        {store.zip_code && `, ${store.zip_code}`}
                      </Typography>
                    </Box>
                  </Box>

                  {/* Last Update */}
                  <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                    <TimeIcon sx={{ mr: 1, fontSize: 16, color: 'text.secondary' }} />
                    <Typography variant="body2" color="text.secondary">
                      עדכון אחרון: {formatDate(store.last_update_date)} בשעה {formatTime(store.last_update_time)}
                    </Typography>
                  </Box>

                  {/* Store Details Chips */}
                  <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5, mt: 'auto' }}>
                    <Chip 
                      label={`סניף ${store.store_id}`} 
                      size="small" 
                      variant="outlined" 
                    />
                    <Chip 
                      label={`תת-רשת ${store.subchain_id}`} 
                      size="small" 
                      variant="outlined" 
                    />
                  </Box>

                  {/* Processed timestamp */}
                  <Typography 
                    variant="caption" 
                    color="text.secondary" 
                    sx={{ display: 'block', mt: 2 }}
                  >
                    נוסף למערכת: {formatDate(store.processed_at)}
                  </Typography>
                </CardContent>
              </Card>
            </Grid>
          ))}
        </Grid>
      )}
    </Container>
  );
};

export default StoresPage;

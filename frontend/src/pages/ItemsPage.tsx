import React, { useState, useEffect } from 'react';
import {
  Container,
  GridLegacy as Grid,
  Typography,
  Box,
  CircularProgress,
  Alert,
  Pagination,
  Paper,
} from '@mui/material';
import SearchFilters from '../components/SearchFilters';
import ItemCard from '../components/ItemCard';
import { Item, Store, SearchFilters as SearchFiltersType, ApiResponse } from '../types';
import { apiService } from '../services/api';

const ItemsPage: React.FC = () => {
  const [items, setItems] = useState<Item[]>([]);
  const [stores, setStores] = useState<Store[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filters, setFilters] = useState<SearchFiltersType>({});
  const [pagination, setPagination] = useState({
    page: 1,
    limit: 12,
    total: 0,
  });

  const loadStores = React.useCallback(async () => {
    try {
      const storesData = await apiService.getStores();
      setStores(storesData);
    } catch (err) {
      console.error('Error loading stores:', err);
    }
  }, []);

  const loadItems = React.useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      
      const response: ApiResponse<Item> = await apiService.getItems(
        filters,
        pagination.page,
        pagination.limit
      );
      
      setItems(response.data);
      setPagination(prev => ({
        ...prev,
        total: response.total,
      }));
    } catch (err) {
      setError('שגיאה בטעינת המוצרים. אנא נסה שוב.');
      console.error('Error loading items:', err);
    } finally {
      setLoading(false);
    }
  }, [filters, pagination.page, pagination.limit]);

  useEffect(() => {
    loadStores();
  }, [loadStores]);

  useEffect(() => {
    loadItems();
  }, [loadItems]);


  const handleFiltersChange = (newFilters: SearchFiltersType) => {
    setFilters(newFilters);
    setPagination(prev => ({ ...prev, page: 1 })); // Reset to first page
  };

  const handlePageChange = (event: React.ChangeEvent<unknown>, value: number) => {
    setPagination(prev => ({ ...prev, page: value }));
  };

  const getStoreById = (storeId: number): Store | undefined => {
    return stores.find(store => store.id === storeId);
  };

  const totalPages = Math.ceil(pagination.total / pagination.limit);

  return (
    <Container maxWidth="xl" sx={{ mt: 4, mb: 4 }}>
      <Typography variant="h4" component="h1" gutterBottom>
        חיפוש מוצרים ומחירים
      </Typography>

      <SearchFilters onFiltersChange={handleFiltersChange} loading={loading} />

      {error && (
        <Alert severity="error" sx={{ mb: 2 }}>
          {error}
        </Alert>
      )}

      {/* Results Summary */}
      <Paper sx={{ p: 2, mb: 2, backgroundColor: 'grey.50' }}>
        <Typography variant="body1">
          {loading ? (
            'טוען מוצרים...'
          ) : (
            `נמצאו ${pagination.total} מוצרים${Object.keys(filters).length > 0 ? ' עבור החיפוש שלך' : ''}`
          )}
        </Typography>
      </Paper>

      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', my: 4 }}>
          <CircularProgress />
        </Box>
      ) : (
        <>
          {items.length === 0 ? (
            <Box sx={{ textAlign: 'center', my: 4 }}>
              <Typography variant="h6" color="text.secondary">
                לא נמצאו מוצרים
              </Typography>
              <Typography variant="body2" color="text.secondary">
                נסה לשנות את הפילטרים או לחפש משהו אחר
              </Typography>
            </Box>
          ) : (
            <>
              {/* Items Grid */}
              <Grid container spacing={3}>
                {items.map((item) => (
                  <Grid item xs={12} sm={6} md={4} lg={3} key={item.id}>
                    <ItemCard 
                      item={item} 
                      store={getStoreById(item.store_pk)} 
                    />
                  </Grid>
                ))}
              </Grid>

              {/* Pagination */}
              {totalPages > 1 && (
                <Box sx={{ display: 'flex', justifyContent: 'center', mt: 4 }}>
                  <Pagination
                    count={totalPages}
                    page={pagination.page}
                    onChange={handlePageChange}
                    color="primary"
                    size="large"
                    showFirstButton
                    showLastButton
                  />
                </Box>
              )}
            </>
          )}
        </>
      )}
    </Container>
  );
};

export default ItemsPage;

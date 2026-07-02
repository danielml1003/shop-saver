import axios from 'axios';
import {
  UserLocation,
  PriceComparisonRequest,
  PriceComparisonResponse,
  BackendStoreInfo,
  ProductSearchResult,
  PaginatedItemsResponse,
} from '../types';

// Production build: same-origin relative URLs — nginx proxies /api/ to the backend.
// Dev (npm start): talk to the locally running API directly; override with REACT_APP_API_URL.
const API_BASE_URL =
  process.env.REACT_APP_API_URL ??
  (process.env.NODE_ENV === 'production' ? '' : 'http://127.0.0.1:3000');

const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

export const apiService = {
  // GET /api/stores/nearby — stores within a radius of the user's location
  getNearbyStores: async (location: UserLocation): Promise<BackendStoreInfo[]> => {
    const params = new URLSearchParams({
      latitude: String(location.latitude),
      longitude: String(location.longitude),
    });
    if (location.radius_km != null) {
      params.set('radius_km', String(location.radius_km));
    }
    const res = await api.get(`/api/stores/nearby?${params.toString()}`);
    return res.data as BackendStoreInfo[];
  },

  // GET /api/stores — all stores with coordinates (for the map page)
  getAllStores: async (): Promise<BackendStoreInfo[]> => {
    const res = await api.get('/api/stores');
    return res.data as BackendStoreInfo[];
  },

  // GET /api/stores/:id/items — items for one store, paginated
  getStoreItems: async (
    storeId: number,
    query?: string,
    page = 1,
    limit = 20
  ): Promise<PaginatedItemsResponse> => {
    const params: Record<string, string | number> = { page, limit };
    if (query) params.q = query;
    const res = await api.get(`/api/stores/${storeId}/items`, { params });
    return res.data as PaginatedItemsResponse;
  },

  // GET /api/items — paginated item search across all stores
  searchItemsPaginated: async (
    query?: string,
    minPrice?: number,
    maxPrice?: number,
    page = 1,
    limit = 20
  ): Promise<PaginatedItemsResponse> => {
    const params: Record<string, string | number> = { page, limit };
    if (query) params.q = query;
    if (minPrice != null) params.min_price = minPrice;
    if (maxPrice != null) params.max_price = maxPrice;
    const res = await api.get('/api/items', { params });
    return res.data as PaginatedItemsResponse;
  },

  // POST /api/compare-prices — compare basket across stores (location/city optional, paginated)
  comparePrices: async (payload: PriceComparisonRequest): Promise<PriceComparisonResponse> => {
    const res = await api.post('/api/compare-prices', payload);
    return res.data as PriceComparisonResponse;
  },

  // GET /api/items/search — autocomplete: {barcode, name} pairs
  searchItemNames: async (q: string): Promise<ProductSearchResult[]> => {
    const res = await api.get('/api/items/search', { params: { q } });
    return res.data as ProductSearchResult[];
  },
};

export default apiService;

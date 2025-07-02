import axios from 'axios';
import { Store, Item, SearchFilters, ApiResponse } from '../types';

// For now, we'll create a mock API service since we need to create a backend API endpoint
// In a real implementation, you'd connect directly to PostgreSQL or create an Express/FastAPI backend

const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:3001/api';

// Axios instance for future API calls
// const api = axios.create({
//   baseURL: API_BASE_URL,
//   headers: {
//     'Content-Type': 'application/json',
//   },
// });

// Mock data for development - replace with real API calls
const mockStores: Store[] = [
  {
    id: 1,
    chain_id: '7290058108879',
    subchain_id: '001',
    store_id: '001',
    store_name: 'רמי לוי השיקמא',
    address: 'רחוב הרצל 123',
    city: 'תל אביב',
    zip_code: '12345',
    last_update_date: '2024-07-01',
    last_update_time: '10:00:00',
    processed_at: '2024-07-01T10:00:00Z',
  },
  {
    id: 2,
    chain_id: '7290058108880',
    subchain_id: '002',
    store_id: '002',
    store_name: 'שופרסל דיל',
    address: 'רחוב בן יהודה 456',
    city: 'ירושלים',
    zip_code: '54321',
    last_update_date: '2024-07-01',
    last_update_time: '09:30:00',
    processed_at: '2024-07-01T09:30:00Z',
  },
];

const mockItems: Item[] = [
  {
    id: 1,
    store_pk: 1,
    item_code: '1234567890',
    item_type: 1,
    item_name: 'חלב 3% 1 ליטר',
    item_name_he: 'חלב 3% 1 ליטר',
    item_name_en: 'Milk 3% 1L',
    manufacturer_name: 'תנובה',
    manufacturer_country: 'ישראל',
    unit_qty: '1',
    quantity: '1',
    unit_of_measure: 'ליטר',
    is_weighted: false,
    qty_in_package: 1,
    item_price: 5.90,
    unit_of_measure_price: 5.90,
    allow_discount: true,
    item_status: 1,
    price_update_date: '2024-07-01',
    processed_at: '2024-07-01T10:00:00Z',
  },
  {
    id: 2,
    store_pk: 1,
    item_code: '1234567891',
    item_type: 1,
    item_name: 'לחם פרוס שלם',
    item_name_he: 'לחם פרוס שלם',
    item_name_en: 'Whole Wheat Bread',
    manufacturer_name: 'אנג\'ל',
    manufacturer_country: 'ישראל',
    unit_qty: '500',
    quantity: '500',
    unit_of_measure: 'גרם',
    is_weighted: false,
    qty_in_package: 1,
    item_price: 8.50,
    unit_of_measure_price: 17.00,
    allow_discount: true,
    item_status: 1,
    price_update_date: '2024-07-01',
    processed_at: '2024-07-01T10:00:00Z',
  },
  {
    id: 3,
    store_pk: 2,
    item_code: '1234567892',
    item_type: 1,
    item_name: 'בלוק צ\'דר אדום אירי',
    item_name_he: 'בלוק צ\'דר אדום אירי',
    item_name_en: 'Irish Red Cheddar Block',
    manufacturer_name: 'גד',
    manufacturer_country: 'ישראל',
    unit_qty: '200',
    quantity: '200',
    unit_of_measure: 'גרם',
    is_weighted: false,
    qty_in_package: 1,
    item_price: 12.90,
    unit_of_measure_price: 64.50,
    allow_discount: false,
    item_status: 1,
    price_update_date: '2024-07-01',
    processed_at: '2024-07-01T09:30:00Z',
  },
];

export const apiService = {
  // Get all stores
  getStores: async (): Promise<Store[]> => {
    try {
      // In production, replace with: const response = await api.get('/stores');
      // return response.data;
      return mockStores;
    } catch (error) {
      console.error('Error fetching stores:', error);
      return mockStores; // Fallback to mock data
    }
  },

  // Get items with optional filters and pagination
  getItems: async (
    filters?: SearchFilters,
    page = 1,
    limit = 20
  ): Promise<ApiResponse<Item>> => {
    try {
      // In production, replace with real API call
      // const response = await api.get('/items', { params: { ...filters, page, limit } });
      // return response.data;
      
      let filteredItems = [...mockItems];
      
      if (filters) {
        if (filters.itemName) {
          filteredItems = filteredItems.filter(item =>
            item.item_name.toLowerCase().includes(filters.itemName!.toLowerCase()) ||
            (item.item_name_en && item.item_name_en.toLowerCase().includes(filters.itemName!.toLowerCase()))
          );
        }
        if (filters.minPrice !== undefined) {
          filteredItems = filteredItems.filter(item => item.item_price >= filters.minPrice!);
        }
        if (filters.maxPrice !== undefined) {
          filteredItems = filteredItems.filter(item => item.item_price <= filters.maxPrice!);
        }
        if (filters.manufacturer) {
          filteredItems = filteredItems.filter(item =>
            item.manufacturer_name?.toLowerCase().includes(filters.manufacturer!.toLowerCase())
          );
        }
      }

      const startIndex = (page - 1) * limit;
      const endIndex = startIndex + limit;
      const paginatedItems = filteredItems.slice(startIndex, endIndex);

      return {
        data: paginatedItems,
        total: filteredItems.length,
        page,
        limit,
      };
    } catch (error) {
      console.error('Error fetching items:', error);
      return {
        data: mockItems,
        total: mockItems.length,
        page: 1,
        limit: 20,
      };
    }
  },

  // Get items by store ID
  getItemsByStore: async (storeId: number): Promise<Item[]> => {
    try {
      // In production: const response = await api.get(`/stores/${storeId}/items`);
      return mockItems.filter(item => item.store_pk === storeId);
    } catch (error) {
      console.error('Error fetching items by store:', error);
      return mockItems.filter(item => item.store_pk === storeId);
    }
  },

  // Search items by name
  searchItems: async (query: string): Promise<Item[]> => {
    try {
      // In production: const response = await api.get(`/items/search?q=${query}`);
      return mockItems.filter(item =>
        item.item_name.toLowerCase().includes(query.toLowerCase()) ||
        (item.item_name_en && item.item_name_en.toLowerCase().includes(query.toLowerCase()))
      );
    } catch (error) {
      console.error('Error searching items:', error);
      return [];
    }
  },

  // Get latest prices for items
  getLatestPrices: async (): Promise<Item[]> => {
    try {
      // In production: const response = await api.get('/items/latest-prices');
      return mockItems;
    } catch (error) {
      console.error('Error fetching latest prices:', error);
      return mockItems;
    }
  },
};

export default apiService;

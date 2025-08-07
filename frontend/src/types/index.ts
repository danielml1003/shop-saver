export interface Store {
  id: number;
  chain_id: string;
  subchain_id: string;
  store_id: string;
  store_name: string;
  address: string;
  city: string;
  zip_code?: string;
  last_update_date: string;
  last_update_time: string;
  processed_at: string;
}

export interface Item {
  id: number;
  store_pk: number;
  item_code: string;
  item_type: number;
  item_name: string;
  manufacturer_item_description?: string;
  unit_qty?: string;
  quantity?: string;
  unit_of_measure?: string;
  is_weighted?: boolean;
  qty_in_package?: number;
  item_price: number;
  unit_of_measure_price?: number;
  allow_discount?: boolean;
  item_status?: number;
  item_id?: number;
  price_update_date: string;
  manufacturer_name?: string;
  manufacturer_country?: string;
  manufacturer_item_description_en?: string;
  manufacturer_item_description_he?: string;
  item_name_en?: string;
  item_name_he?: string;
  processed_at: string;
}

export interface SearchFilters {
  storeName?: string;
  itemName?: string;
  minPrice?: number;
  maxPrice?: number;
  manufacturer?: string;
  city?: string;
}

export interface ApiResponse<T> {
  data: T[];
  total: number;
  page: number;
  limit: number;
}

// Backend API types for price comparison flow
export interface UserLocation {
  latitude: number;
  longitude: number;
  radius_km?: number;
}

export interface PriceComparisonRequest {
  user_location: UserLocation;
  grocery_list: string[];
}

export interface BackendStoreInfo {
  id: number;
  chain_id: string;
  sub_chain_id: number;
  store_id: number;
  address?: string;
  city?: string;
  latitude?: number;
  longitude?: number;
  distance_km?: number;
}

export interface ComparisonItemPrice {
  item_code: string;
  item_name: string;
  price: number;
  unit_of_measure?: string;
  manufacturer_name?: string;
}

export interface StoreComparison {
  store: BackendStoreInfo;
  items: ComparisonItemPrice[];
  total_price: number;
  items_found: number;
  items_missing: string[];
}

export interface PriceComparisonResponse {
  stores: StoreComparison[];
  best_store?: StoreComparison;
  requested_items: string[];
}
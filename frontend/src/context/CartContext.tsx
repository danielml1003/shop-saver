import React, { createContext, useContext, useMemo, useState } from 'react';
import { GroceryItem } from '../types';

export interface CartContextValue {
  items: GroceryItem[];
  addItem: (item: GroceryItem) => void;
  removeItem: (name: string) => void;
  clearCart: () => void;
  contains: (name: string) => boolean;
  setItems: (items: GroceryItem[]) => void;
}

const CartContext = createContext<CartContextValue | undefined>(undefined);

export const CartProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [items, setItems] = useState<GroceryItem[]>([]);

  const addItem = (item: GroceryItem) => {
    setItems(prev => {
      if (prev.some(i => i.name === item.name || (i.barcode && i.barcode === item.barcode))) {
        return prev;
      }
      return [...prev, item];
    });
  };

  const removeItem = (name: string) => {
    setItems(prev => prev.filter(i => i.name !== name));
  };

  const clearCart = () => setItems([]);

  const contains = (name: string) => items.some(i => i.name === name);

  const value = useMemo<CartContextValue>(
    () => ({ items, addItem, removeItem, clearCart, contains, setItems }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [items]
  );

  return <CartContext.Provider value={value}>{children}</CartContext.Provider>;
};

export const useCart = (): CartContextValue => {
  const ctx = useContext(CartContext);
  if (!ctx) throw new Error('useCart must be used within a CartProvider');
  return ctx;
};

import React, { createContext, useContext, useMemo, useState } from 'react';
import { Item } from '../types';

export interface CartContextValue {
  items: Item[];
  addItem: (item: Item) => void;
  removeItem: (itemCode: string) => void;
  clearCart: () => void;
  contains: (itemCode: string) => boolean;
}

const CartContext = createContext<CartContextValue | undefined>(undefined);

export const CartProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [items, setItems] = useState<Item[]>([]);

  const addItem = (item: Item) => {
    setItems(prev => {
      if (prev.some(i => i.item_code === item.item_code)) return prev;
      return [item, ...prev];
    });
  };

  const removeItem = (itemCode: string) => {
    setItems(prev => prev.filter(i => i.item_code !== itemCode));
  };

  const clearCart = () => setItems([]);

  const contains = (itemCode: string) => items.some(i => i.item_code === itemCode);

  const value = useMemo<CartContextValue>(() => ({ items, addItem, removeItem, clearCart, contains }), [items]);

  return <CartContext.Provider value={value}>{children}</CartContext.Provider>;
};

export const useCart = (): CartContextValue => {
  const ctx = useContext(CartContext);
  if (!ctx) throw new Error('useCart must be used within a CartProvider');
  return ctx;
};



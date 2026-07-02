import React from 'react';
import { renderHook, act } from '@testing-library/react';
import { CartProvider, useCart } from './CartContext';

const wrapper = ({ children }: { children: React.ReactNode }) => (
  <CartProvider>{children}</CartProvider>
);

test('adds and removes grocery items', () => {
  const { result } = renderHook(() => useCart(), { wrapper });

  act(() => result.current.addItem({ barcode: '7290000000001', name: 'חלב 3%' }));
  act(() => result.current.addItem({ barcode: null, name: 'לחם' }));

  expect(result.current.items).toHaveLength(2);
  expect(result.current.contains('חלב 3%')).toBe(true);

  act(() => result.current.removeItem('חלב 3%'));
  expect(result.current.items).toHaveLength(1);
  expect(result.current.contains('חלב 3%')).toBe(false);
});

test('deduplicates by name and by barcode', () => {
  const { result } = renderHook(() => useCart(), { wrapper });

  act(() => result.current.addItem({ barcode: '7290000000001', name: 'חלב 3%' }));
  act(() => result.current.addItem({ barcode: '7290000000001', name: 'חלב תנובה' })); // same barcode
  act(() => result.current.addItem({ barcode: null, name: 'חלב 3%' })); // same name

  expect(result.current.items).toHaveLength(1);
});

test('clearCart empties the list', () => {
  const { result } = renderHook(() => useCart(), { wrapper });

  act(() => result.current.addItem({ barcode: null, name: 'לחם' }));
  act(() => result.current.clearCart());

  expect(result.current.items).toHaveLength(0);
});

test('useCart outside a provider throws', () => {
  // Silence the expected React error log
  const spy = jest.spyOn(console, 'error').mockImplementation(() => {});
  expect(() => renderHook(() => useCart())).toThrow('useCart must be used within a CartProvider');
  spy.mockRestore();
});

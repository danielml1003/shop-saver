import React from 'react';
import { render, screen } from '@testing-library/react';
import App from './App';

// jsdom doesn't implement matchMedia (used by MUI useMediaQuery) or IntersectionObserver
beforeAll(() => {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
  (window as any).IntersectionObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
});

test('renders the app shell with navigation', () => {
  render(<App />);
  expect(screen.getByText('ShopSaver')).toBeInTheDocument();
  // Compare page is the home route
  expect(screen.getByText('סל הקניות שלי')).toBeInTheDocument();
});

// Jest stand-in for react-leaflet (ESM-only package that CRA's Jest can't load).
// Renders plain containers so pages using the map can be render-tested.
import React from 'react';

export const MapContainer: React.FC<any> = ({ children }) => (
  <div data-testid="map-container">{children}</div>
);
export const TileLayer: React.FC<any> = () => null;
export const Marker: React.FC<any> = ({ children }) => <div>{children}</div>;
export const Popup: React.FC<any> = ({ children }) => <div>{children}</div>;

# Shop Saver Frontend

A modern React TypeScript frontend for the Shop Saver application that displays Israeli shop data with Hebrew support.

## Features

- 🔍 **Advanced Search & Filtering** - Search products by name, price range, manufacturer, store, and city
- 🏪 **Store Management** - View all stores with detailed information
- 📱 **Responsive Design** - Works perfectly on desktop, tablet, and mobile
- 🇮🇱 **Hebrew Support** - Full RTL (Right-to-Left) support for Hebrew text
- 🎨 **Material-UI Design** - Clean, modern interface using Material-UI components
- ⚡ **Real-time Updates** - Displays the latest data from the PostgreSQL database

## Tech Stack

- **React 18** with TypeScript
- **Material-UI (MUI)** for UI components
- **React Router** for navigation
- **Axios** for API calls
- **PostgreSQL** integration (via API)

## Quick Start

### Prerequisites

- Node.js 16+ and npm
- The Shop Saver backend must be running (Rust + PostgreSQL)

### Installation

```bash
# Navigate to frontend directory
cd frontend

# Install dependencies
npm install

# Start development server
npm start
```

The frontend will be available at `http://localhost:3000`

## Project Structure

```
src/
├── components/           # Reusable UI components
│   ├── Header.tsx       # Navigation header
│   ├── SearchFilters.tsx # Product search and filtering
│   └── ItemCard.tsx     # Product display card
├── pages/               # Main application pages
│   ├── ItemsPage.tsx    # Product search and listing
│   └── StoresPage.tsx   # Store information display
├── services/            # API and data services
│   └── api.ts          # API service with mock data
├── types/              # TypeScript type definitions
│   └── index.ts        # Data interfaces and types
└── App.tsx             # Main application component
```

## Available Pages

### 1. Product Search (`/`)
- Search products by name (Hebrew and English)
- Filter by price range, manufacturer, store, and city
- Paginated results with detailed product cards
- Shows price, manufacturer, store information, and more

### 2. Stores (`/stores`)
- List all available stores
- Display store details including address, city, chain information
- Show last update dates and processing timestamps

## API Integration

Currently using mock data for development. To connect to real data:

1. **Create a Backend API**: You'll need to create REST API endpoints that connect to your PostgreSQL database
2. **Update API Service**: Modify `src/services/api.ts` to use real endpoints
3. **Environment Variables**: Set `REACT_APP_API_URL` to your backend URL

### Required API Endpoints

```
GET /api/stores                     # Get all stores
GET /api/items                      # Get items with filtering/pagination
GET /api/items/search?q={query}     # Search items
GET /api/stores/{id}/items          # Get items by store
GET /api/items/latest-prices        # Get latest prices
```

## Development Commands

```bash
npm start          # Start development server
npm test           # Run tests
npm run build      # Build for production
npm run eject      # Eject from Create React App (not recommended)
```

## Hebrew/RTL Support

The app includes full Hebrew support:

- RTL (Right-to-Left) layout
- Hebrew text rendering
- Proper date/time formatting
- Currency formatting (ILS)

## Next Steps

1. **Connect to Real API**: Replace mock data with actual backend integration
2. **Add More Features**: 
   - Price comparison charts
   - Favorite products
   - Price alerts
   - Store location maps
3. **Enhanced Filtering**: Category-based filtering, advanced search
4. **Performance**: Add caching, virtual scrolling for large datasets

This frontend provides a solid foundation for your Shop Saver application with room for expansion and customization!

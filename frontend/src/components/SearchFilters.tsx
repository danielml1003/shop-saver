import React, { useState } from 'react';
import {
  Box,
  Card,
  CardContent,
  TextField,
  Button,
  GridLegacy as Grid,
  Typography,
  Collapse,
  IconButton,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  ExpandLess as ExpandLessIcon,
  Search as SearchIcon,
  Clear as ClearIcon,
} from '@mui/icons-material';
import { SearchFilters as SearchFiltersType } from '../types';

interface SearchFiltersProps {
  onFiltersChange: (filters: SearchFiltersType) => void;
  loading?: boolean;
}

const SearchFilters: React.FC<SearchFiltersProps> = ({
  onFiltersChange,
  loading = false,
}) => {
  const [expanded, setExpanded] = useState(false);
  const [filters, setFilters] = useState<SearchFiltersType>({
    itemName: '',
    minPrice: undefined,
    maxPrice: undefined,
    manufacturer: '',
    storeName: '',
    city: '',
  });

  const handleFilterChange = (field: keyof SearchFiltersType, value: any) => {
    const newFilters = { ...filters, [field]: value };
    setFilters(newFilters);
  };

  const handleSearch = () => {
    // Remove empty string values
    const cleanFilters: SearchFiltersType = {};
    Object.entries(filters).forEach(([key, value]) => {
      if (value !== '' && value !== undefined && value !== null) {
        cleanFilters[key as keyof SearchFiltersType] = value;
      }
    });
    onFiltersChange(cleanFilters);
  };

  const handleClear = () => {
    const emptyFilters: SearchFiltersType = {
      itemName: '',
      minPrice: undefined,
      maxPrice: undefined,
      manufacturer: '',
      storeName: '',
      city: '',
    };
    setFilters(emptyFilters);
    onFiltersChange({});
  };

  const handleKeyPress = (event: React.KeyboardEvent) => {
    if (event.key === 'Enter') {
      handleSearch();
    }
  };

  return (
    <Card sx={{ mb: 2 }}>
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
          <Typography variant="h6" sx={{ flexGrow: 1 }}>
            חיפוש וסינון מוצרים
          </Typography>
          <IconButton
            onClick={() => setExpanded(!expanded)}
            size="small"
          >
            {expanded ? <ExpandLessIcon /> : <ExpandMoreIcon />}
          </IconButton>
        </Box>

        <Grid container spacing={2} sx={{ mb: 2 }}>
          <Grid item xs={12} md={6}>
            <TextField
              fullWidth
              label="שם מוצר"
              placeholder="חפש לפי שם מוצר..."
              value={filters.itemName}
              onChange={(e) => handleFilterChange('itemName', e.target.value)}
              onKeyPress={handleKeyPress}
              variant="outlined"
              size="small"
            />
          </Grid>
          <Grid item xs={12} md={6}>
            <TextField
              fullWidth
              label="יצרן"
              placeholder="שם יצרן..."
              value={filters.manufacturer}
              onChange={(e) => handleFilterChange('manufacturer', e.target.value)}
              onKeyPress={handleKeyPress}
              variant="outlined"
              size="small"
            />
          </Grid>
        </Grid>

        <Collapse in={expanded}>
          <Grid container spacing={2} sx={{ mb: 2 }}>
            <Grid item xs={12} md={3}>
              <TextField
                fullWidth
                label="מחיר מינימלי"
                type="number"
                placeholder="0.00"
                value={filters.minPrice || ''}
                onChange={(e) => 
                  handleFilterChange('minPrice', e.target.value ? parseFloat(e.target.value) : undefined)
                }
                onKeyPress={handleKeyPress}
                variant="outlined"
                size="small"
                inputProps={{ step: 0.01, min: 0 }}
              />
            </Grid>
            <Grid item xs={12} md={3}>
              <TextField
                fullWidth
                label="מחיר מקסימלי"
                type="number"
                placeholder="999.99"
                value={filters.maxPrice || ''}
                onChange={(e) => 
                  handleFilterChange('maxPrice', e.target.value ? parseFloat(e.target.value) : undefined)
                }
                onKeyPress={handleKeyPress}
                variant="outlined"
                size="small"
                inputProps={{ step: 0.01, min: 0 }}
              />
            </Grid>
            <Grid item xs={12} md={3}>
              <TextField
                fullWidth
                label="שם חנות"
                placeholder="שם רשת או חנות..."
                value={filters.storeName}
                onChange={(e) => handleFilterChange('storeName', e.target.value)}
                onKeyPress={handleKeyPress}
                variant="outlined"
                size="small"
              />
            </Grid>
            <Grid item xs={12} md={3}>
              <TextField
                fullWidth
                label="עיר"
                placeholder="עיר..."
                value={filters.city}
                onChange={(e) => handleFilterChange('city', e.target.value)}
                onKeyPress={handleKeyPress}
                variant="outlined"
                size="small"
              />
            </Grid>
          </Grid>
        </Collapse>

        <Box sx={{ display: 'flex', gap: 1, justifyContent: 'flex-end' }}>
          <Button
            variant="outlined"
            onClick={handleClear}
            startIcon={<ClearIcon />}
            disabled={loading}
          >
            נקה
          </Button>
          <Button
            variant="contained"
            onClick={handleSearch}
            startIcon={<SearchIcon />}
            disabled={loading}
          >
            חפש
          </Button>
        </Box>
      </CardContent>
    </Card>
  );
};

export default SearchFilters;

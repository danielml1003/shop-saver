import React from 'react';
import {
  Card,
  CardContent,
  Typography,
  Box,
  Chip,
  Divider,
} from '@mui/material';
import {
  LocalOffer as PriceIcon,
  Store as StoreIcon,
  Business as ManufacturerIcon,
  Scale as WeightIcon,
} from '@mui/icons-material';
import { Item, Store } from '../types';

interface ItemCardProps {
  item: Item;
  store?: Store;
}

const ItemCard: React.FC<ItemCardProps> = ({ item, store }) => {
  const formatPrice = (price: number) => {
    return new Intl.NumberFormat('he-IL', {
      style: 'currency',
      currency: 'ILS',
    }).format(price);
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('he-IL');
  };

  const getDisplayName = () => {
    return item.item_name_he || item.item_name || 'שם לא זמין';
  };

  const getEnglishName = () => {
    return item.item_name_en && item.item_name_en !== item.item_name_he 
      ? item.item_name_en 
      : null;
  };

  return (
    <Card sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <CardContent sx={{ flexGrow: 1 }}>
        {/* Product Name */}
        <Typography variant="h6" component="h3" gutterBottom>
          {getDisplayName()}
        </Typography>
        
        {/* English Name if different */}
        {getEnglishName() && (
          <Typography variant="body2" color="text.secondary" gutterBottom>
            {getEnglishName()}
          </Typography>
        )}

        {/* Price */}
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
          <PriceIcon sx={{ mr: 1, color: 'primary.main' }} />
          <Typography variant="h5" color="primary.main" fontWeight="bold">
            {formatPrice(item.item_price)}
          </Typography>
        </Box>

        {/* Unit Price if different */}
        {item.unit_of_measure_price && 
         item.unit_of_measure_price !== item.item_price && (
          <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
            מחיר ליחידה: {formatPrice(item.unit_of_measure_price)} / {item.unit_of_measure}
          </Typography>
        )}

        <Divider sx={{ my: 2 }} />

        {/* Product Details */}
        <Box sx={{ mb: 2 }}>
          {item.manufacturer_name && (
            <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
              <ManufacturerIcon sx={{ mr: 1, fontSize: 16, color: 'text.secondary' }} />
              <Typography variant="body2" color="text.secondary">
                יצרן: {item.manufacturer_name}
              </Typography>
            </Box>
          )}

          {item.quantity && item.unit_of_measure && (
            <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
              <WeightIcon sx={{ mr: 1, fontSize: 16, color: 'text.secondary' }} />
              <Typography variant="body2" color="text.secondary">
                כמות: {item.quantity} {item.unit_of_measure}
              </Typography>
            </Box>
          )}

          {store && (
            <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
              <StoreIcon sx={{ mr: 1, fontSize: 16, color: 'text.secondary' }} />
              <Typography variant="body2" color="text.secondary">
                חנות: {store.store_name}
              </Typography>
            </Box>
          )}
        </Box>

        {/* Tags */}
        <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5, mb: 2 }}>
          {item.is_weighted && (
            <Chip label="נמכר במשקל" size="small" variant="outlined" />
          )}
          {item.allow_discount && (
            <Chip label="זכאי להנחה" size="small" color="success" variant="outlined" />
          )}
          {item.manufacturer_country && (
            <Chip 
              label={`יוצר: ${item.manufacturer_country}`} 
              size="small" 
              variant="outlined" 
            />
          )}
        </Box>

        {/* Footer with dates */}
        <Box sx={{ mt: 'auto', pt: 1 }}>
          <Typography variant="caption" color="text.secondary" display="block">
            עודכן: {formatDate(item.price_update_date)}
          </Typography>
          <Typography variant="caption" color="text.secondary" display="block">
            קוד מוצר: {item.item_code}
          </Typography>
        </Box>
      </CardContent>
    </Card>
  );
};

export default ItemCard;

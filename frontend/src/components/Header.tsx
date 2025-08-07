import React from 'react';
import {
  AppBar,
  Toolbar,
  Typography,
  Button,
  Box,
} from '@mui/material';
import {
  Store as StoreIcon,
  Search as SearchIcon,
  ShoppingCart as ShoppingCartIcon,
    CompareArrows as CompareIcon,
} from '@mui/icons-material';
import { useNavigate, useLocation } from 'react-router-dom';

const Header: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();

  const isActive = (path: string) => location.pathname === path;

  return (
    <AppBar position="static" sx={{ backgroundColor: '#1976d2' }}>
      <Toolbar>
        <ShoppingCartIcon sx={{ mr: 2 }} />
        <Typography
          variant="h6"
          component="div"
          sx={{ flexGrow: 1, cursor: 'pointer' }}
          onClick={() => navigate('/')}
        >
          Shop Saver - מחפש מחירים
        </Typography>
        
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            color={isActive('/') ? 'secondary' : 'inherit'}
            onClick={() => navigate('/')}
            startIcon={<SearchIcon />}
          >
            חיפוש מוצרים
          </Button>
          
          <Button
            color={isActive('/stores') ? 'secondary' : 'inherit'}
            onClick={() => navigate('/stores')}
            startIcon={<StoreIcon />}
          >
            חנויות
          </Button>

          <Button
            color={isActive('/compare') ? 'secondary' : 'inherit'}
            onClick={() => navigate('/compare')}
            startIcon={<CompareIcon />}
          >
            השוואה
          </Button>
        </Box>
      </Toolbar>
    </AppBar>
  );
};

export default Header;

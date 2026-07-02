import React from 'react';
import {
  AppBar, Badge, BottomNavigation, BottomNavigationAction,
  Box, Paper, Toolbar, Typography, useMediaQuery, useTheme,
} from '@mui/material';
import CompareArrowsIcon from '@mui/icons-material/CompareArrows';
import SearchIcon from '@mui/icons-material/Search';
import StoreIcon from '@mui/icons-material/Store';
import ShoppingCartIcon from '@mui/icons-material/ShoppingCart';
import { useNavigate, useLocation } from 'react-router-dom';
import { useCart } from '../context/CartContext';

const NAV_ITEMS = [
  { path: '/', label: 'השוואה', icon: <CompareArrowsIcon /> },
  { path: '/items', label: 'מוצרים', icon: <SearchIcon /> },
  { path: '/stores', label: 'חנויות', icon: <StoreIcon /> },
];

const Header: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const { items } = useCart();
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('sm'));

  const isActive = (path: string) =>
    path === '/' ? location.pathname === '/' || location.pathname === '/compare' : location.pathname.startsWith(path);

  if (isMobile) {
    return (
      <>
        {/* Spacer so content isn't hidden behind bottom nav */}
        <Box sx={{ height: 56 }} />
        <Paper
          elevation={3}
          sx={{ position: 'fixed', bottom: 0, left: 0, right: 0, zIndex: 1200 }}
        >
          <BottomNavigation
            value={location.pathname}
            onChange={(_, val) => navigate(val)}
            showLabels
          >
            {NAV_ITEMS.map(item => (
              <BottomNavigationAction
                key={item.path}
                label={item.label}
                value={item.path}
                icon={item.icon}
              />
            ))}
            <BottomNavigationAction
              label="הסל"
              value="/cart"
              icon={
                <Badge badgeContent={items.length || undefined} color="primary">
                  <ShoppingCartIcon />
                </Badge>
              }
            />
          </BottomNavigation>
        </Paper>
      </>
    );
  }

  return (
    <AppBar
      position="sticky"
      color="default"
      elevation={0}
      sx={{ borderBottom: '1px solid', borderColor: 'divider', bgcolor: 'background.paper' }}
    >
      <Toolbar sx={{ gap: 1 }}>
        {/* Logo */}
        <Box
          sx={{ display: 'flex', alignItems: 'center', gap: 1, cursor: 'pointer', mr: 4 }}
          onClick={() => navigate('/')}
        >
          <ShoppingCartIcon color="primary" />
          <Typography variant="h6" fontWeight={700} color="primary">
            ShopSaver
          </Typography>
        </Box>

        {/* Nav links */}
        <Box sx={{ display: 'flex', flex: 1 }}>
          {NAV_ITEMS.map(item => (
            <Box
              key={item.path}
              onClick={() => navigate(item.path)}
              sx={{
                display: 'flex', alignItems: 'center', gap: 0.5,
                px: 2, py: 1, cursor: 'pointer',
                color: isActive(item.path) ? 'primary.main' : 'text.secondary',
                borderBottom: '2px solid',
                borderColor: isActive(item.path) ? 'primary.main' : 'transparent',
                fontWeight: isActive(item.path) ? 600 : 400,
                fontSize: 14,
                '&:hover': { color: 'primary.main' },
                transition: 'color 0.15s, border-color 0.15s',
              }}
            >
              {item.icon}
              <span>{item.label}</span>
            </Box>
          ))}
        </Box>

        {/* Cart badge */}
        <Box
          onClick={() => navigate('/')}
          sx={{
            display: 'flex', alignItems: 'center', gap: 0.5,
            px: 2, py: 1, cursor: 'pointer',
            color: 'text.secondary', '&:hover': { color: 'primary.main' },
          }}
        >
          <Badge badgeContent={items.length || undefined} color="primary">
            <ShoppingCartIcon />
          </Badge>
          <Typography variant="body2" sx={{ fontSize: 14 }}>הסל שלי</Typography>
        </Box>
      </Toolbar>
    </AppBar>
  );
};

export default Header;

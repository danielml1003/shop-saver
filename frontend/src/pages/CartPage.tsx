import { Navigate } from 'react-router-dom';

// The grocery list now lives in ComparePage via CartContext — /cart just redirects.
const CartPage = () => <Navigate to="/" replace />;

export default CartPage;

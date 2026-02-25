export const GOURMET_BASE_URL = 'https://alaclickneu.gourmet.at';
export const GOURMET_LOGIN_URL = `${GOURMET_BASE_URL}/start/`;
export const GOURMET_MENUS_URL = `${GOURMET_BASE_URL}/menus`;
export const GOURMET_ORDERS_URL = `${GOURMET_BASE_URL}/bestellungen`;
export const GOURMET_ADD_TO_CART_URL = `${GOURMET_BASE_URL}/umbraco/api/AlaCartApi/AddToMenuesCart`;
export const GOURMET_BILLING_URL = `${GOURMET_BASE_URL}/umbraco/api/AlaMyBillingApi/GetMyBillings`;
export const GOURMET_SETTINGS_URL = `${GOURMET_BASE_URL}/einstellungen/`;

export const MENU_CACHE_VALIDITY_MS = 4 * 60 * 60 * 1000; // 4 hours

// Secure storage keys for Gourmet credentials
export const CREDENTIALS_KEY_USER = 'gourmet_username';
export const CREDENTIALS_KEY_PASS = 'gourmet_password';

// Ventopay (vending machines / POS billing)
export const VENTOPAY_BASE_URL = 'https://my.ventopay.com/mocca.website';
export const VENTOPAY_LOGIN_URL = `${VENTOPAY_BASE_URL}/Login.aspx`;
export const VENTOPAY_TRANSACTIONS_URL = `${VENTOPAY_BASE_URL}/Transaktionen.aspx`;
export const VENTOPAY_LOGOUT_URL = `${VENTOPAY_BASE_URL}/Ausloggen.aspx`;
export const VENTOPAY_COMPANY_ID = '0da8d3ec-0178-47d5-9ccd-a996f04acb61';

// Demo mode credentials (for App Store review)
export const DEMO_USERNAME = 'demo';
export const DEMO_PASSWORD = 'demo1234!';

export function isDemoCredentials(username: string, password: string): boolean {
  return username.toLowerCase() === DEMO_USERNAME && password === DEMO_PASSWORD;
}

// Location-based notifications
export const GEOFENCE_TASK_NAME = 'COMPANY_GEOFENCE_TASK';
export const BACKGROUND_ORDER_SYNC_TASK = 'BACKGROUND_ORDER_SYNC_TASK';
export const COMPANY_GEOFENCE_RADIUS_M = 500;
export const NOTIFICATION_HOUR = 8;
export const NOTIFICATION_MINUTE = 45;
export const NOTIFICATION_CHANNEL_ID = 'order-reminders';

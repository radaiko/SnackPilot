import { GourmetHttpClient } from './gourmetClient';
import {
  extractLoginFormTokens,
  extractUserInfo,
  isLoggedIn,
  parseMenuItems,
  hasNextMenuPage,
  parseOrderedMenus,
  extractEditModeFormData,
  extractCancelOrderFormData,
  extractLogoutFormTokens,
} from './gourmetParser';
import { AddToCartRequest, GetBillingsRequest, BillingApiBill, BillingApiResponse, SessionExpiredError } from './types';
import { GourmetMenuItem, GourmetUserInfo } from '../types/menu';
import { GourmetOrderedMenu } from '../types/order';
import { GourmetBill, GourmetBillingItem } from '../types/billing';
import {
  GOURMET_LOGIN_URL,
  GOURMET_MENUS_URL,
  GOURMET_ORDERS_URL,
  GOURMET_ADD_TO_CART_URL,
  GOURMET_BILLING_URL,
} from '../utils/constants';
import { formatGourmetDate } from '../utils/dateUtils';

const MAX_MENU_PAGES = 10;

export class GourmetApi {
  private client: GourmetHttpClient;
  private userInfo: GourmetUserInfo | null = null;
  private credentials: { username: string; password: string } | null = null;

  constructor() {
    this.client = new GourmetHttpClient();
  }

  /**
   * Check if the response HTML indicates an active session.
   * If not, attempt to re-login with saved credentials.
   * Throws SessionExpiredError if re-login is not possible.
   */
  private async ensureSession(html: string): Promise<string> {
    if (isLoggedIn(html)) return html;

    if (this.credentials) {
      await this.login(this.credentials.username, this.credentials.password);
      return ''; // Caller should re-fetch
    }

    throw new SessionExpiredError();
  }

  /**
   * Login to the Gourmet system.
   *
   * CRITICAL: This follows the exact sequence observed via Playwright:
   * 1. GET /start/ to load login form and extract CSRF tokens
   * 2. POST /start/ with Username, Password, RememberMe, ufprt, __ncforminfo
   * 3. Verify login by checking for settings link in response
   * 4. Extract user info from the redirected page
   */
  async login(username: string, password: string): Promise<GourmetUserInfo> {
    // Step 1: GET login page to extract CSRF tokens
    let loginPageHtml = await this.client.get(GOURMET_LOGIN_URL);

    // Handle stale session: native cookies (NSURLSession/OkHttp) persist across
    // app restarts. If the server-side session is still valid, we get an
    // authenticated page instead of the login form. Logout first to clear it.
    if (isLoggedIn(loginPageHtml)) {
      try {
        const logoutTokens = extractLogoutFormTokens(loginPageHtml);
        await this.client.postForm('/start/', {
          ufprt: logoutTokens.ufprt,
          __ncforminfo: logoutTokens.ncforminfo,
        });
      } catch {
        // Logout failure is non-critical — session may have expired mid-check
      }
      loginPageHtml = await this.client.get(GOURMET_LOGIN_URL);
    }

    const tokens = extractLoginFormTokens(loginPageHtml);

    // Step 2: POST login form with ALL required fields
    const responseHtml = await this.client.postForm('/start/', {
      Username: username,
      Password: password,
      RememberMe: 'false',
      ufprt: tokens.ufprt,
      __ncforminfo: tokens.ncforminfo,
    });

    // Step 3: Verify login success
    if (!isLoggedIn(responseHtml)) {
      throw new Error('Login failed: invalid credentials or account blocked');
    }

    // Step 4: Extract user info from POST response, or re-fetch start page
    try {
      this.userInfo = extractUserInfo(responseHtml);
    } catch {
      // POST response may redirect to a page without hidden inputs;
      // fetch the start page explicitly to get them.
      const startHtml = await this.client.get(GOURMET_LOGIN_URL);
      this.userInfo = extractUserInfo(startHtml);
    }
    this.credentials = { username, password };
    return this.userInfo;
  }

  /**
   * Get the currently logged-in user info.
   */
  getUserInfo(): GourmetUserInfo | null {
    return this.userInfo;
  }

  /**
   * Fetch all menu items across all pages.
   */
  async getMenus(): Promise<GourmetMenuItem[]> {
    const allItems: GourmetMenuItem[] = [];

    for (let page = 0; page < MAX_MENU_PAGES; page++) {
      const url = `${GOURMET_MENUS_URL}/`;
      const params = page === 0 ? undefined : { page: String(page) };

      let html = await this.client.get(url, params);

      // On first page, check session and re-fetch if expired
      if (page === 0) {
        const checked = await this.ensureSession(html);
        if (checked === '') {
          // Re-fetched after re-login
          html = await this.client.get(url, params);
        }
        if (!this.userInfo) {
          try {
            this.userInfo = extractUserInfo(html);
          } catch {
            // User info extraction may fail
          }
        }
      }

      const items = parseMenuItems(html);
      allItems.push(...items);

      if (!hasNextMenuPage(html)) {
        break;
      }
    }

    return allItems;
  }

  /**
   * Fetch current orders.
   */
  async getOrders(): Promise<GourmetOrderedMenu[]> {
    let html = await this.client.get(`${GOURMET_ORDERS_URL}/`);

    const checked = await this.ensureSession(html);
    if (checked === '') {
      html = await this.client.get(`${GOURMET_ORDERS_URL}/`);
    }

    return parseOrderedMenus(html);
  }

  /**
   * Add menu items to cart (order them).
   *
   * @param items Array of { date: Date, menuId: string } to order
   */
  async addToCart(items: { date: Date; menuId: string }[]): Promise<void> {
    if (!this.userInfo) {
      throw new Error('Not logged in');
    }

    // Group items by date
    const dateMap = new Map<string, string[]>();
    for (const item of items) {
      const dateStr = formatGourmetDate(item.date);
      const existing = dateMap.get(dateStr) || [];
      existing.push(item.menuId);
      dateMap.set(dateStr, existing);
    }

    const request: AddToCartRequest = {
      eaterId: this.userInfo.eaterId,
      shopModelId: this.userInfo.shopModelId,
      staffgroupId: this.userInfo.staffGroupId,
      dates: Array.from(dateMap.entries()).map(([date, menuIds]) => ({
        date,
        menuIds,
      })),
    };

    const response = await this.client.postJson<{ success: boolean; message?: string }>(
      GOURMET_ADD_TO_CART_URL,
      request
    );

    if (!response?.success) {
      throw new Error(`Add to cart failed: ${response?.message || 'unknown error'}`);
    }
  }

  /**
   * Confirm pending orders by toggling edit mode on the orders page.
   * After AddToMenuesCart, orders are unconfirmed until this is called.
   */
  async confirmOrders(): Promise<void> {
    let html = await this.client.get(`${GOURMET_ORDERS_URL}/`);
    const checked = await this.ensureSession(html);
    if (checked === '') {
      html = await this.client.get(`${GOURMET_ORDERS_URL}/`);
    }

    // editMode="False" means we ARE in edit mode (inverted semantics).
    // Only confirm (exit edit mode) if currently in edit mode.
    const editData = extractEditModeFormData(html);
    if (editData.editMode === 'False') {
      // In edit mode — toggle to exit (confirm)
      await this.client.postForm('/bestellungen/', {
        editMode: editData.editMode,
        ufprt: editData.ufprt,
        __ncforminfo: editData.ncforminfo,
      });
    }
    // If editMode="True", already confirmed — nothing to do
  }

  /**
   * Cancel specific orders. Requires entering edit mode first.
   *
   * @param positionIds Array of position IDs to cancel
   */
  async cancelOrders(positionIds: string[]): Promise<void> {
    // Step 1: Get the orders page
    let ordersHtml = await this.client.get(`${GOURMET_ORDERS_URL}/`);
    const checked = await this.ensureSession(ordersHtml);
    if (checked === '') {
      ordersHtml = await this.client.get(`${GOURMET_ORDERS_URL}/`);
    }

    // Step 2: Enter edit mode (only if not already in it)
    // editMode="False" means we ARE in edit mode (inverted semantics)
    const editModeData = extractEditModeFormData(ordersHtml);
    const alreadyInEditMode = editModeData.editMode === 'False';

    if (!alreadyInEditMode) {
      // Toggle to enter edit mode
      await this.client.postForm('/bestellungen/', {
        editMode: editModeData.editMode,
        ufprt: editModeData.ufprt,
        __ncforminfo: editModeData.ncforminfo,
      });

      // Re-fetch the orders page to get the edit mode state reliably
      // (POST redirect responses may not reflect the updated state)
      ordersHtml = await this.client.get(`${GOURMET_ORDERS_URL}/`);

      // Verify we entered edit mode
      const verifyData = extractEditModeFormData(ordersHtml);
      if (verifyData.editMode !== 'False') {
        throw new Error('Failed to enter edit mode');
      }
    }

    // Step 3: Cancel each order (cancel forms only exist in edit mode)
    for (const positionId of positionIds) {
      const cancelData = extractCancelOrderFormData(ordersHtml, positionId);

      const eatingCycleKey = `cp_EatingCycleId_${positionId}`;
      const dateKey = `cp_Date_${positionId}`;

      const formData: Record<string, string> = {
        cp_PositionId: cancelData.positionId,
        [eatingCycleKey]: cancelData.eatingCycleId,
        [dateKey]: cancelData.date,
        ufprt: cancelData.ufprt,
        __ncforminfo: cancelData.ncforminfo,
      };

      await this.client.postForm('/bestellungen/', formData);
      // Re-fetch to get fresh tokens for the next cancellation
      ordersHtml = await this.client.get(`${GOURMET_ORDERS_URL}/`);
    }

    // Exit edit mode after cancel so the page returns to normal state
    const exitEditData = extractEditModeFormData(ordersHtml);
    if (exitEditData.editMode === 'False') {
      await this.client.postForm('/bestellungen/', {
        editMode: exitEditData.editMode,
        ufprt: exitEditData.ufprt,
        __ncforminfo: exitEditData.ncforminfo,
      });
    }
  }

  /**
   * Fetch billing data for a given month offset.
   *
   * @param checkLastMonthNumber "0" = current month, "1" = last month, "2" = 2 months ago
   */
  async getBillings(checkLastMonthNumber: string): Promise<GourmetBill[]> {
    if (!this.userInfo) {
      throw new Error('Not logged in');
    }

    // Ensure session is active by hitting a page first
    const html = await this.client.get(GOURMET_LOGIN_URL);
    await this.ensureSession(html);

    const request: GetBillingsRequest = {
      eaterId: this.userInfo.eaterId,
      shopModelId: this.userInfo.shopModelId,
      checkLastMonthNumber,
    };

    const response = await this.client.postJson<BillingApiResponse | BillingApiBill[]>(
      GOURMET_BILLING_URL,
      request
    );

    // Server wraps the array in {"Billings": [...]}; handle both formats
    const bills = Array.isArray(response)
      ? response
      : (response as BillingApiResponse)?.Billings ?? [];

    return bills.map((bill) => ({
      billNr: bill.BillNr,
      billDate: new Date(bill.BillDate),
      location: bill.Location,
      items: bill.BillingItemInfo.map(
        (item): GourmetBillingItem => ({
          id: item.Id,
          articleId: item.ArticleId,
          count: item.Count,
          description: item.Description,
          total: item.Total,
          subsidy: item.Subsidy,
          discountValue: item.DiscountValue,
          isCustomMenu: item.IsCustomMenu,
        })
      ),
      billing: bill.Billing,
    }));
  }

  /**
   * Logout from the Gourmet system.
   */
  async logout(): Promise<void> {
    try {
      // Get any authenticated page to extract logout form tokens
      const html = await this.client.get(GOURMET_LOGIN_URL);
      const tokens = extractLogoutFormTokens(html);

      await this.client.postForm('/start/', {
        ufprt: tokens.ufprt,
        __ncforminfo: tokens.ncforminfo,
      });
    } catch {
      // Logout failure is non-critical
    } finally {
      this.userInfo = null;
      this.credentials = null;
      this.client.resetClient();
    }
  }

  /** Check if we have an active session */
  isAuthenticated(): boolean {
    return this.userInfo !== null;
  }
}

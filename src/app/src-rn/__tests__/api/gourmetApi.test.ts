import * as fs from 'fs';
import * as path from 'path';

jest.mock('../../api/gourmetClient');
import { GourmetHttpClient } from '../../api/gourmetClient';
import { GourmetApi } from '../../api/gourmetApi';

const fixturesDir = path.join(__dirname, '..', 'fixtures');
const loadFixture = (filepath: string) => fs.readFileSync(path.join(fixturesDir, filepath), 'utf-8');

const loginPage = loadFixture('gourmet/login-page.html');
const loginSuccess = loadFixture('gourmet/login-success.html');
const loginFailed = loadFixture('gourmet/login-failed.html');
const menusPage0 = loadFixture('gourmet/menus-page-0.html');
const menusPage1 = loadFixture('gourmet/menus-page-1.html');
const ordersPage = loadFixture('gourmet/orders-page.html');
const ordersPageEditMode = loadFixture('gourmet/orders-page-edit-mode.html');
const billingJson = JSON.parse(loadFixture('gourmet/billing-current.json'));

const mockGet = jest.fn();
const mockPostForm = jest.fn();
const mockPostJson = jest.fn();
const mockResetClient = jest.fn();

beforeEach(() => {
  jest.clearAllMocks();
  (GourmetHttpClient as jest.MockedClass<typeof GourmetHttpClient>).mockImplementation(() => ({
    get: mockGet,
    postForm: mockPostForm,
    postJson: mockPostJson,
    resetClient: mockResetClient,
  } as any));
});

/** Helper: login the API so userInfo is populated */
async function loginApi(api: GourmetApi) {
  mockGet.mockResolvedValueOnce(loginPage);
  mockPostForm.mockResolvedValueOnce(loginSuccess);
  await api.login('testuser', 'testpass');
  jest.clearAllMocks();
}

describe('GourmetApi', () => {
  describe('login', () => {
    it('calls GET on login URL first', async () => {
      mockGet.mockResolvedValueOnce(loginPage);
      mockPostForm.mockResolvedValueOnce(loginSuccess);

      const api = new GourmetApi();
      await api.login('testuser', 'testpass');

      expect(mockGet).toHaveBeenCalledWith('https://alaclickneu.gourmet.at/start/');
    });

    it('calls postForm with correct form data including ufprt and __ncforminfo', async () => {
      mockGet.mockResolvedValueOnce(loginPage);
      mockPostForm.mockResolvedValueOnce(loginSuccess);

      const api = new GourmetApi();
      await api.login('testuser', 'testpass');

      expect(mockPostForm).toHaveBeenCalledWith('/start/', {
        Username: 'testuser',
        Password: 'testpass',
        RememberMe: 'false',
        ufprt: 'CSRF-TOKEN-LOGIN-ABC123',
        __ncforminfo: 'NCFORM-TOKEN-LOGIN-XYZ789',
      });
    });

    it('returns user info on success', async () => {
      mockGet.mockResolvedValueOnce(loginPage);
      mockPostForm.mockResolvedValueOnce(loginSuccess);

      const api = new GourmetApi();
      const userInfo = await api.login('testuser', 'testpass');

      expect(userInfo).toEqual({
        username: 'TestUser',
        shopModelId: 'SM-TEST-123',
        eaterId: 'EATER-TEST-456',
        staffGroupId: 'SG-TEST-789',
      });
    });

    it('throws on login failure', async () => {
      mockGet.mockResolvedValueOnce(loginPage);
      mockPostForm.mockResolvedValueOnce(loginFailed);

      const api = new GourmetApi();
      await expect(api.login('baduser', 'badpass')).rejects.toThrow(
        'Login failed: invalid credentials or account blocked'
      );
    });

    it('stores credentials for re-login', async () => {
      mockGet.mockResolvedValueOnce(loginPage);
      mockPostForm.mockResolvedValueOnce(loginSuccess);

      const api = new GourmetApi();
      await api.login('testuser', 'testpass');

      expect(api.isAuthenticated()).toBe(true);
    });

    it('logs out stale session before login when native cookies persist', async () => {
      const api = new GourmetApi();

      // GET /start/ returns authenticated page (stale native cookies)
      mockGet.mockResolvedValueOnce(loginSuccess);
      // Logout POST
      mockPostForm.mockResolvedValueOnce('');
      // Re-GET /start/ returns login page after logout
      mockGet.mockResolvedValueOnce(loginPage);
      // Actual login POST
      mockPostForm.mockResolvedValueOnce(loginSuccess);

      const userInfo = await api.login('testuser', 'testpass');

      // Should have called GET twice (stale check + after logout)
      expect(mockGet).toHaveBeenCalledTimes(2);
      // First postForm is logout, second is the actual login
      expect(mockPostForm).toHaveBeenCalledTimes(2);
      expect(mockPostForm).toHaveBeenNthCalledWith(1, '/start/', {
        ufprt: 'CSRF-TOKEN-LOGOUT-DEF456',
        __ncforminfo: 'NCFORM-TOKEN-LOGOUT-UVW012',
      });
      expect(mockPostForm).toHaveBeenNthCalledWith(2, '/start/', {
        Username: 'testuser',
        Password: 'testpass',
        RememberMe: 'false',
        ufprt: 'CSRF-TOKEN-LOGIN-ABC123',
        __ncforminfo: 'NCFORM-TOKEN-LOGIN-XYZ789',
      });
      expect(userInfo.username).toBe('TestUser');
    });
  });

  describe('getMenus', () => {
    it('paginates through all pages until no next link', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      mockGet.mockResolvedValueOnce(menusPage0); // page 0
      mockGet.mockResolvedValueOnce(menusPage1); // page 1

      await api.getMenus();

      // page 0 has no params, page 1 has page=1
      expect(mockGet).toHaveBeenCalledTimes(2);
      expect(mockGet).toHaveBeenNthCalledWith(1, 'https://alaclickneu.gourmet.at/menus/', undefined);
      expect(mockGet).toHaveBeenNthCalledWith(2, 'https://alaclickneu.gourmet.at/menus/', { page: '1' });
    });

    it('returns combined items from all pages', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      mockGet.mockResolvedValueOnce(menusPage0); // 7 items (desktop layout only)
      mockGet.mockResolvedValueOnce(menusPage1); // 2 items

      const items = await api.getMenus();

      // menus-page-0 has 7 items in desktop layout, menus-page-1 has 2
      expect(items.length).toBe(9);
    });

    it('stops when no next page link exists', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      // Only one page without next link
      mockGet.mockResolvedValueOnce(menusPage1);

      const items = await api.getMenus();

      expect(mockGet).toHaveBeenCalledTimes(1);
      expect(items.length).toBe(2);
    });
  });

  describe('getOrders', () => {
    it('returns parsed orders from orders page', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      mockGet.mockResolvedValueOnce(ordersPage);

      const orders = await api.getOrders();

      expect(orders).toHaveLength(3);
      expect(orders[0].positionId).toBe('POS-001');
      expect(orders[0].title).toBe('MENÜ I');
      expect(orders[0].approved).toBe(true); // has fa-check
      expect(orders[1].positionId).toBe('POS-002');
      expect(orders[1].approved).toBe(false);
      expect(orders[2].positionId).toBe('POS-003');
      expect(orders[2].approved).toBe(true); // has checkmark class
    });

    it('re-fetches page after session re-login', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      // First GET returns unauthenticated page -> triggers re-login
      mockGet.mockResolvedValueOnce(loginFailed); // not logged in
      // Re-login sequence
      mockGet.mockResolvedValueOnce(loginPage);
      mockPostForm.mockResolvedValueOnce(loginSuccess);
      // Re-fetch orders
      mockGet.mockResolvedValueOnce(ordersPage);

      const orders = await api.getOrders();
      expect(orders).toHaveLength(3);
    });
  });

  describe('addToCart', () => {
    it('sends correct JSON request with user info and cart items', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      mockPostJson.mockResolvedValueOnce({ success: true });

      await api.addToCart([
        { date: new Date(2026, 1, 10), menuId: 'menu-001' },
        { date: new Date(2026, 1, 10), menuId: 'menu-004' },
        { date: new Date(2026, 1, 11), menuId: 'menu-001' },
      ]);

      expect(mockPostJson).toHaveBeenCalledWith(
        'https://alaclickneu.gourmet.at/umbraco/api/AlaCartApi/AddToMenuesCart',
        {
          eaterId: 'EATER-TEST-456',
          shopModelId: 'SM-TEST-123',
          staffgroupId: 'SG-TEST-789',
          dates: [
            { date: '02-10-2026', menuIds: ['menu-001', 'menu-004'] },
            { date: '02-11-2026', menuIds: ['menu-001'] },
          ],
        }
      );
    });

    it('throws if not logged in', async () => {
      const api = new GourmetApi();

      await expect(
        api.addToCart([{ date: new Date(), menuId: 'menu-001' }])
      ).rejects.toThrow('Not logged in');
    });

    it('throws if API returns success:false', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      mockPostJson.mockResolvedValueOnce({ success: false, message: 'Order deadline passed' });

      await expect(
        api.addToCart([{ date: new Date(2026, 1, 10), menuId: 'menu-001' }])
      ).rejects.toThrow('Add to cart failed: Order deadline passed');
    });
  });

  describe('confirmOrders', () => {
    it('posts edit mode toggle when editMode is False (in edit mode)', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      // ordersPageEditMode has editMode="False" (meaning we ARE in edit mode)
      mockGet.mockResolvedValueOnce(ordersPageEditMode);
      mockPostForm.mockResolvedValueOnce('');

      await api.confirmOrders();

      expect(mockPostForm).toHaveBeenCalledWith('/bestellungen/', {
        editMode: 'False',
        ufprt: 'CSRF-TOKEN-EDITMODE-EXIT-777',
        __ncforminfo: 'NCFORM-TOKEN-EDITMODE-EXIT-888',
      });
    });

    it('does not post when editMode is True (already confirmed)', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      // ordersPage has editMode="True" (not in edit mode = already confirmed)
      mockGet.mockResolvedValueOnce(ordersPage);

      await api.confirmOrders();

      expect(mockPostForm).not.toHaveBeenCalled();
    });
  });

  describe('cancelOrders', () => {
    it('always includes __ncforminfo in cancel form payload', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      mockGet.mockResolvedValueOnce(ordersPageEditMode);
      mockPostForm.mockResolvedValueOnce('');
      mockGet.mockResolvedValueOnce(ordersPageEditMode);
      mockPostForm.mockResolvedValueOnce('');

      await api.cancelOrders(['POS-001']);

      expect(mockPostForm).toHaveBeenNthCalledWith(
        1,
        '/bestellungen/',
        expect.objectContaining({
          cp_PositionId: 'POS-001',
          __ncforminfo: 'NCFORM-TOKEN-CANCEL-POS001-BBB',
        })
      );
    });
  });

  describe('getBillings', () => {
    it('returns mapped billing data with correct structure', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      // ensureSession: GET login URL + check logged in
      mockGet.mockResolvedValueOnce(loginSuccess);
      mockPostJson.mockResolvedValueOnce(billingJson);

      const bills = await api.getBillings('0');

      expect(bills).toHaveLength(2);
      expect(bills[0].billNr).toBe(10001);
      expect(bills[0].location).toBe('Betriebsrestaurant Wien');
      expect(bills[0].billing).toBe(4.50);
      expect(bills[0].items).toHaveLength(2);
      expect(bills[0].items[0]).toEqual({
        id: 'ITEM-001',
        articleId: 'ART-001',
        count: 1,
        description: 'Menü I - Wiener Schnitzel',
        total: 5.50,
        subsidy: 2.50,
        discountValue: 0.00,
        isCustomMenu: false,
      });

      expect(bills[1].billNr).toBe(10002);
      expect(bills[1].items).toHaveLength(1);
      expect(bills[1].billing).toBe(3.00);

      // Verify postJson was called with correct request
      expect(mockPostJson).toHaveBeenCalledWith(
        'https://alaclickneu.gourmet.at/umbraco/api/AlaMyBillingApi/GetMyBillings',
        {
          eaterId: 'EATER-TEST-456',
          shopModelId: 'SM-TEST-123',
          checkLastMonthNumber: '0',
        }
      );
    });
  });

  describe('logout', () => {
    it('calls postForm with logout tokens and resets client', async () => {
      const api = new GourmetApi();
      await loginApi(api);

      // GET page to extract logout tokens
      mockGet.mockResolvedValueOnce(loginSuccess);
      mockPostForm.mockResolvedValueOnce('');

      await api.logout();

      expect(mockPostForm).toHaveBeenCalledWith('/start/', {
        ufprt: 'CSRF-TOKEN-LOGOUT-DEF456',
        __ncforminfo: 'NCFORM-TOKEN-LOGOUT-UVW012',
      });
      expect(mockResetClient).toHaveBeenCalled();
      expect(api.isAuthenticated()).toBe(false);
    });
  });
});

import * as fs from 'fs';
import * as path from 'path';

jest.mock('../../api/ventopayClient');
import { VentopayHttpClient } from '../../api/ventopayClient';
import { VentopayApi } from '../../api/ventopayApi';

const fixturesDir = path.join(__dirname, '..', 'fixtures');
const loadFixture = (filepath: string) => fs.readFileSync(path.join(fixturesDir, filepath), 'utf-8');

const loginPage = loadFixture('ventopay/login-page.html');
const loginSuccess = loadFixture('ventopay/login-success.html');
const transactionsPage = loadFixture('ventopay/transactions-page.html');
const transactionsEmpty = loadFixture('ventopay/transactions-empty.html');

const mockGet = jest.fn();
const mockPostForm = jest.fn();
const mockResetClient = jest.fn();
const mockGetCookieDebug = jest.fn().mockReturnValue('');

beforeEach(() => {
  jest.clearAllMocks();
  (VentopayHttpClient as jest.MockedClass<typeof VentopayHttpClient>).mockImplementation(() => ({
    get: mockGet,
    postForm: mockPostForm,
    getCookieDebug: mockGetCookieDebug,
    resetClient: mockResetClient,
  } as any));
});

/** Helper: login the API so session is active */
async function loginApi(api: VentopayApi) {
  mockGet.mockResolvedValueOnce(loginPage);
  mockPostForm.mockResolvedValueOnce(loginSuccess);
  await api.login('testuser', 'testpass');
  jest.clearAllMocks();
}

describe('VentopayApi', () => {
  describe('login', () => {
    it('calls GET on login URL first', async () => {
      mockGet.mockResolvedValueOnce(loginPage);
      mockPostForm.mockResolvedValueOnce(loginSuccess);

      const api = new VentopayApi();
      await api.login('testuser', 'testpass');

      expect(mockGet).toHaveBeenCalledWith('https://my.ventopay.com/mocca.website/Login.aspx');
    });

    it('calls postForm with all ASP.NET fields and credentials', async () => {
      mockGet.mockResolvedValueOnce(loginPage);
      mockPostForm.mockResolvedValueOnce(loginSuccess);

      const api = new VentopayApi();
      await api.login('testuser', 'testpass');

      expect(mockPostForm).toHaveBeenCalledWith(
        'https://my.ventopay.com/mocca.website/Login.aspx',
        {
          __LASTFOCUS: '',
          __EVENTTARGET: '',
          __EVENTARGUMENT: '',
          __VIEWSTATE: 'VIEWSTATE-TOKEN-LONG-BASE64-STRING-ABC123',
          __VIEWSTATEGENERATOR: 'ABCD1234',
          __EVENTVALIDATION: 'EVENTVALIDATION-TOKEN-XYZ789',
          DropDownList1: '0da8d3ec-0178-47d5-9ccd-a996f04acb61',
          TxtUsername: 'testuser',
          TxtPassword: 'testpass',
          BtnLogin: 'Login',
          languageRadio: 'DE',
        }
      );
    });

    it('throws on login failure', async () => {
      mockGet.mockResolvedValueOnce(loginPage);
      // Return a page without Ausloggen.aspx link
      mockPostForm.mockResolvedValueOnce('<html><body>Login failed</body></html>');

      const api = new VentopayApi();
      await expect(api.login('baduser', 'badpass')).rejects.toThrow(
        'Ventopay login failed: invalid credentials or account blocked'
      );
    });

    it('sets loggedIn state on success', async () => {
      mockGet.mockResolvedValueOnce(loginPage);
      mockPostForm.mockResolvedValueOnce(loginSuccess);

      const api = new VentopayApi();
      expect(api.isAuthenticated()).toBe(false);

      await api.login('testuser', 'testpass');

      expect(api.isAuthenticated()).toBe(true);
    });
  });

  describe('getTransactions', () => {
    it('returns parsed transactions with Gourmet filtered out', async () => {
      const api = new VentopayApi();
      await loginApi(api);

      mockGet.mockResolvedValueOnce(transactionsPage);

      const fromDate = new Date(2026, 1, 9);
      const toDate = new Date(2026, 1, 11);
      const transactions = await api.getTransactions(fromDate, toDate);

      // 6 total transactions in fixture, 1 Gourmet filtered out = 5
      expect(transactions).toHaveLength(5);
      expect(transactions[0].id).toBe('dHhuLTAwMQ==');
      expect(transactions[0].amount).toBe(1.80);
      expect(transactions[0].restaurant).toBe('Café + Co. Automaten');
      expect(transactions[1].id).toBe('dHhuLTAwMg==');
      expect(transactions[2].id).toBe('dHhuLTAwNA==');
    });

    it('passes formatted date params (dd.MM.yyyy)', async () => {
      const api = new VentopayApi();
      await loginApi(api);

      mockGet.mockResolvedValueOnce(transactionsEmpty);

      const fromDate = new Date(2026, 1, 1);
      const toDate = new Date(2026, 1, 28);
      await api.getTransactions(fromDate, toDate);

      expect(mockGet).toHaveBeenCalledWith(
        'https://my.ventopay.com/mocca.website/Transaktionen.aspx',
        {
          fromDate: '01.02.2026',
          untilDate: '28.02.2026',
        }
      );
    });

    it('re-logins on session expiry and retries', async () => {
      const api = new VentopayApi();
      await loginApi(api);

      // First GET returns page without Ausloggen link (session expired)
      mockGet.mockResolvedValueOnce('<html><body>Session expired</body></html>');
      // Re-login sequence
      mockGet.mockResolvedValueOnce(loginPage);
      mockPostForm.mockResolvedValueOnce(loginSuccess);
      // Retry fetch
      mockGet.mockResolvedValueOnce(transactionsPage);

      const transactions = await api.getTransactions(new Date(2026, 1, 9), new Date(2026, 1, 11));

      expect(transactions).toHaveLength(5);
      // 3 GET calls: expired page, login page, retry transactions
      expect(mockGet).toHaveBeenCalledTimes(3);
    });
  });

  describe('logout', () => {
    it('calls GET on logout URL and resets client', async () => {
      const api = new VentopayApi();
      await loginApi(api);

      mockGet.mockResolvedValueOnce('');

      await api.logout();

      expect(mockGet).toHaveBeenCalledWith('https://my.ventopay.com/mocca.website/Ausloggen.aspx');
      expect(mockResetClient).toHaveBeenCalled();
      expect(api.isAuthenticated()).toBe(false);
    });
  });
});

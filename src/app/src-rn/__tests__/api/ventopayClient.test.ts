const mockGet = jest.fn();
const mockPost = jest.fn();
let responseInterceptor: (response: any) => any;
let requestInterceptor: (config: any) => any;

jest.mock('axios', () => ({
  __esModule: true,
  default: {
    create: jest.fn(() => ({
      get: mockGet,
      post: mockPost,
      interceptors: {
        response: {
          use: jest.fn((fn: any) => {
            responseInterceptor = fn;
          }),
        },
        request: {
          use: jest.fn((fn: any) => {
            requestInterceptor = fn;
          }),
        },
      },
    })),
  },
}));

describe('VentopayHttpClient', () => {
  beforeEach(() => {
    jest.resetModules();
    jest.clearAllMocks();

    mockGet.mockResolvedValue({ data: '<html>test</html>', status: 200 });
    mockPost.mockResolvedValue({ data: '<html>post result</html>', status: 200 });
  });

  function createClient() {
    const { VentopayHttpClient } = require('../../api/ventopayClient');
    return new VentopayHttpClient();
  }

  function getAxiosMock() {
    return require('axios').default;
  }

  describe('constructor', () => {
    it('creates axios instance with withCredentials false', () => {
      createClient();
      const axios = getAxiosMock();

      expect(axios.create).toHaveBeenCalledWith(
        expect.objectContaining({
          baseURL: 'https://my.ventopay.com/mocca.website',
          withCredentials: false,
          maxRedirects: 5,
        })
      );
    });

    it('registers response and request interceptors', () => {
      createClient();
      const axios = getAxiosMock();
      const mockInstance = axios.create.mock.results[0].value;

      expect(mockInstance.interceptors.response.use).toHaveBeenCalledTimes(1);
      expect(mockInstance.interceptors.request.use).toHaveBeenCalledTimes(1);
    });
  });

  describe('get()', () => {
    it('calls client.get with url and params', async () => {
      const client = createClient();
      await client.get('/Transaktionen.aspx', { fromDate: '01.01.2025' });

      expect(mockGet).toHaveBeenCalledWith('/Transaktionen.aspx', {
        params: { fromDate: '01.01.2025' },
        responseType: 'text',
      });
    });

    it('returns response data', async () => {
      const client = createClient();
      const result = await client.get('/Login.aspx');

      expect(result).toBe('<html>test</html>');
    });
  });

  describe('postForm()', () => {
    it('sends URL-encoded data with correct content type', async () => {
      const client = createClient();
      const data = { TxtUsername: 'user', TxtPassword: 'pass' };
      await client.postForm('/Login.aspx', data);

      expect(mockPost).toHaveBeenCalledWith(
        '/Login.aspx',
        new URLSearchParams(data).toString(),
        expect.objectContaining({
          headers: expect.objectContaining({
            'Content-Type': 'application/x-www-form-urlencoded',
          }),
          responseType: 'text',
        })
      );
    });

    it('includes Origin and Referer headers', async () => {
      const client = createClient();
      await client.postForm('/Login.aspx', { key: 'value' });

      expect(mockPost).toHaveBeenCalledWith(
        '/Login.aspx',
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({
            Origin: 'https://my.ventopay.com',
            Referer: expect.any(String),
          }),
        })
      );
    });
  });

  describe('response interceptor', () => {
    it('captures Set-Cookie headers into cookies', () => {
      const client = createClient();

      const response = {
        headers: {
          'set-cookie': ['ASP.NET_SessionId=abc123; path=/; HttpOnly', 'OtherCookie=xyz789; path=/'],
        },
        data: '<html></html>',
        status: 200,
      };

      const result = responseInterceptor(response);

      const debug = client.getCookieDebug();
      expect(debug).toContain('ASP.NET_SessionId');
      expect(debug).toContain('OtherCookie');
      expect(debug).not.toContain('abc123');
      expect(debug).not.toContain('xyz789');
      expect(result).toBe(response);
    });

    it('handles single Set-Cookie string (non-array)', () => {
      const client = createClient();

      const response = {
        headers: {
          'set-cookie': 'SessionId=single123; path=/',
        },
        data: '',
        status: 200,
      };

      responseInterceptor(response);

      expect(client.getCookieDebug()).toContain('SessionId');
      expect(client.getCookieDebug()).not.toContain('single123');
    });
  });

  describe('request interceptor', () => {
    it('injects Cookie header from stored cookies', () => {
      createClient();

      responseInterceptor({
        headers: {
          'set-cookie': ['SessId=abc; path=/', 'Token=def; path=/'],
        },
        data: '',
        status: 200,
      });

      const config = { headers: {} as Record<string, string> };
      const result = requestInterceptor(config);

      expect(result.headers['Cookie']).toBe('SessId=abc; Token=def');
    });

    it('does not inject Cookie header when no cookies stored', () => {
      createClient();

      const config = { headers: {} as Record<string, string> };
      const result = requestInterceptor(config);

      expect(result.headers['Cookie']).toBeUndefined();
    });
  });

  describe('resetClient()', () => {
    it('clears cookies and lastPageUrl', () => {
      const client = createClient();

      responseInterceptor({
        headers: {
          'set-cookie': ['SessId=abc; path=/'],
        },
        data: '',
        status: 200,
      });

      expect(client.getCookieDebug()).toContain('SessId');

      client.resetClient();

      expect(client.getCookieDebug()).toBe('');

      const config = { headers: {} as Record<string, string> };
      const result = requestInterceptor(config);
      expect(result.headers['Cookie']).toBeUndefined();
    });
  });
});

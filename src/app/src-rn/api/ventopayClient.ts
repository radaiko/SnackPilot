import axios, { AxiosInstance, AxiosResponse } from 'axios';
import { VENTOPAY_BASE_URL } from '../utils/constants';

const VENTOPAY_ORIGIN = 'https://my.ventopay.com';

/**
 * Low-level HTTP client for the Ventopay website.
 *
 * Cookies are managed manually via interceptors because React Native's native
 * HTTP stack doesn't reliably persist Set-Cookie headers between requests.
 *
 * CRITICAL: Do not add custom User-Agent headers.
 * CRITICAL: Do not add request throttling/delays.
 * CRITICAL: withCredentials MUST be false — we manage cookies manually.
 *           Setting it to true causes the native layer (NSURLSession) to also
 *           manage cookies, creating a dual-cookie conflict.
 */
export class VentopayHttpClient {
  private client: AxiosInstance;
  private cookies: Map<string, string> = new Map();
  private lastPageUrl: string = '';

  constructor() {
    this.client = axios.create({
      baseURL: VENTOPAY_BASE_URL,
      withCredentials: false,
      maxRedirects: 5,
      validateStatus: (status) => status >= 200 && status < 400,
    });

    // Intercept responses to capture Set-Cookie headers
    this.client.interceptors.response.use((response) => {
      const setCookie = response.headers['set-cookie'];
      if (setCookie) {
        const cookieArray = Array.isArray(setCookie) ? setCookie : [setCookie];
        for (const cookie of cookieArray) {
          const [nameValue] = cookie.split(';');
          const eqIndex = nameValue.indexOf('=');
          if (eqIndex > 0) {
            const name = nameValue.substring(0, eqIndex).trim();
            const value = nameValue.substring(eqIndex + 1).trim();
            this.cookies.set(name, value);
          }
        }
      }
      return response;
    });

    // Intercept requests to inject stored cookies and browser-like headers
    this.client.interceptors.request.use((config) => {
      if (this.cookies.size > 0) {
        const cookieStr = Array.from(this.cookies.entries())
          .map(([name, value]) => `${name}=${value}`)
          .join('; ');
        config.headers['Cookie'] = cookieStr;
      }
      return config;
    });
  }

  /** GET request returning HTML string */
  async get(url: string, params?: Record<string, string>): Promise<string> {
    const response: AxiosResponse<string> = await this.client.get(url, {
      params,
      responseType: 'text',
    });
    // Track the last page URL for Referer on subsequent POSTs
    this.lastPageUrl = typeof url === 'string' && url.startsWith('http')
      ? url
      : `${VENTOPAY_BASE_URL}${url.startsWith('/') ? '' : '/'}${url}`;
    return response.data;
  }

  /** POST form data (application/x-www-form-urlencoded) returning HTML string */
  async postForm(url: string, data: Record<string, string>): Promise<string> {
    const response: AxiosResponse<string> = await this.client.post(
      url,
      new URLSearchParams(data).toString(),
      {
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
          'Origin': VENTOPAY_ORIGIN,
          'Referer': this.lastPageUrl || url,
        },
        responseType: 'text',
      }
    );
    return response.data;
  }

  /** Debug helper: get cookie names only */
  getCookieDebug(): string {
    return Array.from(this.cookies.keys()).join('; ');
  }

  /** Reset client (for logout - clears all stored cookies) */
  resetClient(): void {
    this.cookies.clear();
    this.lastPageUrl = '';
  }
}

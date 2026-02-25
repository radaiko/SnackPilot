import axios, { AxiosInstance, AxiosResponse } from 'axios';
import { GOURMET_BASE_URL } from '../utils/constants';

const GOURMET_ORIGIN = 'https://alaclickneu.gourmet.at';

/**
 * Low-level HTTP client for the Gourmet website.
 *
 * Uses withCredentials: true so iOS's native NSURLSession handles cookies.
 * This is necessary because the Gourmet server's set-cookie headers are
 * consumed by NSURLSession and not exposed to JavaScript.
 *
 * All Gourmet forms use enctype="multipart/form-data", so postForm uses
 * axios.postForm() which handles multipart encoding + boundary automatically.
 *
 * CRITICAL: Do not add custom User-Agent headers.
 * CRITICAL: Do not add request throttling/delays.
 */
export class GourmetHttpClient {
  private client: AxiosInstance;
  private lastPageUrl: string = '';

  constructor() {
    this.client = axios.create({
      baseURL: GOURMET_BASE_URL,
      withCredentials: true,
      maxRedirects: 5,
      validateStatus: (status) => status >= 200 && status < 400,
    });
  }

  /** GET request returning HTML string */
  async get(url: string, params?: Record<string, string>): Promise<string> {
    const response: AxiosResponse<string> = await this.client.get(url, {
      params,
      responseType: 'text',
    });
    this.lastPageUrl = typeof url === 'string' && url.startsWith('http')
      ? url
      : `${GOURMET_BASE_URL}${url.startsWith('/') ? '' : '/'}${url}`;
    return response.data;
  }

  /** POST form data (multipart/form-data) returning HTML string.
   *  All Gourmet forms use enctype="multipart/form-data". */
  async postForm(url: string, data: Record<string, string>): Promise<string> {
    const response: AxiosResponse<string> = await this.client.postForm(
      url,
      data,
      {
        headers: {
          'Origin': GOURMET_ORIGIN,
          'Referer': this.lastPageUrl || url,
        },
        responseType: 'text',
      }
    );
    return response.data;
  }

  /** POST JSON data returning JSON response */
  async postJson<T>(url: string, data: unknown): Promise<T> {
    const response: AxiosResponse<T> = await this.client.post(url, data, {
      headers: {
        'Content-Type': 'application/json',
        'Origin': GOURMET_ORIGIN,
        'Referer': this.lastPageUrl || url,
      },
    });
    return response.data;
  }

  /** Reset client (for logout - clears all cookies via native layer) */
  resetClient(): void {
    this.lastPageUrl = '';
  }
}

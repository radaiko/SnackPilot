import { VentopayHttpClient } from './ventopayClient';
import {
  extractAspNetState,
  isVentopayLoggedIn,
  parseTransactions,
} from './ventopayParser';
import { VentopayTransaction } from '../types/ventopay';
import {
  VENTOPAY_LOGIN_URL,
  VENTOPAY_TRANSACTIONS_URL,
  VENTOPAY_LOGOUT_URL,
  VENTOPAY_COMPANY_ID,
} from '../utils/constants';

export class VentopayApi {
  private client: VentopayHttpClient;
  private credentials: { username: string; password: string } | null = null;
  private loggedIn = false;

  constructor() {
    this.client = new VentopayHttpClient();
  }

  /**
   * Login to the Ventopay system.
   *
   * CRITICAL: This follows the exact ASP.NET login sequence from CLAUDE.md:
   * 1. GET Login.aspx to load form and extract ASP.NET state
   * 2. POST Login.aspx with company ID, credentials, and ASP.NET state
   * 3. Verify login by checking for Ausloggen.aspx link
   */
  async login(username: string, password: string): Promise<void> {
    // Step 1: GET login page to extract ASP.NET state
    const loginPageHtml = await this.client.get(VENTOPAY_LOGIN_URL);

    const state = extractAspNetState(loginPageHtml);

    // Step 2: POST login form with ALL required fields in exact browser order
    const responseHtml = await this.client.postForm(VENTOPAY_LOGIN_URL, {
      __LASTFOCUS: state.lastFocus,
      __EVENTTARGET: state.eventTarget,
      __EVENTARGUMENT: state.eventArgument,
      __VIEWSTATE: state.viewState,
      __VIEWSTATEGENERATOR: state.viewStateGenerator,
      __EVENTVALIDATION: state.eventValidation,
      DropDownList1: VENTOPAY_COMPANY_ID,
      TxtUsername: username,
      TxtPassword: password,
      BtnLogin: 'Login',
      languageRadio: 'DE',
    });

    // Step 3: Verify login success
    const loggedIn = isVentopayLoggedIn(responseHtml);

    if (!loggedIn) {
      throw new Error('Ventopay login failed: invalid credentials or account blocked');
    }

    this.credentials = { username, password };
    this.loggedIn = true;
  }

  /**
   * Ensure we have an active session. Re-login if needed.
   */
  private async ensureSession(): Promise<void> {
    if (this.loggedIn) return;

    if (this.credentials) {
      await this.login(this.credentials.username, this.credentials.password);
      return;
    }

    throw new Error('Ventopay session expired and no credentials saved');
  }

  /**
   * Fetch transactions for a date range.
   *
   * @param fromDate Start date
   * @param toDate End date
   * @returns Parsed transactions (Gourmet transactions filtered out)
   */
  async getTransactions(fromDate: Date, toDate: Date): Promise<VentopayTransaction[]> {
    await this.ensureSession();

    const fromStr = formatVentopayDate(fromDate);
    const toStr = formatVentopayDate(toDate);

    const html = await this.client.get(VENTOPAY_TRANSACTIONS_URL, {
      fromDate: fromStr,
      untilDate: toStr,
    });

    // Check if session expired during fetch
    if (!isVentopayLoggedIn(html)) {
      this.loggedIn = false;
      await this.ensureSession();
      const retryHtml = await this.client.get(VENTOPAY_TRANSACTIONS_URL, {
        fromDate: fromStr,
        untilDate: toStr,
      });
      return parseTransactions(retryHtml);
    }

    return parseTransactions(html);
  }

  /**
   * Logout from the Ventopay system.
   */
  async logout(): Promise<void> {
    try {
      await this.client.get(VENTOPAY_LOGOUT_URL);
    } catch {
      // Logout failure is non-critical
    } finally {
      this.loggedIn = false;
      this.credentials = null;
      this.client.resetClient();
    }
  }

  /** Check if we have an active session */
  isAuthenticated(): boolean {
    return this.loggedIn;
  }
}

/** Format date as dd.MM.yyyy (Ventopay system format) */
function formatVentopayDate(date: Date): string {
  const day = String(date.getDate()).padStart(2, '0');
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const year = date.getFullYear();
  return `${day}.${month}.${year}`;
}

import * as cheerio from 'cheerio';
import { VentopayTransaction } from '../types/ventopay';

/** ASP.NET hidden form state fields (all fields the browser submits) */
export interface AspNetState {
  lastFocus: string;
  eventTarget: string;
  eventArgument: string;
  viewState: string;
  viewStateGenerator: string;
  eventValidation: string;
}

/**
 * Extract ASP.NET hidden form state from a page.
 *
 * Extracts all 6 ASP.NET hidden fields. __LASTFOCUS, __EVENTTARGET,
 * and __EVENTARGUMENT are typically empty but must be included in POST.
 */
export function extractAspNetState(html: string): AspNetState {
  const $ = cheerio.load(html);

  const viewState = $('#__VIEWSTATE').attr('value');
  const viewStateGenerator = $('#__VIEWSTATEGENERATOR').attr('value');
  const eventValidation = $('#__EVENTVALIDATION').attr('value');

  if (!viewState || !viewStateGenerator || !eventValidation) {
    throw new Error('Could not extract ASP.NET state from page');
  }

  return {
    lastFocus: $('#__LASTFOCUS').attr('value') ?? '',
    eventTarget: $('#__EVENTTARGET').attr('value') ?? '',
    eventArgument: $('#__EVENTARGUMENT').attr('value') ?? '',
    viewState,
    viewStateGenerator,
    eventValidation,
  };
}

/**
 * Check if a Ventopay page is from an authenticated session.
 * Checks for the logout link.
 */
export function isVentopayLoggedIn(html: string): boolean {
  return /href="Ausloggen\.aspx"/i.test(html);
}

/** German month name to 0-based index */
const GERMAN_MONTHS: Record<string, number> = {
  'jan': 0, 'jän': 0, 'feb': 1, 'mär': 2, 'mar': 2, 'mrz': 2, 'apr': 3,
  'mai': 4, 'jun': 5, 'jul': 6, 'aug': 7, 'sep': 8, 'okt': 9,
  'nov': 10, 'dez': 11,
};

/**
 * Parse the Ventopay timestamp format.
 * Observed format: "09. Feb 2026 - 11:49 Uhr"
 */
function parseVentopayTimestamp(text: string): Date {
  const trimmed = text.trim();

  // "09. Feb 2026 - 11:49 Uhr" or "03. Mrz 2026 - 11:40 Uhr"
  // Use \p{L} (Unicode letter) instead of \w to match German umlauts (ä, ö, ü)
  const match = trimmed.match(/(\d{1,2})\.\s*(\p{L}{3})\p{L}*\s+(\d{4})\s*-\s*(\d{1,2}):(\d{2})/u);
  if (match) {
    const day = parseInt(match[1], 10);
    const monthStr = match[2].toLowerCase();
    const year = parseInt(match[3], 10);
    const hours = parseInt(match[4], 10);
    const minutes = parseInt(match[5], 10);
    const month = GERMAN_MONTHS[monthStr] ?? 0;
    return new Date(year, month, day, hours, minutes);
  }

  return new Date(trimmed);
}

/**
 * Parse German currency format ("€ 1,80" or "1,80") to number.
 */
function parseGermanCurrency(text: string): number {
  const cleaned = text.replace(/[^\d,\-]/g, '').replace(',', '.');
  return parseFloat(cleaned) || 0;
}

/**
 * Parse the transact_title text.
 * Format: "€ 1,80 (Café + Co. Automaten)"
 * Returns { amount, restaurant }.
 */
function parseTransactTitle(text: string): { amount: number; restaurant: string } {
  const trimmed = text.trim();

  // Match "€ 1,80 (Restaurant Name)"
  const match = trimmed.match(/€\s*([\d,]+)\s*\((.+)\)/);
  if (match) {
    return {
      amount: parseGermanCurrency(match[1]),
      restaurant: match[2].trim(),
    };
  }

  // Fallback: try to extract any amount
  return {
    amount: parseGermanCurrency(trimmed),
    restaurant: trimmed,
  };
}

/**
 * Parse transactions from the Ventopay transactions page.
 *
 * HTML structure per transaction:
 *   <div class="transact" id="{base64Id}">
 *     <a href="rechnung.aspx?id=...">
 *       <div class="transact_title">€ 1,80 (Café + Co. Automaten)</div>
 *       <div class="transact_timestamp">09. Feb 2026 - 11:49 Uhr</div>
 *     </a>
 *   </div>
 *
 * Filters out transactions where restaurant contains "Gourmet"
 * (already covered by the Gourmet billing system).
 */
export function parseTransactions(html: string): VentopayTransaction[] {
  const $ = cheerio.load(html);
  const transactions: VentopayTransaction[] = [];

  $('div.transact').each((_, el) => {
    const transact = $(el);

    const id = transact.attr('id') || '';
    if (!id) return;

    // Extract from actual DOM classes
    const titleText = transact.find('.transact_title').text().trim();
    const timestampText = transact.find('.transact_timestamp').text().trim();

    if (!titleText) return;

    const { amount, restaurant } = parseTransactTitle(titleText);

    // Filter out Gourmet transactions (already covered by Gourmet billing)
    if (restaurant.toLowerCase().includes('gourmet')) return;

    const date = timestampText ? parseVentopayTimestamp(timestampText) : new Date();

    transactions.push({ id, date, amount, restaurant, location: restaurant });
  });

  return transactions;
}

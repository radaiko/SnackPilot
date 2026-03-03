import * as fs from 'fs';
import * as path from 'path';
import {
  extractAspNetState,
  isVentopayLoggedIn,
  parseTransactions,
} from '../../api/ventopayParser';

const fixturesDir = path.join(__dirname, '..', 'fixtures');
const loadFixture = (filepath: string) =>
  fs.readFileSync(path.join(fixturesDir, filepath), 'utf-8');

const loginPageHtml = loadFixture('ventopay/login-page.html');
const loginSuccessHtml = loadFixture('ventopay/login-success.html');
const transactionsPageHtml = loadFixture('ventopay/transactions-page.html');
const transactionsEmptyHtml = loadFixture('ventopay/transactions-empty.html');

describe('extractAspNetState', () => {
  it('extracts viewState from login page', () => {
    const state = extractAspNetState(loginPageHtml);
    expect(state.viewState).toBe('VIEWSTATE-TOKEN-LONG-BASE64-STRING-ABC123');
  });

  it('extracts viewStateGenerator', () => {
    const state = extractAspNetState(loginPageHtml);
    expect(state.viewStateGenerator).toBe('ABCD1234');
  });

  it('extracts eventValidation', () => {
    const state = extractAspNetState(loginPageHtml);
    expect(state.eventValidation).toBe('EVENTVALIDATION-TOKEN-XYZ789');
  });

  it('lastFocus, eventTarget, eventArgument are empty strings', () => {
    const state = extractAspNetState(loginPageHtml);
    expect(state.lastFocus).toBe('');
    expect(state.eventTarget).toBe('');
    expect(state.eventArgument).toBe('');
  });

  it('throws for page without ASP.NET state', () => {
    expect(() => extractAspNetState('<html><body></body></html>')).toThrow(
      /Could not extract ASP\.NET state/
    );
  });
});

describe('isVentopayLoggedIn', () => {
  it('returns true for login-success.html', () => {
    expect(isVentopayLoggedIn(loginSuccessHtml)).toBe(true);
  });

  it('returns false for login-page.html', () => {
    expect(isVentopayLoggedIn(loginPageHtml)).toBe(false);
  });
});

describe('parseTransactions', () => {
  it('filters out Gourmet transactions (5 returned from 6 in fixture)', () => {
    const transactions = parseTransactions(transactionsPageHtml);
    expect(transactions).toHaveLength(5);
  });

  it('first transaction has correct id, amount, restaurant', () => {
    const transactions = parseTransactions(transactionsPageHtml);
    const first = transactions[0];
    expect(first.id).toBe('dHhuLTAwMQ==');
    expect(first.amount).toBe(1.8);
    expect(first.restaurant).toBe('Café + Co. Automaten');
  });

  it('parses German currency correctly', () => {
    const transactions = parseTransactions(transactionsPageHtml);
    expect(transactions[0].amount).toBe(1.8);
    expect(transactions[1].amount).toBe(3.2);
    expect(transactions[2].amount).toBe(0.5);
    expect(transactions[3].amount).toBe(2.4);
    expect(transactions[4].amount).toBe(1.5);
  });

  it('parses timestamp correctly', () => {
    const transactions = parseTransactions(transactionsPageHtml);
    const date = transactions[0].date;
    expect(date.getFullYear()).toBe(2026);
    expect(date.getMonth()).toBe(1); // February (0-indexed)
    expect(date.getDate()).toBe(9);
    expect(date.getHours()).toBe(11);
    expect(date.getMinutes()).toBe(49);
  });

  it('parses "Mrz" month abbreviation correctly (March)', () => {
    const transactions = parseTransactions(transactionsPageHtml);
    const mrzTransaction = transactions[3]; // "03. Mrz 2026"
    expect(mrzTransaction.date.getFullYear()).toBe(2026);
    expect(mrzTransaction.date.getMonth()).toBe(2); // March (0-indexed)
    expect(mrzTransaction.date.getDate()).toBe(3);
  });

  it('parses "Jän" month abbreviation with umlaut correctly (January)', () => {
    const transactions = parseTransactions(transactionsPageHtml);
    const jänTransaction = transactions[4]; // "15. Jän 2026"
    expect(jänTransaction.date.getFullYear()).toBe(2026);
    expect(jänTransaction.date.getMonth()).toBe(0); // January (0-indexed)
    expect(jänTransaction.date.getDate()).toBe(15);
  });

  it('returns empty array for empty transactions page', () => {
    const transactions = parseTransactions(transactionsEmptyHtml);
    expect(transactions).toEqual([]);
  });
});

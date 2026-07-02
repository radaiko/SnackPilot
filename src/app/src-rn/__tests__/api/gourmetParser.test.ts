import * as fs from 'fs';
import * as path from 'path';
import {
  extractFormTokens,
  extractLoginFormTokens,
  isLoggedIn,
  extractUserInfo,
  parseMenuItems,
  hasNextMenuPage,
  parseOrderedMenus,
  extractEditModeFormData,
  extractCancelOrderFormData,
  extractLogoutFormTokens,
} from '../../api/gourmetParser';
import { GourmetMenuCategory } from '../../types/menu';

const fixturesDir = path.join(__dirname, '..', 'fixtures');
const loadFixture = (filepath: string) =>
  fs.readFileSync(path.join(fixturesDir, filepath), 'utf-8');

const loginPageHtml = loadFixture('gourmet/login-page.html');
const loginSuccessHtml = loadFixture('gourmet/login-success.html');
const loginFailedHtml = loadFixture('gourmet/login-failed.html');
const menusPage0Html = loadFixture('gourmet/menus-page-0.html');
const menusPage1Html = loadFixture('gourmet/menus-page-1.html');
const ordersPageHtml = loadFixture('gourmet/orders-page.html');
const ordersPageEditModeHtml = loadFixture('gourmet/orders-page-edit-mode.html');

describe('extractLoginFormTokens / extractFormTokens', () => {
  it('extracts ufprt and ncforminfo from login page', () => {
    const tokens = extractLoginFormTokens(loginPageHtml);
    expect(tokens).toHaveProperty('ufprt');
    expect(tokens).toHaveProperty('ncforminfo');
  });

  it('returns correct token values', () => {
    const tokens = extractLoginFormTokens(loginPageHtml);
    expect(tokens.ufprt).toBe('CSRF-TOKEN-LOGIN-ABC123');
    expect(tokens.ncforminfo).toBe('NCFORM-TOKEN-LOGIN-XYZ789');
  });

  it('throws if ufprt is missing', () => {
    expect(() => extractFormTokens('<html><form></form></html>', 'form')).toThrow(
      /ufprt/
    );
  });

  it('throws if __ncforminfo is missing', () => {
    const htmlWithUfprtOnly =
      '<html><form><input name="ufprt" value="x" /></form></html>';
    expect(() => extractFormTokens(htmlWithUfprtOnly, 'form')).toThrow(
      /__ncforminfo/
    );
  });
});

describe('isLoggedIn', () => {
  it('returns true for login-success.html (has /einstellungen/ link)', () => {
    expect(isLoggedIn(loginSuccessHtml)).toBe(true);
  });

  it('returns false for login-page.html', () => {
    expect(isLoggedIn(loginPageHtml)).toBe(false);
  });

  it('returns false for login-failed.html', () => {
    expect(isLoggedIn(loginFailedHtml)).toBe(false);
  });
});

describe('extractUserInfo', () => {
  it('extracts all 4 fields from login-success.html', () => {
    const info = extractUserInfo(loginSuccessHtml);
    expect(info).toHaveProperty('username');
    expect(info).toHaveProperty('shopModelId');
    expect(info).toHaveProperty('eaterId');
    expect(info).toHaveProperty('staffGroupId');
  });

  it('username is "TestUser"', () => {
    const info = extractUserInfo(loginSuccessHtml);
    expect(info.username).toBe('TestUser');
  });

  it('shopModelId is "SM-TEST-123"', () => {
    const info = extractUserInfo(loginSuccessHtml);
    expect(info.shopModelId).toBe('SM-TEST-123');
  });

  it('eaterId is "EATER-TEST-456"', () => {
    const info = extractUserInfo(loginSuccessHtml);
    expect(info.eaterId).toBe('EATER-TEST-456');
  });

  it('staffGroupId is "SG-TEST-789"', () => {
    const info = extractUserInfo(loginSuccessHtml);
    expect(info.staffGroupId).toBe('SG-TEST-789');
  });

  it('throws for page without user info', () => {
    expect(() => extractUserInfo('<html><body></body></html>')).toThrow(
      /Could not extract user info/
    );
  });
});

describe('parseMenuItems', () => {
  const items = parseMenuItems(menusPage0Html);

  it('parses correct number of items from menus-page-0 (7 items from desktop layout)', () => {
    expect(items).toHaveLength(7);
  });

  it('first item has correct id, title, subtitle, allergens, category', () => {
    const first = items[0];
    expect(first.id).toBe('menu-001');
    expect(first.title).toBe('MENÜ I');
    expect(first.subtitle).toBe('Wiener Schnitzel mit Kartoffelsalat');
    expect(first.allergens).toEqual(['A', 'C', 'G']);
    expect(first.category).toBe(GourmetMenuCategory.Menu1);
  });

  it('MENU I category detection works', () => {
    const menu1Items = items.filter(
      (i) => i.category === GourmetMenuCategory.Menu1
    );
    expect(menu1Items.length).toBeGreaterThanOrEqual(1);
    expect(menu1Items[0].title).toBe('MENÜ I');
  });

  it('MENU II category detection works', () => {
    const menu2Items = items.filter(
      (i) => i.category === GourmetMenuCategory.Menu2
    );
    expect(menu2Items.length).toBeGreaterThanOrEqual(1);
    expect(menu2Items[0].title).toBe('MENÜ II');
  });

  it('MENU III category detection works', () => {
    const menu3Items = items.filter(
      (i) => i.category === GourmetMenuCategory.Menu3
    );
    expect(menu3Items.length).toBeGreaterThanOrEqual(1);
    expect(menu3Items[0].title).toBe('MENÜ III');
  });

  it('SUPPE & SALAT category detection works', () => {
    const soupItems = items.filter(
      (i) => i.category === GourmetMenuCategory.SoupAndSalad
    );
    expect(soupItems.length).toBeGreaterThanOrEqual(1);
    expect(soupItems[0].title).toBe('SUPPE & SALAT');
  });

  it('available items have available=true', () => {
    // First item (MENU I, Mon) has a checkbox
    expect(items[0].available).toBe(true);
  });

  it('unavailable items (no checkbox) have available=false', () => {
    // Tue MENU II (index 5) has no checkbox
    const unavailableItem = items.find(
      (i) => i.subtitle === 'Gebratener Lachs mit Dillsauce'
    );
    expect(unavailableItem).toBeDefined();
    expect(unavailableItem!.available).toBe(false);
  });

  it('ordered items (checked checkbox) have ordered=true', () => {
    // Tue MENU I (index 4) is checked
    const orderedItem = items.find(
      (i) => i.subtitle === 'Schweinsbraten mit Knödel'
    );
    expect(orderedItem).toBeDefined();
    expect(orderedItem!.ordered).toBe(true);
  });

  it('empty allergens parsed as empty array', () => {
    // Last item (MENU III, Tue) has empty allergen
    const noAllergenItem = items.find(
      (i) => i.subtitle === 'Reis mit Gemüse'
    );
    expect(noAllergenItem).toBeDefined();
    expect(noAllergenItem!.allergens).toEqual([]);
  });

  it('parses price correctly', () => {
    expect(items[0].price).toBe('€ 5,50');
  });
});

describe('hasNextMenuPage', () => {
  it('returns true for menus-page-0', () => {
    expect(hasNextMenuPage(menusPage0Html)).toBe(true);
  });

  it('returns false for menus-page-1', () => {
    expect(hasNextMenuPage(menusPage1Html)).toBe(false);
  });
});

describe('parseOrderedMenus', () => {
  const orders = parseOrderedMenus(ordersPageHtml);

  it('parses correct number of orders from orders-page (3)', () => {
    expect(orders).toHaveLength(3);
  });

  it('first order has correct positionId and eatingCycleId', () => {
    expect(orders[0].positionId).toBe('POS-001');
    expect(orders[0].eatingCycleId).toBe('EC-001');
  });

  it('confirmed orders (fa-check) have approved=true', () => {
    const pos001 = orders.find((o) => o.positionId === 'POS-001');
    expect(pos001!.approved).toBe(true);
  });

  it('unconfirmed orders have approved=false', () => {
    const pos002 = orders.find((o) => o.positionId === 'POS-002');
    expect(pos002!.approved).toBe(false);
  });

  it('checkmark class also marks approved=true', () => {
    const pos003 = orders.find((o) => o.positionId === 'POS-003');
    expect(pos003!.approved).toBe(true);
  });
});

describe('extractEditModeFormData', () => {
  it('extracts editMode="True" from orders-page (not in edit mode)', () => {
    const data = extractEditModeFormData(ordersPageHtml);
    expect(data.editMode).toBe('True');
  });

  it('extracts editMode="False" from orders-page-edit-mode (in edit mode)', () => {
    const data = extractEditModeFormData(ordersPageEditModeHtml);
    expect(data.editMode).toBe('False');
  });

  it('extracts ufprt and ncforminfo', () => {
    const data = extractEditModeFormData(ordersPageHtml);
    expect(data.ufprt).toBe('CSRF-TOKEN-EDITMODE-555');
    expect(data.ncforminfo).toBe('NCFORM-TOKEN-EDITMODE-666');
  });
});

describe('extractCancelOrderFormData', () => {
  it('extracts cancel data for POS-001 from edit mode page', () => {
    const data = extractCancelOrderFormData(ordersPageEditModeHtml, 'POS-001');
    expect(data.positionId).toBe('POS-001');
    expect(data.ufprt).toBe('CSRF-TOKEN-CANCEL-POS001-AAA');
    expect(data.ncforminfo).toBe('NCFORM-TOKEN-CANCEL-POS001-BBB');
    expect(data.eatingCycleId).toBe('EC-001');
  });

  it('extracts cancel data for POS-002 from edit mode page', () => {
    const data = extractCancelOrderFormData(ordersPageEditModeHtml, 'POS-002');
    expect(data.positionId).toBe('POS-002');
    expect(data.ufprt).toBe('CSRF-TOKEN-CANCEL-POS002-CCC');
    expect(data.ncforminfo).toBe('NCFORM-TOKEN-CANCEL-POS002-DDD');
    expect(data.eatingCycleId).toBe('EC-002');
  });

  it('contains correct ufprt and ncforminfo', () => {
    const data = extractCancelOrderFormData(ordersPageEditModeHtml, 'POS-001');
    expect(data.ufprt).toBeTruthy();
    expect(data.ncforminfo).toBeTruthy();
  });

  it('throws when __ncforminfo is missing on cancel form', () => {
    const htmlMissingNcforminfo = `
      <html>
        <body>
          <form id="form_POS-001_cp">
            <input name="cp_PositionId" value="POS-001" />
            <input name="cp_EatingCycleId_POS-001" value="EC-001" />
            <input name="cp_Date_POS-001" value="10.02.2026 00:00:00" />
            <input name="ufprt" value="CSRF-CANCEL-1" />
          </form>
        </body>
      </html>
    `;
    expect(() => extractCancelOrderFormData(htmlMissingNcforminfo, 'POS-001')).toThrow(
      /Could not extract cancel form data/
    );
  });
});

describe('extractLogoutFormTokens', () => {
  it('extracts tokens from login-success.html', () => {
    const tokens = extractLogoutFormTokens(loginSuccessHtml);
    expect(tokens.ufprt).toBe('CSRF-TOKEN-LOGOUT-DEF456');
    expect(tokens.ncforminfo).toBe('NCFORM-TOKEN-LOGOUT-UVW012');
  });

  it('throws for page without logout form', () => {
    expect(() => extractLogoutFormTokens('<html><body></body></html>')).toThrow(
      /Could not find logout form/
    );
  });
});

describe('parser edge cases', () => {
  it('detects Unknown category for titles that match no known pattern', () => {
    const html = `
      <div class="row hide-sm-down">
        <div class="meal">
          <a class="open_info menu-article-detail" data-id="m-x" data-date="02-10-2026"></a>
          <div class="title">WOCHENANGEBOT<div class="subtitle">Special</div></div>
        </div>
      </div>`;

    const items = parseMenuItems(html);

    expect(items).toHaveLength(1);
    expect(items[0].category).toBe(GourmetMenuCategory.Unknown);
  });

  it('extractEditModeFormData throws when the form is missing its tokens', () => {
    const html = '<form class="form-toggleEditMode"><input name="editMode" value="True" /></form>';
    expect(() => extractEditModeFormData(html)).toThrow('Could not extract edit mode form data');
  });

  it('extractCancelOrderFormData falls back to matching by position input', () => {
    const html = `
      <form id="someOtherId">
        <input name="cp_PositionId" value="POS-77" />
        <input name="cp_EatingCycleId_POS-77" value="EC-77" />
        <input name="cp_Date_POS-77" value="10.02.2026 00:00:00" />
        <input name="ufprt" value="UFPRT-77" />
        <input name="__ncforminfo" value="NC-77" />
      </form>`;

    const data = extractCancelOrderFormData(html, 'POS-77');

    expect(data).toEqual({
      positionId: 'POS-77',
      eatingCycleId: 'EC-77',
      date: '10.02.2026 00:00:00',
      ufprt: 'UFPRT-77',
      ncforminfo: 'NC-77',
    });
  });

  it('extractCancelOrderFormData throws when tokens are missing', () => {
    const html = '<form><input name="cp_PositionId" value="POS-88" /></form>';
    expect(() => extractCancelOrderFormData(html, 'POS-88')).toThrow(
      'Could not extract cancel form data for position: POS-88'
    );
  });

  it('extractLogoutFormTokens throws when the logout form lacks tokens', () => {
    const html = '<form><button id="btnHeaderLogout">Logout</button></form>';
    expect(() => extractLogoutFormTokens(html)).toThrow('Could not extract logout form tokens');
  });
});

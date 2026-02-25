import * as cheerio from 'cheerio';
import { GourmetMenuItem, GourmetMenuCategory, GourmetUserInfo } from '../types/menu';
import { GourmetOrderedMenu } from '../types/order';
import { parseGourmetDate, parseGourmetOrderDate } from '../utils/dateUtils';
import { FormTokens, EditModeFormData, CancelOrderFormData } from './types';

const MENU_CATEGORY_REGEX = /MEN(?:Ü|U)\s+([I]{1,3})/i;
const SOUP_SALAD_PATTERN = 'SUPPE & SALAT';

/**
 * Extract CSRF tokens (ufprt + __ncforminfo) from a form.
 * Both fields MUST be sent with every form POST to avoid account bans.
 */
export function extractFormTokens(html: string, formSelector: string): FormTokens {
  const $ = cheerio.load(html);
  const form = $(formSelector).first();

  const ufprt = form.find('input[name="ufprt"]').attr('value');
  const ncforminfo = form.find('input[name="__ncforminfo"]').attr('value');

  if (!ufprt) {
    throw new Error(`Could not find ufprt in form: ${formSelector}`);
  }
  if (!ncforminfo) {
    throw new Error(`Could not find __ncforminfo in form: ${formSelector}`);
  }

  return { ufprt, ncforminfo };
}

/**
 * Extract login form tokens from the start page.
 * The login form is the first form on the page.
 */
export function extractLoginFormTokens(html: string): FormTokens {
  return extractFormTokens(html, 'form:first-of-type');
}

/**
 * Check if the page is from an authenticated session.
 * Checks multiple indicators since different pages use different layouts.
 */
export function isLoggedIn(html: string): boolean {
  return (
    html.includes('/einstellungen/') ||   // settings link (absolute or relative)
    html.includes('btnHeaderLogout') ||    // logout button
    html.includes('class="loginname"') ||  // username display
    html.includes('id="eater"')            // eater hidden input (menus/orders pages)
  );
}

/**
 * Extract user info (shopModelId, eaterId, staffGroupId, username) from a page.
 */
export function extractUserInfo(html: string): GourmetUserInfo {
  const $ = cheerio.load(html);

  const shopModelId = $('#shopModel').attr('value');
  const eaterId = $('#eater').attr('value');
  const staffGroupId = $('#staffGroup').attr('value');
  const username = $('span.loginname').text().trim();

  if (!shopModelId || !eaterId || !staffGroupId) {
    throw new Error('Could not extract user info from page');
  }

  return { username, shopModelId, eaterId, staffGroupId };
}

/**
 * Detect menu category from the title text.
 */
function detectCategory(title: string): GourmetMenuCategory {
  if (title.includes(SOUP_SALAD_PATTERN)) {
    return GourmetMenuCategory.SoupAndSalad;
  }

  const match = title.match(MENU_CATEGORY_REGEX);
  if (match) {
    const romanNumeral = match[1];
    switch (romanNumeral.length) {
      case 1: return GourmetMenuCategory.Menu1;
      case 2: return GourmetMenuCategory.Menu2;
      case 3: return GourmetMenuCategory.Menu3;
    }
  }

  return GourmetMenuCategory.Unknown;
}

/**
 * Parse menu items from a menus page HTML.
 */
export function parseMenuItems(html: string): GourmetMenuItem[] {
  const $ = cheerio.load(html);
  const items: GourmetMenuItem[] = [];

  // Only parse desktop layout (hide-sm-down) to avoid duplicates
  $('div.row.hide-sm-down .meal').each((_, el) => {
    const meal = $(el);
    const openInfo = meal.find('.open_info.menu-article-detail');

    const id = openInfo.attr('data-id');
    const dateStr = openInfo.attr('data-date');
    if (!id || !dateStr) return;

    const titleEl = meal.find('.title');
    // Title is the direct text node (category), subtitle is nested div
    const titleText = titleEl.contents().filter(function () {
      return this.type === 'text';
    }).text().trim();
    const subtitle = meal.find('.subtitle').text().trim();

    // Allergens: single <li class="allergen"> with comma-separated letters
    const allergenText = meal.find('li.allergen').text().trim();
    const allergens = allergenText
      ? allergenText.split(',').map((a) => a.trim()).filter(Boolean)
      : [];

    // Availability: determined by presence of checkbox
    const checkbox = meal.find('input[type="checkbox"].menu-clicked');
    const hasCheckbox = checkbox.length > 0;

    // Already ordered: checkbox exists and is checked
    const isOrdered = hasCheckbox && (checkbox.attr('checked') !== undefined || checkbox.is(':checked'));

    // Price
    const price = meal.find('.price span').text().trim();

    items.push({
      id,
      day: parseGourmetDate(dateStr),
      title: titleText,
      subtitle,
      allergens,
      available: hasCheckbox,
      ordered: isOrdered,
      category: detectCategory(titleText),
      price: price || '',
    });
  });

  return items;
}

/**
 * Check if there is a next page of menus.
 */
export function hasNextMenuPage(html: string): boolean {
  const $ = cheerio.load(html);
  return $('a[class*="menues-next"]').length > 0;
}

/**
 * Parse ordered menus from the orders page HTML.
 */
export function parseOrderedMenus(html: string): GourmetOrderedMenu[] {
  const $ = cheerio.load(html);
  const orders: GourmetOrderedMenu[] = [];

  $('div.order-item, div[class*="order-item"]').each((_, el) => {
    const item = $(el);

    const positionId = item.find('input[name="cp_PositionId"]').attr('value');
    if (!positionId) return;

    const eatingCycleInput = item.find(`input[name^="cp_EatingCycleId_"]`);
    const eatingCycleId = eatingCycleInput.attr('value') || '';

    const dateInput = item.find(`input[name^="cp_Date_"]`);
    const dateStr = dateInput.attr('value') || '';

    const titleEl = item.find('.title');
    // Title is the direct text (category name), subtitle is nested div
    const title = titleEl.contents().filter(function () {
      return this.type === 'text';
    }).text().trim();
    const subtitle = item.find('.subtitle').text().trim();

    // Check if order is approved/confirmed
    // Confirmed: has check icon or checkmark span (no .confirmed class exists on the site)
    const hasCheckIcon = item.find('.fa-check').length > 0;
    const hasCheckmark = item.find('.checkmark').length > 0;
    const approved = hasCheckIcon || hasCheckmark;

    orders.push({
      positionId,
      eatingCycleId,
      date: dateStr ? parseGourmetOrderDate(dateStr) : new Date(),
      title,
      subtitle,
      approved,
    });
  });

  return orders;
}

/**
 * Extract edit mode form data from the orders page.
 */
export function extractEditModeFormData(html: string): EditModeFormData {
  const $ = cheerio.load(html);
  const form = $('form.form-toggleEditMode');

  const editMode = form.find('input[name="editMode"]').attr('value') || 'True';
  const ufprt = form.find('input[name="ufprt"]').attr('value');
  const ncforminfo = form.find('input[name="__ncforminfo"]').attr('value');

  if (!ufprt || !ncforminfo) {
    throw new Error('Could not extract edit mode form data');
  }

  return { editMode, ufprt, ncforminfo };
}

/**
 * Extract cancel order form data for a specific order.
 * Uses the form ID pattern from the C# implementation: form_{positionId}_cp
 */
export function extractCancelOrderFormData(
  html: string,
  positionId: string
): CancelOrderFormData {
  const $ = cheerio.load(html);
  // C# uses: form[@id='form_{positionId}_cp']
  let form = $(`form#form_${positionId}_cp`);
  if (form.length === 0) {
    // Fallback: find form containing the position input
    form = $(`form:has(input[name="cp_PositionId"][value="${positionId}"])`);
  }

  const eatingCycleInput = form.find(`input[name^="cp_EatingCycleId_"]`);
  const dateInput = form.find(`input[name^="cp_Date_"]`);

  const ufprt = form.find('input[name="ufprt"]').attr('value');
  const ncforminfo = form.find('input[name="__ncforminfo"]').attr('value');

  if (!ufprt || !ncforminfo) {
    throw new Error(`Could not extract cancel form data for position: ${positionId}`);
  }

  return {
    positionId,
    eatingCycleId: eatingCycleInput.attr('value') || '',
    date: dateInput.attr('value') || '',
    ufprt,
    ncforminfo,
  };
}

/**
 * Extract logout form tokens from any authenticated page.
 */
export function extractLogoutFormTokens(html: string): FormTokens {
  const $ = cheerio.load(html);
  // The logout form contains the logout button
  const form = $('form:has(button#btnHeaderLogout), form:has(button:contains("Logout"))');

  if (form.length === 0) {
    throw new Error('Could not find logout form');
  }

  const ufprt = form.find('input[name="ufprt"]').attr('value');
  const ncforminfo = form.find('input[name="__ncforminfo"]').attr('value');

  if (!ufprt || !ncforminfo) {
    throw new Error('Could not extract logout form tokens');
  }

  return { ufprt, ncforminfo };
}

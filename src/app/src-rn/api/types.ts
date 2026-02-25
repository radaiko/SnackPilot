/** Request body for AddToMenuesCart API */
export interface AddToCartRequest {
  eaterId: string;
  shopModelId: string;
  staffgroupId: string;
  dates: AddToCartDate[];
}

export interface AddToCartDate {
  date: string; // MM-dd-yyyy
  menuIds: string[];
}

/** Request body for GetMyBillings API */
export interface GetBillingsRequest {
  eaterId: string;
  shopModelId: string;
  checkLastMonthNumber: string; // "0" = current month
}

/** Raw billing API response item (from server JSON) */
export interface BillingApiItem {
  Id: string;
  ArticleId: string;
  Count: number;
  Description: string;
  Total: number;
  Subsidy: number;
  DiscountValue: number;
  IsCustomMenu: boolean;
}

/** Raw billing API response bill (from server JSON) */
export interface BillingApiBill {
  BillNr: number;
  BillDate: string; // ISO date string
  Location: string;
  BillingItemInfo: BillingApiItem[];
  Billing: number;
}

/** Form data extracted from a page (CSRF tokens) */
export interface FormTokens {
  ufprt: string;
  ncforminfo: string;
}

/** Edit mode form data on orders page */
export interface EditModeFormData extends FormTokens {
  editMode: string;
}

/** Cancel order form data */
export interface CancelOrderFormData {
  positionId: string;
  eatingCycleId: string;
  date: string;
  ufprt: string;
  ncforminfo: string;
}

/** Thrown when the server session has expired and re-login is needed */
export class SessionExpiredError extends Error {
  constructor() {
    super('Session expired');
    this.name = 'SessionExpiredError';
  }
}

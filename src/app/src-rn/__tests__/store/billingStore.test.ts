jest.mock('@react-native-async-storage/async-storage', () => {
  const store: Record<string, string> = {};
  return {
    __esModule: true,
    default: {
      getItem: jest.fn((key: string) => Promise.resolve(store[key] ?? null)),
      setItem: jest.fn((key: string, value: string) => { store[key] = value; return Promise.resolve(); }),
      removeItem: jest.fn((key: string) => { delete store[key]; return Promise.resolve(); }),
      clear: jest.fn(() => { Object.keys(store).forEach(k => delete store[k]); return Promise.resolve(); }),
    },
  };
});

jest.mock('../../api/gourmetApi');
jest.mock('../../api/ventopayApi');

jest.mock('../../store/authStore', () => {
  const mockApi = { getBillings: jest.fn() };
  return {
    useAuthStore: {
      getState: () => ({ api: mockApi }),
      setState: jest.fn(),
      subscribe: jest.fn(),
    },
  };
});

jest.mock('../../store/ventopayAuthStore', () => {
  const mockApi = { getTransactions: jest.fn() };
  const state = { api: mockApi, status: 'authenticated' };
  return {
    useVentopayAuthStore: {
      getState: () => state,
      setState: jest.fn(),
      subscribe: jest.fn(),
    },
  };
});

import { useBillingStore } from '../../store/billingStore';
import { useAuthStore } from '../../store/authStore';
import { useVentopayAuthStore } from '../../store/ventopayAuthStore';
import { GourmetBill } from '../../types/billing';
import { VentopayTransaction } from '../../types/ventopay';

const mockGourmetApi = (useAuthStore as any).getState().api;
const mockVentopayApi = (useVentopayAuthStore as any).getState().api;

function makeBill(overrides: Partial<GourmetBill> = {}): GourmetBill {
  return {
    billNr: 10001,
    billDate: new Date('2026-02-10'),
    location: 'Test Restaurant',
    items: [{
      id: '1',
      articleId: 'A1',
      count: 1,
      description: 'Menü I',
      total: 5.50,
      subsidy: 2.50,
      discountValue: 0,
      isCustomMenu: false,
    }],
    billing: 3.00,
    ...overrides,
  };
}

function makeTransaction(overrides: Partial<VentopayTransaction> = {}): VentopayTransaction {
  return {
    id: 'T1',
    date: new Date('2026-02-10'),
    amount: 4.50,
    restaurant: 'Cafeteria',
    location: 'Main Building',
    ...overrides,
  };
}

beforeEach(async () => {
  const AsyncStorage = require('@react-native-async-storage/async-storage').default;
  await AsyncStorage.clear();
  jest.clearAllMocks();
  (useVentopayAuthStore as any).getState().status = 'authenticated';
  useBillingStore.setState({
    gourmetMonths: {},
    ventopayMonths: {},
    selectedMonthIndex: 0,
    sourceFilter: 'all',
    loading: false,
    error: null,
  });
});

describe('billingStore', () => {
  describe('getMonthOptions', () => {
    it('returns 3 month options', () => {
      const options = useBillingStore.getState().getMonthOptions();
      expect(options).toHaveLength(3);
      expect(options[0].offset).toBe(0);
      expect(options[1].offset).toBe(1);
      expect(options[2].offset).toBe(2);
    });

    it('labels are German month names', () => {
      const options = useBillingStore.getState().getMonthOptions();
      const germanMonths = [
        'Jänner', 'Februar', 'März', 'April', 'Mai', 'Juni',
        'Juli', 'August', 'September', 'Oktober', 'November', 'Dezember',
      ];
      for (const opt of options) {
        const hasGermanMonth = germanMonths.some((m) => opt.label.includes(m));
        expect(hasGermanMonth).toBe(true);
      }
    });
  });

  describe('fetchBilling', () => {
    it('calls api.getBillings with correct month offset', async () => {
      mockGourmetApi.getBillings.mockResolvedValue([]);

      await useBillingStore.getState().fetchBilling(0);

      expect(mockGourmetApi.getBillings).toHaveBeenCalledWith('0');
    });

    it('stores billing data in gourmetMonths', async () => {
      const bills = [makeBill()];
      mockGourmetApi.getBillings.mockResolvedValue(bills);

      await useBillingStore.getState().fetchBilling(0);

      const options = useBillingStore.getState().getMonthOptions();
      const monthData = useBillingStore.getState().gourmetMonths[options[0].key];
      expect(monthData).toBeDefined();
      expect(monthData.bills).toEqual(bills);
      expect(monthData.totalBilling).toBe(3.00);
    });

    it('caches to AsyncStorage', async () => {
      const AsyncStorage = require('@react-native-async-storage/async-storage').default;
      const bills = [makeBill()];
      mockGourmetApi.getBillings.mockResolvedValue(bills);

      await useBillingStore.getState().fetchBilling(0);

      expect(AsyncStorage.setItem).toHaveBeenCalled();
      const [key] = AsyncStorage.setItem.mock.calls[0];
      expect(key).toContain('billing_');
    });

    it('sets error on failure', async () => {
      mockGourmetApi.getBillings.mockRejectedValue(new Error('Billing error'));

      await useBillingStore.getState().fetchBilling(0);

      expect(useBillingStore.getState().error).toBe('Billing error');
      expect(useBillingStore.getState().loading).toBe(false);
    });

    it('skips fetch for past months with existing data', async () => {
      const options = useBillingStore.getState().getMonthOptions();
      const pastKey = options[1].key;
      useBillingStore.setState({
        gourmetMonths: {
          [pastKey]: {
            monthKey: pastKey,
            label: options[1].label,
            bills: [makeBill()],
            totalGross: 5.50,
            totalSubsidy: 2.50,
            totalDiscount: 0,
            totalBilling: 3.00,
            fetchedAt: Date.now(),
          },
        },
      });

      await useBillingStore.getState().fetchBilling(1);

      expect(mockGourmetApi.getBillings).not.toHaveBeenCalled();
    });
  });

  describe('fetchVentopayBilling', () => {
    it('calls api.getTransactions with date range', async () => {
      mockVentopayApi.getTransactions.mockResolvedValue([]);

      await useBillingStore.getState().fetchVentopayBilling(0);

      expect(mockVentopayApi.getTransactions).toHaveBeenCalled();
      const [from, to] = mockVentopayApi.getTransactions.mock.calls[0];
      expect(from).toBeInstanceOf(Date);
      expect(to).toBeInstanceOf(Date);
      // from should be first day of month, to should be last day
      expect(from.getDate()).toBe(1);
    });

    it('stores transaction data in ventopayMonths', async () => {
      const transactions = [makeTransaction()];
      mockVentopayApi.getTransactions.mockResolvedValue(transactions);

      await useBillingStore.getState().fetchVentopayBilling(0);

      const options = useBillingStore.getState().getMonthOptions();
      const monthData = useBillingStore.getState().ventopayMonths[options[0].key];
      expect(monthData).toBeDefined();
      expect(monthData.transactions).toEqual(transactions);
      expect(monthData.total).toBe(4.50);
    });
  });

  describe('selectMonth / setSourceFilter', () => {
    it('updates selectedMonthIndex', () => {
      // Mock APIs to avoid actual fetch calls from selectMonth
      mockGourmetApi.getBillings.mockResolvedValue([]);
      mockVentopayApi.getTransactions.mockResolvedValue([]);

      useBillingStore.getState().selectMonth(2);
      expect(useBillingStore.getState().selectedMonthIndex).toBe(2);
    });

    it('triggers both fetches for the selected month', () => {
      mockGourmetApi.getBillings.mockResolvedValue([]);
      mockVentopayApi.getTransactions.mockResolvedValue([]);

      useBillingStore.getState().selectMonth(0);

      expect(mockGourmetApi.getBillings).toHaveBeenCalledWith('0');
      expect(mockVentopayApi.getTransactions).toHaveBeenCalled();
    });

    it('updates sourceFilter', () => {
      useBillingStore.getState().setSourceFilter('ventopay');
      expect(useBillingStore.getState().sourceFilter).toBe('ventopay');
    });
  });

  describe('selected month getters', () => {
    it('return null when no data exists for the selected month', () => {
      expect(useBillingStore.getState().getSelectedGourmetBilling()).toBeNull();
      expect(useBillingStore.getState().getSelectedVentopayBilling()).toBeNull();
    });

    it('return the data of the selected month', async () => {
      mockGourmetApi.getBillings.mockResolvedValue([makeBill()]);
      mockVentopayApi.getTransactions.mockResolvedValue([makeTransaction()]);
      await useBillingStore.getState().fetchBilling(0);
      await useBillingStore.getState().fetchVentopayBilling(0);

      const gourmet = useBillingStore.getState().getSelectedGourmetBilling();
      const ventopay = useBillingStore.getState().getSelectedVentopayBilling();
      expect(gourmet?.totalBilling).toBe(3.00);
      expect(ventopay?.total).toBe(4.50);
    });

    it('return null for an out-of-range month index', () => {
      useBillingStore.setState({ selectedMonthIndex: 7 });
      expect(useBillingStore.getState().getSelectedGourmetBilling()).toBeNull();
      expect(useBillingStore.getState().getSelectedVentopayBilling()).toBeNull();
    });
  });

  describe('fetchBilling guards', () => {
    it('returns early for an invalid month offset', async () => {
      await useBillingStore.getState().fetchBilling(7);
      expect(mockGourmetApi.getBillings).not.toHaveBeenCalled();
    });

    it('falls back to the selected month when no offset is given', async () => {
      mockGourmetApi.getBillings.mockResolvedValue([]);
      mockVentopayApi.getTransactions.mockResolvedValue([]);
      useBillingStore.setState({ selectedMonthIndex: 1 });

      await useBillingStore.getState().fetchBilling();
      await useBillingStore.getState().fetchVentopayBilling();

      expect(mockGourmetApi.getBillings).toHaveBeenCalledWith('1');
      expect(mockVentopayApi.getTransactions).toHaveBeenCalled();
    });

    it('uses a fallback message for non-Error rejections', async () => {
      mockGourmetApi.getBillings.mockRejectedValue('boom');

      await useBillingStore.getState().fetchBilling(0);

      expect(useBillingStore.getState().error).toBe('Abrechnung konnte nicht geladen werden');
    });

    it('skips fetching while another fetch is in flight', async () => {
      useBillingStore.setState({ loading: true });
      await useBillingStore.getState().fetchBilling(0);
      expect(mockGourmetApi.getBillings).not.toHaveBeenCalled();
    });
  });

  describe('fetchVentopayBilling guards and errors', () => {
    it('returns early for an invalid month offset', async () => {
      await useBillingStore.getState().fetchVentopayBilling(7);
      expect(mockVentopayApi.getTransactions).not.toHaveBeenCalled();
    });

    it('skips fetch for past months with existing transactions', async () => {
      const options = useBillingStore.getState().getMonthOptions();
      const pastKey = options[1].key;
      useBillingStore.setState({
        ventopayMonths: {
          [pastKey]: {
            monthKey: pastKey,
            label: options[1].label,
            transactions: [makeTransaction()],
            total: 4.50,
            fetchedAt: Date.now(),
          },
        },
      });

      await useBillingStore.getState().fetchVentopayBilling(1);

      expect(mockVentopayApi.getTransactions).not.toHaveBeenCalled();
    });

    it('skips fetch when Ventopay is not authenticated', async () => {
      (useVentopayAuthStore as any).getState().status = 'unauthenticated';

      await useBillingStore.getState().fetchVentopayBilling(0);

      expect(mockVentopayApi.getTransactions).not.toHaveBeenCalled();
    });

    it('warns but does not set error on failure', async () => {
      const warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => undefined);
      mockVentopayApi.getTransactions.mockRejectedValue(new Error('Ventopay down'));

      await useBillingStore.getState().fetchVentopayBilling(0);

      expect(useBillingStore.getState().error).toBeNull();
      expect(warnSpy).toHaveBeenCalled();
      warnSpy.mockRestore();
    });

    it('caches transactions to AsyncStorage', async () => {
      const AsyncStorage = require('@react-native-async-storage/async-storage').default;
      mockVentopayApi.getTransactions.mockResolvedValue([makeTransaction()]);

      await useBillingStore.getState().fetchVentopayBilling(0);

      const call = AsyncStorage.setItem.mock.calls.find(([k]: [string]) => k.startsWith('ventopay_billing_'));
      expect(call).toBeDefined();
    });
  });

  describe('loadCachedMonths', () => {
    it('restores gourmet and ventopay months with revived dates', async () => {
      // Populate the cache through real fetches, then reset in-memory state
      mockGourmetApi.getBillings.mockResolvedValue([makeBill()]);
      mockVentopayApi.getTransactions.mockResolvedValue([makeTransaction()]);
      await useBillingStore.getState().fetchBilling(0);
      await useBillingStore.getState().fetchVentopayBilling(0);
      useBillingStore.setState({ gourmetMonths: {}, ventopayMonths: {} });

      await useBillingStore.getState().loadCachedMonths();

      const options = useBillingStore.getState().getMonthOptions();
      const gourmet = useBillingStore.getState().gourmetMonths[options[0].key];
      const ventopay = useBillingStore.getState().ventopayMonths[options[0].key];

      expect(gourmet).toBeDefined();
      expect(gourmet.bills[0].billDate).toBeInstanceOf(Date);
      expect(gourmet.totalBilling).toBe(3.00);
      expect(gourmet.fetchedAt).toBe(0);

      expect(ventopay).toBeDefined();
      expect(ventopay.transactions[0].date).toBeInstanceOf(Date);
      expect(ventopay.total).toBe(4.50);
    });

    it('leaves months without cached data absent', async () => {
      await useBillingStore.getState().loadCachedMonths();
      expect(Object.keys(useBillingStore.getState().gourmetMonths)).toHaveLength(0);
      expect(Object.keys(useBillingStore.getState().ventopayMonths)).toHaveLength(0);
    });
  });
});

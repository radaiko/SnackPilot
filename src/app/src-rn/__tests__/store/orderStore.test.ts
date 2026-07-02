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
jest.mock('../../store/authStore', () => {
  const mockApi = {
    getOrders: jest.fn(),
    confirmOrders: jest.fn(),
    cancelOrders: jest.fn(),
  };
  return {
    useAuthStore: {
      getState: () => ({ api: mockApi }),
      setState: jest.fn(),
      subscribe: jest.fn(),
    },
  };
});

const mockCancelGeofenceNotification = jest.fn().mockResolvedValue(undefined);
jest.mock('../../utils/notificationService', () => ({
  cancelGeofenceNotification: mockCancelGeofenceNotification,
}));
const mockCheckDailyReminder = jest.fn().mockResolvedValue(undefined);
jest.mock('../../utils/dailyReminderCheck', () => ({
  checkDailyReminder: mockCheckDailyReminder,
}));
const mockCheckCancelReminder = jest.fn().mockResolvedValue(undefined);
jest.mock('../../utils/cancelReminderCheck', () => ({
  checkCancelReminder: mockCheckCancelReminder,
}));

import { useOrderStore } from '../../store/orderStore';
import { useAuthStore } from '../../store/authStore';
import { GourmetOrderedMenu } from '../../types/order';

const mockApi = (useAuthStore as any).getState().api;

function makeOrder(overrides: Partial<GourmetOrderedMenu> = {}): GourmetOrderedMenu {
  return {
    positionId: 'P1',
    eatingCycleId: 'E1',
    date: new Date(),
    title: 'MENÜ I',
    subtitle: '',
    approved: true,
    ...overrides,
  };
}

beforeEach(async () => {
  const AsyncStorage = require('@react-native-async-storage/async-storage').default;
  await AsyncStorage.clear();
  jest.clearAllMocks();
  useOrderStore.setState({
    orders: [],
    loading: false,
    cancellingId: null,
    error: null,
  });
});

describe('orderStore', () => {
  describe('fetchOrders', () => {
    it('sets loading during fetch', async () => {
      mockApi.getOrders.mockResolvedValue([]);

      const promise = useOrderStore.getState().fetchOrders();
      expect(useOrderStore.getState().loading).toBe(true);

      await promise;
      expect(useOrderStore.getState().loading).toBe(false);
    });

    it('stores fetched orders', async () => {
      const orders = [makeOrder()];
      mockApi.getOrders.mockResolvedValue(orders);

      await useOrderStore.getState().fetchOrders();

      expect(useOrderStore.getState().orders).toEqual(orders);
    });

    it('sets error on failure', async () => {
      mockApi.getOrders.mockRejectedValue(new Error('Fetch failed'));

      await useOrderStore.getState().fetchOrders();

      expect(useOrderStore.getState().error).toBe('Fetch failed');
      expect(useOrderStore.getState().loading).toBe(false);
    });

    it('uses a fallback message for non-Error rejections', async () => {
      mockApi.getOrders.mockRejectedValue('boom');

      await useOrderStore.getState().fetchOrders();

      expect(useOrderStore.getState().error).toBe('Bestellungen konnten nicht geladen werden');
    });

    it('skips fetch while another fetch is in flight', async () => {
      useOrderStore.setState({ loading: true });

      await useOrderStore.getState().fetchOrders();

      expect(mockApi.getOrders).not.toHaveBeenCalled();
    });

    it('cancels the geofence notification and updates reminders when an order exists today', async () => {
      mockApi.getOrders.mockResolvedValue([makeOrder({ date: new Date() })]);

      await useOrderStore.getState().fetchOrders();

      expect(mockCancelGeofenceNotification).toHaveBeenCalled();
      expect(mockCheckDailyReminder).toHaveBeenCalled();
      expect(mockCheckCancelReminder).toHaveBeenCalled();
    });

    it('does not cancel the geofence notification without an order today', async () => {
      const nextWeek = new Date();
      nextWeek.setDate(nextWeek.getDate() + 7);
      mockApi.getOrders.mockResolvedValue([makeOrder({ date: nextWeek })]);

      await useOrderStore.getState().fetchOrders();

      expect(mockCancelGeofenceNotification).not.toHaveBeenCalled();
      expect(mockCheckDailyReminder).toHaveBeenCalled();
    });

    it('ignores notification helper failures', async () => {
      mockApi.getOrders.mockResolvedValue([makeOrder({ date: new Date() })]);
      mockCancelGeofenceNotification.mockRejectedValueOnce(new Error('no permission'));

      await useOrderStore.getState().fetchOrders();

      expect(useOrderStore.getState().error).toBeNull();
      expect(useOrderStore.getState().orders).toHaveLength(1);
    });
  });

  describe('confirmOrders', () => {
    it('calls api.confirmOrders', async () => {
      mockApi.confirmOrders.mockResolvedValue(undefined);
      mockApi.getOrders.mockResolvedValue([]);

      await useOrderStore.getState().confirmOrders();

      expect(mockApi.confirmOrders).toHaveBeenCalled();
    });

    it('refreshes orders after confirm', async () => {
      mockApi.confirmOrders.mockResolvedValue(undefined);
      mockApi.getOrders.mockResolvedValue([]);

      await useOrderStore.getState().confirmOrders();

      expect(mockApi.getOrders).toHaveBeenCalled();
    });

    it('sets error on failure', async () => {
      mockApi.confirmOrders.mockRejectedValue(new Error('Confirm failed'));

      await useOrderStore.getState().confirmOrders();

      expect(useOrderStore.getState().error).toBe('Confirm failed');
      expect(useOrderStore.getState().loading).toBe(false);
      expect(mockApi.getOrders).not.toHaveBeenCalled();
    });

    it('uses a fallback message for non-Error rejections', async () => {
      mockApi.confirmOrders.mockRejectedValue('boom');

      await useOrderStore.getState().confirmOrders();

      expect(useOrderStore.getState().error).toBe('Bestellungen konnten nicht bestätigt werden');
    });
  });

  describe('cancelOrder', () => {
    it('calls api.cancelOrders with positionId', async () => {
      mockApi.cancelOrders.mockResolvedValue(undefined);
      mockApi.getOrders.mockResolvedValue([]);

      await useOrderStore.getState().cancelOrder('P1');

      expect(mockApi.cancelOrders).toHaveBeenCalledWith(['P1']);
    });

    it('sets cancellingId during operation', async () => {
      let resolveFn: () => void;
      mockApi.cancelOrders.mockReturnValue(new Promise<void>((r) => { resolveFn = r; }));
      mockApi.getOrders.mockResolvedValue([]);

      const promise = useOrderStore.getState().cancelOrder('P1');
      expect(useOrderStore.getState().cancellingId).toBe('P1');

      resolveFn!();
      await promise;
      expect(useOrderStore.getState().cancellingId).toBe(null);
    });

    it('skips when another cancellation is in flight', async () => {
      useOrderStore.setState({ cancellingId: 'P9' });

      await useOrderStore.getState().cancelOrder('P1');

      expect(mockApi.cancelOrders).not.toHaveBeenCalled();
      expect(useOrderStore.getState().cancellingId).toBe('P9');
    });

    it('sets error and clears cancellingId on failure', async () => {
      mockApi.cancelOrders.mockRejectedValue(new Error('Cancel failed'));

      await useOrderStore.getState().cancelOrder('P1');

      expect(useOrderStore.getState().error).toBe('Cancel failed');
      expect(useOrderStore.getState().cancellingId).toBe(null);
      expect(mockApi.getOrders).not.toHaveBeenCalled();
    });

    it('uses a fallback message for non-Error rejections', async () => {
      mockApi.cancelOrders.mockRejectedValue('boom');

      await useOrderStore.getState().cancelOrder('P1');

      expect(useOrderStore.getState().error).toBe('Bestellung konnte nicht storniert werden');
    });
  });

  describe('computed getters', () => {
    const futureDate = new Date();
    futureDate.setDate(futureDate.getDate() + 7);
    const pastDate = new Date();
    pastDate.setDate(pastDate.getDate() - 7);

    beforeEach(() => {
      useOrderStore.setState({
        orders: [
          makeOrder({ positionId: 'P1', date: futureDate, approved: true }),
          makeOrder({ positionId: 'P2', date: pastDate, approved: false }),
        ],
      });
    });

    it('getUpcomingOrders returns future orders', () => {
      const upcoming = useOrderStore.getState().getUpcomingOrders();
      expect(upcoming).toHaveLength(1);
      expect(upcoming[0].positionId).toBe('P1');
    });

    it('getPastOrders returns past orders', () => {
      const past = useOrderStore.getState().getPastOrders();
      expect(past).toHaveLength(1);
      expect(past[0].positionId).toBe('P2');
    });

    it('getUnconfirmedCount returns count of unapproved orders', () => {
      expect(useOrderStore.getState().getUnconfirmedCount()).toBe(1);
    });
  });

  describe('caching', () => {
    it('fetchOrders writes orders to AsyncStorage', async () => {
      const AsyncStorage = require('@react-native-async-storage/async-storage').default;
      const orders = [makeOrder()];
      mockApi.getOrders.mockResolvedValue(orders);

      await useOrderStore.getState().fetchOrders();

      expect(AsyncStorage.setItem).toHaveBeenCalled();
      const [key] = AsyncStorage.setItem.mock.calls[0];
      expect(key).toBe('orders_list');
    });

    it('loadCachedOrders restores orders from AsyncStorage', async () => {
      const orders = [makeOrder({ date: new Date(2026, 1, 10) })];

      // Simulate a previous fetchOrders that cached data
      mockApi.getOrders.mockResolvedValue(orders);
      await useOrderStore.getState().fetchOrders();

      // Reset state to simulate cold start
      useOrderStore.setState({ orders: [], loading: false });

      // Load from cache
      await useOrderStore.getState().loadCachedOrders();

      const restored = useOrderStore.getState().orders;
      expect(restored).toHaveLength(1);
      expect(restored[0].date).toBeInstanceOf(Date);
      expect(restored[0].date.getFullYear()).toBe(2026);
      expect(restored[0].positionId).toBe('P1');
    });

    it('loadCachedOrders does nothing when cache is empty', async () => {
      await useOrderStore.getState().loadCachedOrders();
      expect(useOrderStore.getState().orders).toEqual([]);
    });

    it('loadCachedOrders discards a corrupt cache entry', async () => {
      const AsyncStorage = require('@react-native-async-storage/async-storage').default;
      await AsyncStorage.setItem('orders_list', '{not valid json');

      await useOrderStore.getState().loadCachedOrders();

      expect(useOrderStore.getState().orders).toEqual([]);
      expect(AsyncStorage.removeItem).toHaveBeenCalledWith('orders_list');
    });
  });
});

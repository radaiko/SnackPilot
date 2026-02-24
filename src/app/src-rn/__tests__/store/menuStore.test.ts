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
jest.mock('../../utils/dateUtils', () => ({
  ...jest.requireActual('../../utils/dateUtils'),
  isOrderingCutoff: jest.fn((...args: unknown[]) =>
    (jest.requireActual('../../utils/dateUtils') as any).isOrderingCutoff(...args)
  ),
}));
jest.mock('../../store/authStore', () => {
  const mockApi = {
    getMenus: jest.fn(),
    addToCart: jest.fn(),
    confirmOrders: jest.fn(),
    cancelOrders: jest.fn().mockResolvedValue(undefined),
  };
  return {
    useAuthStore: {
      getState: () => ({ api: mockApi }),
      setState: jest.fn(),
      subscribe: jest.fn(),
    },
  };
});
const mockCancelOrder = jest.fn().mockResolvedValue(undefined);
const mockFetchOrders = jest.fn().mockResolvedValue(undefined);
jest.mock('../../store/orderStore', () => ({
  useOrderStore: {
    getState: () => ({
      fetchOrders: mockFetchOrders,
      cancelOrder: mockCancelOrder,
      orders: [],
    }),
    setState: jest.fn(),
    subscribe: jest.fn(),
  },
}));

import { useMenuStore, ORDERING_CUTOFF_MESSAGE } from '../../store/menuStore';
import { useAuthStore } from '../../store/authStore';
import { GourmetMenuCategory } from '../../types/menu';
import { MENU_CACHE_VALIDITY_MS } from '../../utils/constants';

const mockApi = (useAuthStore as any).getState().api;

function makeItem(overrides: Partial<import('../../types/menu').GourmetMenuItem> = {}): import('../../types/menu').GourmetMenuItem {
  return {
    id: 'menu-001',
    day: new Date(2026, 1, 10),
    title: GourmetMenuCategory.Menu1,
    subtitle: '',
    allergens: [],
    available: true,
    ordered: false,
    category: GourmetMenuCategory.Menu1,
    price: '',
    ...overrides,
  };
}

beforeEach(async () => {
  const AsyncStorage = require('@react-native-async-storage/async-storage').default;
  await AsyncStorage.clear();
  jest.clearAllMocks();
  useMenuStore.setState({
    items: [],
    lastFetched: null,
    loading: false,
    refreshing: false,
    error: null,
    selectedDate: new Date(),
    pendingOrders: new Set(),
    pendingCancellations: new Set(),
    orderProgress: null,
  });
});

describe('menuStore', () => {
  describe('fetchMenus', () => {
    it('sets loading=true during fetch', async () => {
      const items = [makeItem()];
      mockApi.getMenus.mockResolvedValue(items);

      const promise = useMenuStore.getState().fetchMenus();
      expect(useMenuStore.getState().loading).toBe(true);

      await promise;
      expect(useMenuStore.getState().loading).toBe(false);
    });

    it('stores fetched items and lastFetched', async () => {
      const items = [makeItem()];
      mockApi.getMenus.mockResolvedValue(items);

      await useMenuStore.getState().fetchMenus();

      expect(useMenuStore.getState().items).toEqual(items);
      expect(useMenuStore.getState().lastFetched).toBeDefined();
      expect(typeof useMenuStore.getState().lastFetched).toBe('number');
    });

    it('does not fetch if already loading', async () => {
      useMenuStore.setState({ loading: true });

      await useMenuStore.getState().fetchMenus();

      expect(mockApi.getMenus).not.toHaveBeenCalled();
    });

    it('uses cache if within validity period', async () => {
      useMenuStore.setState({ lastFetched: Date.now(), items: [makeItem()] });

      await useMenuStore.getState().fetchMenus();

      expect(mockApi.getMenus).not.toHaveBeenCalled();
    });

    it('force bypasses cache', async () => {
      useMenuStore.setState({ lastFetched: Date.now(), items: [makeItem()] });
      mockApi.getMenus.mockResolvedValue([makeItem()]);

      await useMenuStore.getState().fetchMenus(true);

      expect(mockApi.getMenus).toHaveBeenCalled();
    });

    it('sets error on failure', async () => {
      mockApi.getMenus.mockRejectedValue(new Error('Network error'));

      await useMenuStore.getState().fetchMenus();

      expect(useMenuStore.getState().error).toBe('Network error');
      expect(useMenuStore.getState().loading).toBe(false);
    });
  });

  describe('refreshAvailability', () => {
    it('merges fresh availability into cached items', async () => {
      const cachedItems = [
        makeItem({ available: true, ordered: false }),
      ];
      useMenuStore.setState({ items: cachedItems });

      const freshItems = [
        makeItem({ available: false, ordered: true }),
      ];
      mockApi.getMenus.mockResolvedValue(freshItems);

      await useMenuStore.getState().refreshAvailability();

      const merged = useMenuStore.getState().items;
      expect(merged).toHaveLength(1);
      expect(merged[0].available).toBe(false);
      expect(merged[0].ordered).toBe(true);
    });
  });

  describe('setSelectedDate', () => {
    it('updates selectedDate', () => {
      const date = new Date(2026, 2, 15);
      useMenuStore.getState().setSelectedDate(date);
      expect(useMenuStore.getState().selectedDate).toBe(date);
    });
  });

  describe('togglePendingOrder', () => {
    const items = [
      makeItem({ id: 'menu-001', day: new Date(2026, 1, 10), title: GourmetMenuCategory.Menu1, category: GourmetMenuCategory.Menu1 }),
      makeItem({ id: 'menu-002', day: new Date(2026, 1, 10), title: 'MENÜ II', category: GourmetMenuCategory.Menu2 }),
    ];

    beforeEach(() => {
      useMenuStore.setState({ items });
    });

    it('adds item to pendingOrders', () => {
      useMenuStore.getState().togglePendingOrder('menu-001', new Date(2026, 1, 10));
      expect(useMenuStore.getState().pendingOrders.size).toBe(1);
    });

    it('removes item on second toggle', () => {
      const date = new Date(2026, 1, 10);
      useMenuStore.getState().togglePendingOrder('menu-001', date);
      expect(useMenuStore.getState().pendingOrders.size).toBe(1);

      useMenuStore.getState().togglePendingOrder('menu-001', date);
      expect(useMenuStore.getState().pendingOrders.size).toBe(0);
    });

    it('allows multiple main menus per day', () => {
      const date = new Date(2026, 1, 10);
      useMenuStore.getState().togglePendingOrder('menu-001', date);
      expect(useMenuStore.getState().pendingOrders.size).toBe(1);

      // Adding MENÜ II for the same day should keep both
      useMenuStore.getState().togglePendingOrder('menu-002', date);
      expect(useMenuStore.getState().pendingOrders.size).toBe(2);

      const keys = Array.from(useMenuStore.getState().pendingOrders);
      expect(keys.some((k) => k.startsWith('menu-001|'))).toBe(true);
      expect(keys.some((k) => k.startsWith('menu-002|'))).toBe(true);
    });
  });

  describe('pendingCancellations', () => {
    it('togglePendingOrder on ordered item adds to pendingCancellations', () => {
      const items = [
        makeItem({ id: 'menu-001', day: new Date(2026, 1, 10), ordered: true }),
      ];
      useMenuStore.setState({ items });

      useMenuStore.getState().togglePendingOrder('menu-001', new Date(2026, 1, 10));

      expect(useMenuStore.getState().pendingCancellations.size).toBe(1);
      expect(useMenuStore.getState().pendingOrders.size).toBe(0);
    });

    it('togglePendingOrder on non-ordered item still adds to pendingOrders', () => {
      const items = [
        makeItem({ id: 'menu-001', day: new Date(2026, 1, 10), ordered: false }),
      ];
      useMenuStore.setState({ items });

      useMenuStore.getState().togglePendingOrder('menu-001', new Date(2026, 1, 10));

      expect(useMenuStore.getState().pendingOrders.size).toBe(1);
      expect(useMenuStore.getState().pendingCancellations.size).toBe(0);
    });

    it('second toggle on ordered item removes from pendingCancellations', () => {
      const items = [
        makeItem({ id: 'menu-001', day: new Date(2026, 1, 10), ordered: true }),
      ];
      useMenuStore.setState({ items });

      const date = new Date(2026, 1, 10);
      useMenuStore.getState().togglePendingOrder('menu-001', date);
      expect(useMenuStore.getState().pendingCancellations.size).toBe(1);

      useMenuStore.getState().togglePendingOrder('menu-001', date);
      expect(useMenuStore.getState().pendingCancellations.size).toBe(0);
    });
  });

  describe('submitOrders', () => {
    beforeEach(() => {
      const items = [
        makeItem({ id: 'menu-001', day: new Date(2026, 5, 10) }),
      ];
      useMenuStore.setState({ items });
      mockApi.addToCart.mockResolvedValue(undefined);
      mockApi.confirmOrders.mockResolvedValue(undefined);
      mockApi.getMenus.mockResolvedValue([]);
      // Restore default orderStore mock (tests that override getState must not leak)
      (require('../../store/orderStore').useOrderStore as any).getState = () => ({
        fetchOrders: mockFetchOrders,
        cancelOrder: mockCancelOrder,
        orders: [],
      });
    });

    it('calls addToCart and confirmOrders', async () => {
      const date = new Date(2026, 5, 10);
      useMenuStore.getState().togglePendingOrder('menu-001', date);

      await useMenuStore.getState().submitOrders();

      expect(mockApi.addToCart).toHaveBeenCalled();
      expect(mockApi.confirmOrders).toHaveBeenCalled();
    });

    it('submits multiple menus for the same date', async () => {
      const items = [
        makeItem({ id: 'menu-001', day: new Date(2026, 5, 10), category: GourmetMenuCategory.Menu1 }),
        makeItem({ id: 'menu-002', day: new Date(2026, 5, 10), category: GourmetMenuCategory.Menu2 }),
      ];
      useMenuStore.setState({ items });
      const date = new Date(2026, 5, 10);
      useMenuStore.getState().togglePendingOrder('menu-001', date);
      useMenuStore.getState().togglePendingOrder('menu-002', date);

      await useMenuStore.getState().submitOrders();
      expect(useMenuStore.getState().error).toBeNull();

      expect(mockApi.addToCart).toHaveBeenCalledTimes(1);
      const callArg: { menuId: string; date: Date }[] = mockApi.addToCart.mock.calls[0][0];
      expect(callArg).toHaveLength(2);
      expect(callArg.some((i: { menuId: string }) => i.menuId === 'menu-001')).toBe(true);
      expect(callArg.some((i: { menuId: string }) => i.menuId === 'menu-002')).toBe(true);
    });

    it('clears pendingOrders after submit', async () => {
      const date = new Date(2026, 5, 10);
      useMenuStore.getState().togglePendingOrder('menu-001', date);

      await useMenuStore.getState().submitOrders();

      expect(useMenuStore.getState().pendingOrders.size).toBe(0);
    });

    it('sets error on failure', async () => {
      const date = new Date(2026, 5, 10);
      useMenuStore.getState().togglePendingOrder('menu-001', date);
      mockApi.addToCart.mockRejectedValue(new Error('Cart error'));

      await useMenuStore.getState().submitOrders();

      expect(useMenuStore.getState().error).toBe('Cart error');
    });

    it('cancels orders from pendingCancellations before adding new ones', async () => {
      const date = new Date(2026, 5, 10);
      const items = [
        makeItem({ id: 'menu-001', day: date, ordered: true, category: GourmetMenuCategory.Menu1 }),
        makeItem({ id: 'menu-002', day: date, ordered: false, category: GourmetMenuCategory.Menu2 }),
      ];
      useMenuStore.setState({ items });

      // Simulate: orderStore has the order we want to cancel
      const mockOrders = [{ positionId: 'P1', eatingCycleId: 'E1', date, title: GourmetMenuCategory.Menu1, subtitle: '', approved: true }];
      (require('../../store/orderStore').useOrderStore as any).getState = () => ({
        fetchOrders: mockFetchOrders,
        cancelOrder: mockCancelOrder,
        orders: mockOrders,
      });

      // Mark menu-001 for cancellation (it's ordered)
      useMenuStore.getState().togglePendingOrder('menu-001', date);
      // Select menu-002 as new order
      useMenuStore.getState().togglePendingOrder('menu-002', date);

      await useMenuStore.getState().submitOrders();

      expect(mockApi.cancelOrders).toHaveBeenCalledWith(['P1']);
      expect(mockApi.addToCart).toHaveBeenCalled();
      expect(mockApi.confirmOrders).toHaveBeenCalled();
    });

    it('handles cancellation-only submit (no new orders)', async () => {
      const date = new Date(2026, 5, 10);
      const items = [
        makeItem({ id: 'menu-001', day: date, ordered: true, category: GourmetMenuCategory.Menu1 }),
      ];
      useMenuStore.setState({ items });

      const mockOrders = [{ positionId: 'P1', eatingCycleId: 'E1', date, title: GourmetMenuCategory.Menu1, subtitle: '', approved: true }];
      (require('../../store/orderStore').useOrderStore as any).getState = () => ({
        fetchOrders: mockFetchOrders,
        cancelOrder: mockCancelOrder,
        orders: mockOrders,
      });

      useMenuStore.getState().togglePendingOrder('menu-001', date);

      await useMenuStore.getState().submitOrders();

      expect(mockApi.cancelOrders).toHaveBeenCalledWith(['P1']);
      expect(mockApi.addToCart).not.toHaveBeenCalled();
    });

    it('processes cancellations even when new orders are cutoff-blocked', async () => {
      const { isOrderingCutoff } = require('../../utils/dateUtils') as { isOrderingCutoff: jest.Mock };
      isOrderingCutoff.mockReturnValue(true);

      try {
        const today = new Date(2026, 1, 21);
        const items = [
          makeItem({ id: 'menu-001', day: today, ordered: true, category: GourmetMenuCategory.Menu1 }),
          makeItem({ id: 'menu-002', day: today, ordered: false, category: GourmetMenuCategory.Menu2 }),
        ];
        useMenuStore.setState({ items });

        const mockOrders = [{ positionId: 'P1', eatingCycleId: 'E1', date: today, title: GourmetMenuCategory.Menu1, subtitle: '', approved: true }];
        (require('../../store/orderStore').useOrderStore as any).getState = () => ({
          fetchOrders: mockFetchOrders,
          cancelOrder: mockCancelOrder,
          orders: mockOrders,
        });

        // Mark menu-001 for cancellation and menu-002 as new order
        useMenuStore.getState().togglePendingOrder('menu-001', today);
        useMenuStore.getState().togglePendingOrder('menu-002', today);

        await useMenuStore.getState().submitOrders();

        // Cancellation should still proceed
        expect(mockApi.cancelOrders).toHaveBeenCalledWith(['P1']);
        // New order should be blocked
        expect(mockApi.addToCart).not.toHaveBeenCalled();
        // Error should be shown for the blocked new order
        expect(useMenuStore.getState().error).toBe(ORDERING_CUTOFF_MESSAGE);
      } finally {
        isOrderingCutoff.mockRestore();
      }
    });

    it('clears both pendingOrders and pendingCancellations after submit', async () => {
      const date = new Date(2026, 5, 10);
      const items = [
        makeItem({ id: 'menu-001', day: date, ordered: true, category: GourmetMenuCategory.Menu1 }),
        makeItem({ id: 'menu-002', day: date, ordered: false, category: GourmetMenuCategory.Menu2 }),
      ];
      useMenuStore.setState({ items });

      const mockOrders = [{ positionId: 'P1', eatingCycleId: 'E1', date, title: GourmetMenuCategory.Menu1, subtitle: '', approved: true }];
      (require('../../store/orderStore').useOrderStore as any).getState = () => ({
        fetchOrders: mockFetchOrders,
        cancelOrder: mockCancelOrder,
        orders: mockOrders,
      });

      useMenuStore.getState().togglePendingOrder('menu-001', date);
      useMenuStore.getState().togglePendingOrder('menu-002', date);

      await useMenuStore.getState().submitOrders();

      expect(useMenuStore.getState().pendingOrders.size).toBe(0);
      expect(useMenuStore.getState().pendingCancellations.size).toBe(0);
    });
  });

  describe('computed getters', () => {
    it('getAvailableDates returns sorted unique dates', () => {
      const items = [
        makeItem({ id: 'a', day: new Date(2026, 1, 12) }),
        makeItem({ id: 'b', day: new Date(2026, 1, 10) }),
        makeItem({ id: 'c', day: new Date(2026, 1, 10), category: GourmetMenuCategory.Menu2 }),
      ];
      useMenuStore.setState({ items });

      const dates = useMenuStore.getState().getAvailableDates();
      expect(dates).toHaveLength(2);
      expect(dates[0].getDate()).toBe(10);
      expect(dates[1].getDate()).toBe(12);
    });

    it('getMenusForDate returns items matching the date', () => {
      const items = [
        makeItem({ id: 'a', day: new Date(2026, 1, 10) }),
        makeItem({ id: 'b', day: new Date(2026, 1, 11) }),
      ];
      useMenuStore.setState({ items });

      const result = useMenuStore.getState().getMenusForDate(new Date(2026, 1, 10));
      expect(result).toHaveLength(1);
      expect(result[0].id).toBe('a');
    });

    it('getDayMenus returns grouped menus by date', () => {
      const items = [
        makeItem({ id: 'a', day: new Date(2026, 1, 10) }),
        makeItem({ id: 'b', day: new Date(2026, 1, 11) }),
      ];
      useMenuStore.setState({ items });

      const dayMenus = useMenuStore.getState().getDayMenus();
      expect(dayMenus).toHaveLength(2);
      expect(dayMenus[0].items).toHaveLength(1);
    });

    it('getPendingCount returns size of pendingOrders', () => {
      useMenuStore.setState({
        items: [makeItem()],
        pendingOrders: new Set(['menu-001|2026-02-10']),
      });

      expect(useMenuStore.getState().getPendingCount()).toBe(1);
    });

    it('getPendingCount includes both pendingOrders and pendingCancellations', () => {
      useMenuStore.setState({
        items: [makeItem()],
        pendingOrders: new Set(['menu-001|2026-02-10']),
        pendingCancellations: new Set(['menu-002|2026-02-11']),
      });

      expect(useMenuStore.getState().getPendingCount()).toBe(2);
    });

    it('getPendingCancellationCount returns only cancellations count', () => {
      useMenuStore.setState({
        items: [makeItem()],
        pendingOrders: new Set(['menu-001|2026-02-10']),
        pendingCancellations: new Set(['menu-002|2026-02-11']),
      });

      expect(useMenuStore.getState().getPendingCancellationCount()).toBe(1);
    });
  });

  describe('caching', () => {
    it('fetchMenus writes items to AsyncStorage', async () => {
      const AsyncStorage = require('@react-native-async-storage/async-storage').default;
      const items = [makeItem()];
      mockApi.getMenus.mockResolvedValue(items);

      await useMenuStore.getState().fetchMenus();

      expect(AsyncStorage.setItem).toHaveBeenCalled();
      const [key] = AsyncStorage.setItem.mock.calls[0];
      expect(key).toBe('menus_items');
    });

    it('loadCachedMenus restores items from AsyncStorage', async () => {
      const AsyncStorage = require('@react-native-async-storage/async-storage').default;
      const items = [makeItem({ day: new Date(2026, 1, 10) })];

      // Simulate a previous fetchMenus that cached data
      mockApi.getMenus.mockResolvedValue(items);
      await useMenuStore.getState().fetchMenus();

      // Reset state to simulate cold start
      useMenuStore.setState({ items: [], lastFetched: null });

      // Load from cache
      await useMenuStore.getState().loadCachedMenus();

      const restored = useMenuStore.getState().items;
      expect(restored).toHaveLength(1);
      expect(restored[0].day).toBeInstanceOf(Date);
      expect(restored[0].day.getFullYear()).toBe(2026);
      expect(restored[0].id).toBe('menu-001');
    });

    it('loadCachedMenus does nothing when cache is empty', async () => {
      await useMenuStore.getState().loadCachedMenus();
      expect(useMenuStore.getState().items).toEqual([]);
    });
  });
});

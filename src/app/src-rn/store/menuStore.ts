import { create } from 'zustand';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { GourmetMenuItem, GourmetDayMenu } from '../types/menu';
import { useAuthStore } from './authStore';
import { useOrderStore } from './orderStore';
import { MENU_CACHE_VALIDITY_MS } from '../utils/constants';
import { isSameDay, isOrderingCutoff, localDateKey, findNearestDate } from '../utils/dateUtils';
import { trackSignal } from '../utils/analytics';

const MENU_CACHE_KEY = 'menus_items';
export const ORDERING_CUTOFF_MESSAGE = 'Bestellung für heute geschlossen (Bestellschluss 9:00)';

/** Serialize menu items for AsyncStorage (Date -> ISO string). */
function serializeMenuItems(items: GourmetMenuItem[]): string {
  return JSON.stringify(items.map((item) => ({
    ...item,
    day: item.day.toISOString(),
  })));
}

/** Deserialize menu items from AsyncStorage (ISO string -> Date). */
function deserializeMenuItems(json: string): GourmetMenuItem[] {
  return JSON.parse(json).map((item: any) => ({
    ...item,
    day: new Date(item.day),
  }));
}

export type OrderProgress = 'adding' | 'confirming' | 'cancelling' | 'refreshing' | null;

interface MenuState {
  items: GourmetMenuItem[];
  lastFetched: number | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  selectedDate: Date;
  pendingOrders: Set<string>; // Set of "menuId|dateStr" keys for items to order
  pendingCancellations: Set<string>; // Set of "menuId|dateStr" keys for ordered items to cancel
  orderProgress: OrderProgress; // Non-blocking background order step

  loadCachedMenus: () => Promise<void>;
  fetchMenus: (force?: boolean) => Promise<void>;
  refreshAvailability: () => Promise<void>;
  setSelectedDate: (date: Date) => void;
  togglePendingOrder: (menuId: string, date: Date) => void;
  clearPendingChanges: () => void;
  submitOrders: () => Promise<void>;
  getAvailableDates: () => Date[];
  getMenusForDate: (date: Date) => GourmetMenuItem[];
  getDayMenus: () => GourmetDayMenu[];
  getPendingCount: () => number;
  getPendingCancellationCount: () => number;
}

function makePendingKey(menuId: string, date: Date): string {
  return `${menuId}|${localDateKey(date)}`;
}

export const useMenuStore = create<MenuState>((set, get) => ({
  items: [],
  lastFetched: null,
  loading: false,
  refreshing: false,
  error: null,
  selectedDate: new Date(),
  pendingOrders: new Set(),
  pendingCancellations: new Set(),
  orderProgress: null,

  loadCachedMenus: async () => {
    const cached = await AsyncStorage.getItem(MENU_CACHE_KEY);
    if (!cached) return;
    try {
      const items = deserializeMenuItems(cached);
      set({ items });
    } catch {
      await AsyncStorage.removeItem(MENU_CACHE_KEY);
    }
  },

  fetchMenus: async (force = false) => {
    const { lastFetched, loading } = get();
    if (loading) return;

    // Check cache validity
    if (!force && lastFetched && Date.now() - lastFetched < MENU_CACHE_VALIDITY_MS) {
      return;
    }

    set({ loading: true, error: null });
    try {
      const api = useAuthStore.getState().api;
      const items = await api.getMenus();
      set({ items, lastFetched: Date.now(), loading: false });
      await AsyncStorage.setItem(MENU_CACHE_KEY, serializeMenuItems(items));

      // Auto-select nearest date if current selection has no menus
      const dates = get().getAvailableDates();
      const current = get().selectedDate;
      const stillExists = dates.some((d) => d.toDateString() === current.toDateString());
      if (dates.length > 0 && !stillExists) {
        const nearest = findNearestDate(dates, current);
        set({ selectedDate: nearest ?? dates[0] });
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Menüs konnten nicht geladen werden';
      set({ error: message, loading: false });
    }
  },

  /**
   * Background refresh: fetch fresh data and merge only available/ordered
   * into the cached items. No loading spinner, no replacing static fields.
   */
  refreshAvailability: async () => {
    const { refreshing, items } = get();
    if (refreshing || items.length === 0) return;

    const minVisible = new Promise((r) => setTimeout(r, 800));
    set({ refreshing: true });
    try {
      const api = useAuthStore.getState().api;
      const freshItems = await api.getMenus();

      // Build lookup from fresh data
      const freshMap = new Map<string, GourmetMenuItem>();
      for (const item of freshItems) {
        freshMap.set(`${item.id}|${localDateKey(item.day)}`, item);
      }

      // Merge: keep cached static fields, update only available/ordered
      const merged = get().items.map((cached) => {
        const key = `${cached.id}|${localDateKey(cached.day)}`;
        const fresh = freshMap.get(key);
        if (fresh) {
          freshMap.delete(key);
          return { ...cached, available: fresh.available, ordered: fresh.ordered };
        }
        return cached;
      });

      // Append any brand-new items not in cache
      for (const fresh of freshMap.values()) {
        merged.push(fresh);
      }

      // Keep banner visible for at least 800ms so the user notices it
      await minVisible;
      set({ items: merged, lastFetched: Date.now(), refreshing: false });
      await AsyncStorage.setItem(MENU_CACHE_KEY, serializeMenuItems(merged));
    } catch {
      await minVisible;
      // Silent fail — cached data remains visible
      set({ refreshing: false });
    }
  },

  setSelectedDate: (date: Date) => set({ selectedDate: date }),

  togglePendingOrder: (menuId: string, date: Date) => {
    const key = makePendingKey(menuId, date);
    const item = get().items.find(
      (i) => i.id === menuId && localDateKey(i.day) === localDateKey(date)
    );
    const isOrdered = item?.ordered ?? false;

    if (isOrdered) {
      const cancellations = new Set(get().pendingCancellations);
      if (cancellations.has(key)) {
        cancellations.delete(key);
      } else {
        cancellations.add(key);
      }
      set({ pendingCancellations: cancellations });
    } else {
      const pending = new Set(get().pendingOrders);
      if (pending.has(key)) {
        pending.delete(key);
      } else {
        pending.add(key);
      }
      set({ pendingOrders: pending });
    }
  },

  clearPendingChanges: () => set({ pendingOrders: new Set(), pendingCancellations: new Set() }),

  submitOrders: async () => {
    const { pendingOrders, pendingCancellations } = get();
    if (pendingOrders.size === 0 && pendingCancellations.size === 0) return;

    const api = useAuthStore.getState().api;
    const orderStoreState = useOrderStore.getState();

    // --- Resolve cancellations to positionIds ---
    const cancellationPositionIds: string[] = [];
    for (const key of pendingCancellations) {
      const [menuId, dateStr] = key.split('|');
      // Find the matching menu item to get its category
      const menuItem = get().items.find(
        (i) => i.id === menuId && localDateKey(i.day) === dateStr
      );
      if (!menuItem) continue;

      // Find the matching order by category + date
      const order = orderStoreState.orders.find(
        (o) => o.title === menuItem.category && localDateKey(o.date) === dateStr
      );
      if (order) {
        cancellationPositionIds.push(order.positionId);
      }
    }

    if (cancellationPositionIds.length < pendingCancellations.size) {
      console.warn(
        `Could not resolve all cancellations: ${cancellationPositionIds.length}/${pendingCancellations.size}`
      );
    }

    // --- Resolve new orders ---
    const newOrderItems = Array.from(pendingOrders).map((key) => {
      const [menuId, dateStr] = key.split('|');
      const [y, m, d] = dateStr.split('-').map(Number);
      return { menuId, date: new Date(y, m - 1, d) };
    });

    // Filter out cutoff-blocked new orders (cancellations are always allowed)
    const allowedNewOrders = newOrderItems.filter((i) => !isOrderingCutoff(i.date));
    const hasCutoffBlocked = allowedNewOrders.length < newOrderItems.length;
    if (hasCutoffBlocked && allowedNewOrders.length === 0 && cancellationPositionIds.length === 0) {
      set({ error: ORDERING_CUTOFF_MESSAGE });
      return;
    }

    // --- Optimistic UI update ---
    const cancelKeys = new Set(pendingCancellations);
    const orderKeys = new Set(pendingOrders);
    const optimisticItems = get().items.map((item) => {
      const key = makePendingKey(item.id, item.day);
      if (cancelKeys.has(key)) {
        return { ...item, ordered: false };
      }
      if (orderKeys.has(key)) {
        return { ...item, ordered: true };
      }
      return item;
    });
    set({
      items: optimisticItems,
      pendingOrders: new Set(),
      pendingCancellations: new Set(),
      error: hasCutoffBlocked ? ORDERING_CUTOFF_MESSAGE : null,
    });

    try {
      // Step 1: Cancel orders (batched — single edit-mode toggle)
      if (cancellationPositionIds.length > 0) {
        set({ orderProgress: 'cancelling' });
        await api.cancelOrders(cancellationPositionIds);
      }

      // Step 2: Add new orders to cart
      if (allowedNewOrders.length > 0) {
        set({ orderProgress: 'adding' });
        await api.addToCart(allowedNewOrders);

        set({ orderProgress: 'confirming' });
        await api.confirmOrders();
      }

      // Step 3: Refresh
      set({ orderProgress: 'refreshing' });
      await useOrderStore.getState().fetchOrders();
      await get().fetchMenus(true);

      trackSignal('order.submitted', {
        orderedCount: String(allowedNewOrders.length),
        cancelledCount: String(cancellationPositionIds.length),
      });

      set({
        orderProgress: null,
        // Restore cutoff warning (fetchMenus clears error)
        error: hasCutoffBlocked ? ORDERING_CUTOFF_MESSAGE : null,
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Bestellung konnte nicht aufgegeben werden';
      set({ error: message, orderProgress: null });
      // Revert optimistic update on failure
      try {
        const freshApi = useAuthStore.getState().api;
        const freshItems = await freshApi.getMenus();
        set({ items: freshItems, lastFetched: Date.now() });
        await AsyncStorage.setItem(MENU_CACHE_KEY, serializeMenuItems(freshItems));
      } catch {
        // Silent — keep optimistic state if revert also fails
      }
    }
  },

  getAvailableDates: () => {
    const { items } = get();
    const dateSet = new Map<string, Date>();
    for (const item of items) {
      const key = localDateKey(item.day);
      if (!dateSet.has(key)) {
        dateSet.set(key, item.day);
      }
    }
    return Array.from(dateSet.values()).sort((a, b) => a.getTime() - b.getTime());
  },

  getMenusForDate: (date: Date) => {
    return get().items.filter((item) => isSameDay(item.day, date));
  },

  getDayMenus: () => {
    const dates = get().getAvailableDates();
    return dates.map((date) => ({
      date,
      items: get().getMenusForDate(date),
    }));
  },

  getPendingCount: () => get().pendingOrders.size + get().pendingCancellations.size,

  getPendingCancellationCount: () => get().pendingCancellations.size,
}));

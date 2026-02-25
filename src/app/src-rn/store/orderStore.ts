import { create } from 'zustand';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { GourmetOrderedMenu } from '../types/order';
import { useAuthStore } from './authStore';
import { trackSignal } from '../utils/analytics';

const ORDER_CACHE_KEY = 'orders_list';

/** Serialize orders for AsyncStorage (Date -> ISO string). */
function serializeOrders(orders: GourmetOrderedMenu[]): string {
  return JSON.stringify(orders.map((o) => ({
    ...o,
    date: o.date.toISOString(),
  })));
}

/** Deserialize orders from AsyncStorage (ISO string -> Date). */
function deserializeOrders(json: string): GourmetOrderedMenu[] {
  return JSON.parse(json).map((o: any) => ({
    ...o,
    date: new Date(o.date),
  }));
}

interface OrderState {
  orders: GourmetOrderedMenu[];
  loading: boolean;
  cancellingId: string | null; // positionId currently being cancelled
  error: string | null;

  loadCachedOrders: () => Promise<void>;
  fetchOrders: () => Promise<void>;
  confirmOrders: () => Promise<void>;
  cancelOrder: (positionId: string) => Promise<void>;
  getUpcomingOrders: () => GourmetOrderedMenu[];
  getPastOrders: () => GourmetOrderedMenu[];
  getUnconfirmedCount: () => number;
}

export const useOrderStore = create<OrderState>((set, get) => ({
  orders: [],
  loading: false,
  cancellingId: null,
  error: null,

  loadCachedOrders: async () => {
    const cached = await AsyncStorage.getItem(ORDER_CACHE_KEY);
    if (!cached) return;
    try {
      const orders = deserializeOrders(cached);
      set({ orders });
    } catch {
      await AsyncStorage.removeItem(ORDER_CACHE_KEY);
    }
  },

  fetchOrders: async () => {
    if (get().loading) return;

    set({ loading: true, error: null });
    try {
      const api = useAuthStore.getState().api;
      const orders = await api.getOrders();
      set({ orders, loading: false });
      await AsyncStorage.setItem(ORDER_CACHE_KEY, serializeOrders(orders));
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Bestellungen konnten nicht geladen werden';
      set({ error: message, loading: false });
    }
  },

  confirmOrders: async () => {
    set({ loading: true, error: null });
    try {
      const api = useAuthStore.getState().api;
      await api.confirmOrders();
      trackSignal('order.confirmed');
      set({ loading: false });
      // Refresh orders to reflect confirmed state
      await get().fetchOrders();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Bestellungen konnten nicht bestätigt werden';
      set({ error: message, loading: false });
    }
  },

  cancelOrder: async (positionId: string) => {
    if (get().cancellingId !== null) return;
    set({ cancellingId: positionId, error: null });
    try {
      const api = useAuthStore.getState().api;
      await api.cancelOrders([positionId]);
      trackSignal('order.cancelled');
      set({ cancellingId: null });
      // Refresh orders after cancellation
      await get().fetchOrders();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Bestellung konnte nicht storniert werden';
      set({ error: message, cancellingId: null });
    }
  },

  getUpcomingOrders: () => {
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    return get().orders.filter((o) => o.date >= now);
  },

  getPastOrders: () => {
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    return get().orders.filter((o) => o.date < now);
  },

  getUnconfirmedCount: () => get().orders.filter((o) => !o.approved).length,
}));

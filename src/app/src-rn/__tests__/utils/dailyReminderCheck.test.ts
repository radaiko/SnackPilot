jest.mock('@react-native-async-storage/async-storage', () => {
  const store: Record<string, string> = {};
  return {
    __esModule: true,
    default: {
      getItem: jest.fn((key: string) => Promise.resolve(store[key] ?? null)),
      setItem: jest.fn((key: string, value: string) => { store[key] = value; return Promise.resolve(); }),
      removeItem: jest.fn((key: string) => { delete store[key]; return Promise.resolve(); }),
      clear: jest.fn(() => { Object.keys(store).forEach((k) => { delete store[k]; }); return Promise.resolve(); }),
    },
  };
});

const mockScheduleDailyReminderNotification = jest.fn().mockResolvedValue(true);
const mockCancelDailyReminderNotification = jest.fn().mockResolvedValue(undefined);

jest.mock('../../utils/notificationService', () => ({
  scheduleDailyReminderNotification: (...args: any[]) => mockScheduleDailyReminderNotification(...args),
  cancelDailyReminderNotification: (...args: any[]) => mockCancelDailyReminderNotification(...args),
}));

jest.mock('../../store/orderStore', () => {
  let orders: any[] = [];
  return {
    useOrderStore: {
      getState: () => ({ orders }),
      __setOrders: (o: any[]) => { orders = o; },
    },
  };
});

jest.mock('../../utils/dateUtils', () => ({
  viennaMinutes: jest.fn(),
  viennaToday: jest.fn(),
  isSameDay: jest.fn((a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  ),
  localDateKey: jest.fn((d: Date) => {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }),
}));

import { useOrderStore } from '../../store/orderStore';
import { viennaMinutes, viennaToday } from '../../utils/dateUtils';
import { setReminderEnabled, setReminderTime, setReminderSentDate } from '../../utils/reminderStorage';
import { checkDailyReminder } from '../../utils/dailyReminderCheck';

const mockViennaMinutes = viennaMinutes as jest.Mock;
const mockViennaToday = viennaToday as jest.Mock;

beforeEach(async () => {
  jest.clearAllMocks();
  const AsyncStorage = require('@react-native-async-storage/async-storage').default;
  await AsyncStorage.clear();
  (useOrderStore as any).__setOrders([]);
  mockViennaToday.mockReturnValue(new Date(2026, 1, 25)); // Feb 25, 2026
  mockViennaMinutes.mockReturnValue(0); // default: midnight
});

describe('checkDailyReminder', () => {
  it('does nothing when reminder is disabled', async () => {
    await setReminderEnabled(false);
    await setReminderTime(11, 0);

    await checkDailyReminder();

    expect(mockScheduleDailyReminderNotification).not.toHaveBeenCalled();
  });

  it('does nothing when no time is configured', async () => {
    await setReminderEnabled(true);

    await checkDailyReminder();

    expect(mockScheduleDailyReminderNotification).not.toHaveBeenCalled();
  });

  it('cancels notification and skips when no orders for today', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 26), title: 'MENÜ I', subtitle: 'Schnitzel', approved: true },
    ]);

    await checkDailyReminder();

    expect(mockCancelDailyReminderNotification).toHaveBeenCalled();
    expect(mockScheduleDailyReminderNotification).not.toHaveBeenCalled();
  });

  it('does nothing when already sent today', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    await setReminderSentDate('2026-02-25');

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Schnitzel', approved: true },
    ]);

    await checkDailyReminder();

    expect(mockScheduleDailyReminderNotification).not.toHaveBeenCalled();
  });

  it('schedules notification with single order', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ II', subtitle: 'Wiener Schnitzel', approved: true },
    ]);

    await checkDailyReminder();

    expect(mockScheduleDailyReminderNotification).toHaveBeenCalledWith(
      11, 0, 'MENÜ II \u2014 Wiener Schnitzel'
    );
  });

  it('schedules notification with multiple orders', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ II', subtitle: 'Wiener Schnitzel', approved: true },
      { positionId: '2', eatingCycleId: 'e2', date: new Date(2026, 1, 25), title: 'SUPPE & SALAT', subtitle: 'Tomatensuppe', approved: true },
    ]);

    await checkDailyReminder();

    expect(mockScheduleDailyReminderNotification).toHaveBeenCalledWith(
      11, 0, 'MENÜ II \u2014 Wiener Schnitzel\nSUPPE & SALAT \u2014 Tomatensuppe'
    );
  });

  it('marks sent date after scheduling notification', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Test', approved: true },
    ]);

    await checkDailyReminder();
    // Second call should not schedule again
    await checkDailyReminder();

    expect(mockScheduleDailyReminderNotification).toHaveBeenCalledTimes(1);
  });

  it('schedules again on a new day', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    await setReminderSentDate('2026-02-24'); // yesterday

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Test', approved: true },
    ]);

    await checkDailyReminder();

    expect(mockScheduleDailyReminderNotification).toHaveBeenCalledTimes(1);
  });

  it('schedules regardless of current time (no time window)', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(6 * 60); // 6:00 AM — far from 11:00

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Test', approved: true },
    ]);

    await checkDailyReminder();

    // Should still schedule — the scheduling function handles timing
    expect(mockScheduleDailyReminderNotification).toHaveBeenCalledWith(11, 0, 'MENÜ I \u2014 Test');
  });

  it('omits subtitle separator when subtitle is empty', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: '', approved: true },
    ]);

    await checkDailyReminder();

    expect(mockScheduleDailyReminderNotification).toHaveBeenCalledWith(11, 0, 'MENÜ I');
  });

  it('omits subtitle separator when subtitle is undefined', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', approved: true },
    ]);

    await checkDailyReminder();

    expect(mockScheduleDailyReminderNotification).toHaveBeenCalledWith(11, 0, 'MENÜ I');
  });
});

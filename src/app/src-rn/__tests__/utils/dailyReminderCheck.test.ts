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

jest.mock('expo-notifications', () => ({
  scheduleNotificationAsync: jest.fn(),
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

import * as Notifications from 'expo-notifications';
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
  mockViennaMinutes.mockReturnValue(0); // default: midnight, outside any reminder window
});

describe('checkDailyReminder', () => {
  it('does nothing when reminder is disabled', async () => {
    await setReminderEnabled(false);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });

  it('does nothing when no time is configured', async () => {
    await setReminderEnabled(true);
    mockViennaMinutes.mockReturnValue(11 * 60);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });

  it('does nothing when outside the ±15 min time window', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(9 * 60); // 9:00, way before 11:00

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Schnitzel', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });

  it('does nothing when no orders for today', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 26), title: 'MENÜ I', subtitle: 'Schnitzel', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });

  it('does nothing when already sent today', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    await setReminderSentDate('2026-02-25');
    mockViennaMinutes.mockReturnValue(11 * 60);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Schnitzel', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });

  it('sends notification with single order', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ II', subtitle: 'Wiener Schnitzel', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledWith({
      content: {
        title: 'Deine Bestellung heute',
        body: 'MENÜ II \u2014 Wiener Schnitzel',
        sound: 'default',
        data: { screen: '/(tabs)/orders' },
      },
      trigger: null,
    });
  });

  it('sends notification with multiple orders', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60 + 5); // 11:05, within window

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ II', subtitle: 'Wiener Schnitzel', approved: true },
      { positionId: '2', eatingCycleId: 'e2', date: new Date(2026, 1, 25), title: 'SUPPE & SALAT', subtitle: 'Tomatensuppe', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledWith({
      content: {
        title: 'Deine Bestellung heute',
        body: 'MENÜ II \u2014 Wiener Schnitzel\nSUPPE & SALAT \u2014 Tomatensuppe',
        sound: 'default',
        data: { screen: '/(tabs)/orders' },
      },
      trigger: null,
    });
  });

  it('marks sent date after firing notification', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Test', approved: true },
    ]);

    await checkDailyReminder();
    // Second call should not send again
    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledTimes(1);
  });

  it('sends again on a new day', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    await setReminderSentDate('2026-02-24'); // yesterday
    mockViennaMinutes.mockReturnValue(11 * 60);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Test', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledTimes(1);
  });

  it('fires at boundary of ±15 min window', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60 + 15); // exactly +15 min

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Test', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledTimes(1);
  });

  it('omits subtitle separator when subtitle is empty', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: '', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledWith({
      content: {
        title: 'Deine Bestellung heute',
        body: 'MENÜ I',
        sound: 'default',
        data: { screen: '/(tabs)/orders' },
      },
      trigger: null,
    });
  });

  it('does not fire just outside ±15 min window', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60 + 16); // +16 min, outside

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Test', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });
});

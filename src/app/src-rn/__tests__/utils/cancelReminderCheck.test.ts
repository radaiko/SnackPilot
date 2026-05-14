const mockScheduleCancelReminderNotification = jest.fn().mockResolvedValue(true);
const mockCancelCancelReminderNotification = jest.fn().mockResolvedValue(undefined);

jest.mock('../../utils/notificationService', () => ({
  scheduleCancelReminderNotification: (...args: any[]) => mockScheduleCancelReminderNotification(...args),
  cancelCancelReminderNotification: (...args: any[]) => mockCancelCancelReminderNotification(...args),
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

jest.mock('../../store/locationStore', () => {
  let isAtCompany = false;
  return {
    useLocationStore: {
      getState: () => ({ isAtCompany }),
      __setIsAtCompany: (v: boolean) => { isAtCompany = v; },
    },
  };
});

jest.mock('../../utils/dateUtils', () => ({
  viennaToday: jest.fn(),
  isSameDay: jest.fn((a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  ),
}));

import { useOrderStore } from '../../store/orderStore';
import { useLocationStore } from '../../store/locationStore';
import { viennaToday } from '../../utils/dateUtils';
import { checkCancelReminder } from '../../utils/cancelReminderCheck';

const mockViennaToday = viennaToday as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
  (useOrderStore as any).__setOrders([]);
  (useLocationStore as any).__setIsAtCompany(false);
  mockViennaToday.mockReturnValue(new Date(2026, 1, 25));
});

describe('checkCancelReminder', () => {
  it('cancels when user is at company', async () => {
    (useLocationStore as any).__setIsAtCompany(true);
    (useOrderStore as any).__setOrders([
      { positionId: '1', date: new Date(2026, 1, 25), title: 'MENÜ I' },
    ]);

    await checkCancelReminder();

    expect(mockCancelCancelReminderNotification).toHaveBeenCalled();
    expect(mockScheduleCancelReminderNotification).not.toHaveBeenCalled();
  });

  it('cancels when no order for today', async () => {
    (useLocationStore as any).__setIsAtCompany(false);
    (useOrderStore as any).__setOrders([
      { positionId: '1', date: new Date(2026, 1, 26), title: 'MENÜ I' },
    ]);

    await checkCancelReminder();

    expect(mockCancelCancelReminderNotification).toHaveBeenCalled();
    expect(mockScheduleCancelReminderNotification).not.toHaveBeenCalled();
  });

  it('schedules when not at company and has order today', async () => {
    (useLocationStore as any).__setIsAtCompany(false);
    (useOrderStore as any).__setOrders([
      { positionId: '1', date: new Date(2026, 1, 25), title: 'MENÜ I' },
    ]);

    await checkCancelReminder();

    expect(mockScheduleCancelReminderNotification).toHaveBeenCalled();
    expect(mockCancelCancelReminderNotification).not.toHaveBeenCalled();
  });
});

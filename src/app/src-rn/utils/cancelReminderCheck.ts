import { useOrderStore } from '../store/orderStore';
import { useLocationStore } from '../store/locationStore';
import { viennaToday, isSameDay } from './dateUtils';
import {
  scheduleCancelReminderNotification,
  cancelCancelReminderNotification,
} from './notificationService';

/**
 * Schedule or cancel the cancel-reminder notification.
 * Fires at 08:45 when user has an order today but is not at the office.
 *
 * Cancellation paths:
 * - User is at company (geofence Enter) → cancel
 * - No order for today → cancel
 *
 * Called from:
 * - orderStore.fetchOrders (foreground refresh)
 * - BACKGROUND_ORDER_SYNC_TASK (uses cached orders + last known location)
 * - Geofence Enter / Exit task handlers
 */
export async function checkCancelReminder(): Promise<void> {
  const { isAtCompany } = useLocationStore.getState();
  const orders = useOrderStore.getState().orders;
  const today = viennaToday();
  const hasOrderToday = orders.some((o) => isSameDay(o.date, today));

  if (isAtCompany || !hasOrderToday) {
    try {
      await cancelCancelReminderNotification();
    } catch { /* may not exist */ }
    return;
  }

  await scheduleCancelReminderNotification();
}

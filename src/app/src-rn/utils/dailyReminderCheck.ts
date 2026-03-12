import { useOrderStore } from '../store/orderStore';
import { viennaToday, isSameDay, localDateKey } from './dateUtils';
import {
  getReminderEnabled,
  getReminderTime,
  getReminderSentDate,
  setReminderSentDate,
} from './reminderStorage';
import { appendLogEntry } from './notificationLogStorage';
import {
  scheduleDailyReminderNotification,
  cancelDailyReminderNotification,
} from './notificationService';

/**
 * Schedule or cancel the daily order reminder notification.
 * Called from BACKGROUND_ORDER_SYNC_TASK and after orders are fetched.
 *
 * Instead of relying on background task timing (±15 min window),
 * this schedules a local notification at the configured time,
 * which iOS delivers reliably.
 *
 * Guards:
 * 1. Reminder must be enabled
 * 2. Time must be configured
 * 3. There must be orders for today
 * 4. Notification must not have been sent today already
 */
export async function checkDailyReminder(): Promise<void> {
  await appendLogEntry('daily-reminder', 'info', 'check_start');

  const enabled = await getReminderEnabled();
  if (!enabled) {
    await appendLogEntry('daily-reminder', 'guard', 'disabled');
    return;
  }

  const time = await getReminderTime();
  if (!time) {
    await appendLogEntry('daily-reminder', 'guard', 'no_time_configured');
    return;
  }

  const today = viennaToday();
  const todayKey = localDateKey(today);

  const sentDate = await getReminderSentDate();
  if (sentDate === todayKey) {
    await appendLogEntry('daily-reminder', 'guard', 'already_sent_today',
      `sentDate=${sentDate}`);
    return;
  }

  const orders = useOrderStore.getState().orders;
  const todayOrders = orders.filter((o) => isSameDay(o.date, today));
  if (todayOrders.length === 0) {
    // No orders for today — cancel any previously scheduled reminder
    try {
      await cancelDailyReminderNotification();
    } catch { /* may not exist */ }
    await appendLogEntry('daily-reminder', 'guard', 'no_orders_today',
      `date=${todayKey} totalOrders=${orders.length}`);
    return;
  }

  const body = todayOrders
    .map((o) => (o.subtitle ? `${o.title} \u2014 ${o.subtitle}` : o.title))
    .join('\n');

  await scheduleDailyReminderNotification(time.hour, time.minute, body);
  await setReminderSentDate(todayKey);
  await appendLogEntry('daily-reminder', 'notification', 'scheduled',
    `date=${todayKey} orderCount=${todayOrders.length} targetTime=${time.hour}:${time.minute}`);
}

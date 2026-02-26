import * as Notifications from 'expo-notifications';
import { useOrderStore } from '../store/orderStore';
import { viennaMinutes, viennaToday, isSameDay, localDateKey } from './dateUtils';
import {
  getReminderEnabled,
  getReminderTime,
  getReminderSentDate,
  setReminderSentDate,
} from './reminderStorage';
import { appendLogEntry } from './notificationLogStorage';

/**
 * Check if a daily order reminder notification should fire.
 * Called from BACKGROUND_ORDER_SYNC_TASK.
 *
 * Guards:
 * 1. Reminder must be enabled
 * 2. Time must be configured
 * 3. Current Vienna time must be within ±15 min of configured time
 * 4. There must be orders for today
 * 5. Notification must not have been sent today already
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

  const targetMinutes = time.hour * 60 + time.minute;
  const currentMinutes = viennaMinutes();
  const delta = Math.abs(currentMinutes - targetMinutes);
  if (delta > 15) {
    await appendLogEntry('daily-reminder', 'guard', 'time_guard_fail',
      `currentMin=${currentMinutes} targetMin=${targetMinutes} delta=${delta}`);
    return;
  }
  await appendLogEntry('daily-reminder', 'guard', 'time_guard_pass',
    `currentMin=${currentMinutes} targetMin=${targetMinutes}`);

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
    await appendLogEntry('daily-reminder', 'guard', 'no_orders_today',
      `date=${todayKey} totalOrders=${orders.length}`);
    return;
  }

  const body = todayOrders
    .map((o) => (o.subtitle ? `${o.title} \u2014 ${o.subtitle}` : o.title))
    .join('\n');

  await Notifications.scheduleNotificationAsync({
    content: {
      title: 'Deine Bestellung heute',
      body,
      sound: 'default',
      data: { screen: '/(tabs)/orders' },
    },
    trigger: null,
  });

  await setReminderSentDate(todayKey);
  await appendLogEntry('daily-reminder', 'notification', 'fired',
    `date=${todayKey} orderCount=${todayOrders.length}`);
}

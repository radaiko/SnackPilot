import * as Notifications from 'expo-notifications';
import { useOrderStore } from '../store/orderStore';
import { viennaMinutes, viennaToday, isSameDay, localDateKey } from './dateUtils';
import {
  getReminderEnabled,
  getReminderTime,
  getReminderSentDate,
  setReminderSentDate,
} from './reminderStorage';

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
  const enabled = await getReminderEnabled();
  if (!enabled) return;

  const time = await getReminderTime();
  if (!time) return;

  const targetMinutes = time.hour * 60 + time.minute;
  const currentMinutes = viennaMinutes();
  if (Math.abs(currentMinutes - targetMinutes) > 15) return;

  const today = viennaToday();
  const todayKey = localDateKey(today);

  const sentDate = await getReminderSentDate();
  if (sentDate === todayKey) return;

  const orders = useOrderStore.getState().orders;
  const todayOrders = orders.filter((o) => isSameDay(o.date, today));
  if (todayOrders.length === 0) return;

  const body = todayOrders
    .map((o) => (o.subtitle ? `${o.title} \u2014 ${o.subtitle}` : o.title))
    .join('\n');

  await setReminderSentDate(todayKey);

  await Notifications.scheduleNotificationAsync({
    content: {
      title: 'Deine Bestellung heute',
      body,
      sound: 'default',
      data: { screen: '/(tabs)/orders' },
    },
    trigger: null,
  });
}

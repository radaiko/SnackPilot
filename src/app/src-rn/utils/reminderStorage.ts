import AsyncStorage from '@react-native-async-storage/async-storage';

const REMINDER_ENABLED_KEY = 'daily_reminder_enabled';
const REMINDER_HOUR_KEY = 'daily_reminder_hour';
const REMINDER_MINUTE_KEY = 'daily_reminder_minute';
const REMINDER_SENT_DATE_KEY = 'daily_reminder_sent_date';

export async function getReminderEnabled(): Promise<boolean> {
  const value = await AsyncStorage.getItem(REMINDER_ENABLED_KEY);
  return value === 'true';
}

export async function setReminderEnabled(enabled: boolean): Promise<void> {
  await AsyncStorage.setItem(REMINDER_ENABLED_KEY, String(enabled));
}

export async function getReminderTime(): Promise<{ hour: number; minute: number } | null> {
  const hour = await AsyncStorage.getItem(REMINDER_HOUR_KEY);
  const minute = await AsyncStorage.getItem(REMINDER_MINUTE_KEY);
  if (hour === null || minute === null) return null;
  return { hour: Number(hour), minute: Number(minute) };
}

export async function setReminderTime(hour: number, minute: number): Promise<void> {
  await AsyncStorage.setItem(REMINDER_HOUR_KEY, String(hour));
  await AsyncStorage.setItem(REMINDER_MINUTE_KEY, String(minute));
}

export async function getReminderSentDate(): Promise<string | null> {
  return AsyncStorage.getItem(REMINDER_SENT_DATE_KEY);
}

export async function setReminderSentDate(dateKey: string): Promise<void> {
  await AsyncStorage.setItem(REMINDER_SENT_DATE_KEY, dateKey);
}

import AsyncStorage from '@react-native-async-storage/async-storage';

const REMINDER_ENABLED_KEY = 'daily_reminder_enabled';
const REMINDER_TIME_KEY = 'daily_reminder_time';
const REMINDER_SENT_DATE_KEY = 'daily_reminder_sent_date';

export async function getReminderEnabled(): Promise<boolean> {
  const value = await AsyncStorage.getItem(REMINDER_ENABLED_KEY);
  return value === 'true';
}

export async function setReminderEnabled(enabled: boolean): Promise<void> {
  await AsyncStorage.setItem(REMINDER_ENABLED_KEY, String(enabled));
}

export async function getReminderTime(): Promise<{ hour: number; minute: number } | null> {
  const raw = await AsyncStorage.getItem(REMINDER_TIME_KEY);
  if (raw === null) return null;
  try {
    const parsed = JSON.parse(raw);
    if (
      parsed !== null &&
      typeof parsed === 'object' &&
      typeof parsed.hour === 'number' &&
      typeof parsed.minute === 'number'
    ) {
      return parsed as { hour: number; minute: number };
    }
    return null;
  } catch {
    return null;
  }
}

export async function setReminderTime(hour: number, minute: number): Promise<void> {
  await AsyncStorage.setItem(REMINDER_TIME_KEY, JSON.stringify({ hour, minute }));
}

export async function getReminderSentDate(): Promise<string | null> {
  return AsyncStorage.getItem(REMINDER_SENT_DATE_KEY);
}

export async function setReminderSentDate(dateString: string): Promise<void> {
  await AsyncStorage.setItem(REMINDER_SENT_DATE_KEY, dateString);
}

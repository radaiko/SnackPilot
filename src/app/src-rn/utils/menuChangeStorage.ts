import AsyncStorage from '@react-native-async-storage/async-storage';
import { serializeKnownMenus, deserializeKnownMenus } from './menuFingerprint';

const KNOWN_MENUS_KEY = 'known_menu_fingerprints';
const NOTIFICATION_SENT_KEY = 'menu_notification_sent';

export async function getKnownMenus(): Promise<Map<string, string>> {
  const json = await AsyncStorage.getItem(KNOWN_MENUS_KEY);
  return deserializeKnownMenus(json);
}

export async function setKnownMenus(map: Map<string, string>): Promise<void> {
  await AsyncStorage.setItem(KNOWN_MENUS_KEY, serializeKnownMenus(map));
}

export async function getNotificationSent(): Promise<boolean> {
  const value = await AsyncStorage.getItem(NOTIFICATION_SENT_KEY);
  return value === 'true';
}

export async function setNotificationSent(sent: boolean): Promise<void> {
  await AsyncStorage.setItem(NOTIFICATION_SENT_KEY, String(sent));
}

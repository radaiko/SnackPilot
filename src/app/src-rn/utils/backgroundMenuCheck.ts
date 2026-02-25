import * as BackgroundFetch from 'expo-background-fetch';
import * as TaskManager from 'expo-task-manager';
import * as Notifications from 'expo-notifications';
import { Platform } from 'react-native';
import { GourmetApi } from '../api/gourmetApi';
import * as secureStorage from './secureStorage';
import { CREDENTIALS_KEY_USER, CREDENTIALS_KEY_PASS, isDemoCredentials } from './constants';
import { computeFingerprints, detectNewMenus } from './menuFingerprint';
import {
  getKnownMenus,
  setKnownMenus,
  getNotificationSent,
  setNotificationSent,
} from './menuChangeStorage';

const TASK_NAME = 'BACKGROUND_MENU_CHECK';

// Android notification channel
if (Platform.OS === 'android') {
  Notifications.setNotificationChannelAsync('menu-updates', {
    name: 'Neue Menüs',
    importance: Notifications.AndroidImportance.DEFAULT,
  });
}

/**
 * The background task. Runs headlessly — no React, no hooks.
 * Logs in, fetches menus, compares fingerprints, fires notification if new.
 */
async function backgroundMenuCheckTask(): Promise<BackgroundFetch.BackgroundFetchResult> {
  try {
    // Read credentials
    const username = await secureStorage.getItem(CREDENTIALS_KEY_USER);
    const password = await secureStorage.getItem(CREDENTIALS_KEY_PASS);
    if (!username || !password) {
      return BackgroundFetch.BackgroundFetchResult.NoData;
    }

    // Skip background check for demo credentials — they are fake and
    // must never be sent to the live Gourmet server.
    if (isDemoCredentials(username, password)) {
      return BackgroundFetch.BackgroundFetchResult.NoData;
    }

    // Check if we already sent a notification for the current batch
    const alreadySent = await getNotificationSent();
    if (alreadySent) {
      return BackgroundFetch.BackgroundFetchResult.NoData;
    }

    // Login and fetch menus
    const api = new GourmetApi();
    await api.login(username, password);
    const items = await api.getMenus();

    // Compare fingerprints
    const currentFingerprints = computeFingerprints(items);
    const knownMenus = await getKnownMenus();
    const hasNew = detectNewMenus(currentFingerprints, knownMenus);

    if (hasNew) {
      await Notifications.scheduleNotificationAsync({
        content: {
          title: 'Neue Menüs verfügbar',
          body: 'Es gibt neue Menüs. Öffne SnackPilot um sie anzusehen.',
          data: { screen: '/(tabs)' },
          ...(Platform.OS === 'android' ? { channelId: 'menu-updates' } : {}),
        },
        trigger: null, // Fire immediately
      });
      await setNotificationSent(true);
      await setKnownMenus(currentFingerprints);
      return BackgroundFetch.BackgroundFetchResult.NewData;
    }

    return BackgroundFetch.BackgroundFetchResult.NoData;
  } catch {
    return BackgroundFetch.BackgroundFetchResult.Failed;
  }
}

// Register the task at module load time (required by expo-task-manager)
TaskManager.defineTask(TASK_NAME, async () => {
  return backgroundMenuCheckTask();
});

/**
 * Register the background fetch task with the OS.
 * Call once from the root layout after login.
 * Safe to call multiple times — re-registration is a no-op if already registered.
 */
export async function registerBackgroundMenuCheck(): Promise<void> {
  if (Platform.OS === 'web') return;

  const isRegistered = await TaskManager.isTaskRegisteredAsync(TASK_NAME);
  if (isRegistered) return;

  await BackgroundFetch.registerTaskAsync(TASK_NAME, {
    minimumInterval: 15 * 60, // 15 minutes (OS may choose longer)
    stopOnTerminate: false,
    startOnBoot: true,
  });
}

/**
 * Request notification permissions. Call once after first login.
 * Returns true if permissions granted.
 */
export async function requestNotificationPermissions(): Promise<boolean> {
  if (Platform.OS === 'web') return false;

  const { status: existing } = await Notifications.getPermissionsAsync();
  if (existing === 'granted') return true;

  const { status } = await Notifications.requestPermissionsAsync();
  return status === 'granted';
}

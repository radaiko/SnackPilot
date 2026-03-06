import * as BackgroundTask from 'expo-background-task';
import * as TaskManager from 'expo-task-manager';
import * as Notifications from 'expo-notifications';
import { Platform } from 'react-native';
import { GourmetApi } from '../api/gourmetApi';
import * as secureStorage from './secureStorage';
import { CREDENTIALS_KEY_USER, CREDENTIALS_KEY_PASS, isDemoCredentials } from './constants';
import { computeFingerprints, detectNewMenus } from './menuFingerprint';
import { trackSignal } from './analytics';
import {
  getKnownMenus,
  setKnownMenus,
  getNotificationSent,
  setNotificationSent,
} from './menuChangeStorage';
import { appendLogEntry } from './notificationLogStorage';

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
async function backgroundMenuCheckTask(): Promise<BackgroundTask.BackgroundTaskResult> {
  try {
    await appendLogEntry('menu-check', 'info', 'task_start');

    // Read credentials
    const username = await secureStorage.getItem(CREDENTIALS_KEY_USER);
    const password = await secureStorage.getItem(CREDENTIALS_KEY_PASS);
    if (!username || !password) {
      await appendLogEntry('menu-check', 'guard', 'no_credentials');
      return BackgroundTask.BackgroundTaskResult.Success;
    }

    // Skip background check for demo credentials — they are fake and
    // must never be sent to the live Gourmet server.
    if (isDemoCredentials(username, password)) {
      await appendLogEntry('menu-check', 'guard', 'demo_credentials_skip');
      return BackgroundTask.BackgroundTaskResult.Success;
    }

    // Login and fetch menus
    await appendLogEntry('menu-check', 'info', 'login_start');
    const api = new GourmetApi();
    await api.login(username, password);
    await appendLogEntry('menu-check', 'info', 'login_success');

    const items = await api.getMenus();
    await appendLogEntry('menu-check', 'info', 'menus_fetched',
      `count=${items.length}`);

    // Compare fingerprints
    const currentFingerprints = computeFingerprints(items);
    const knownMenus = await getKnownMenus();
    const hasNew = detectNewMenus(currentFingerprints, knownMenus);

    // Only notify if menus changed AND we haven't already sent for this batch
    const alreadySent = await getNotificationSent();

    await appendLogEntry('menu-check', 'info', 'comparison_result',
      `hasNew=${hasNew} alreadySent=${alreadySent} currentCount=${currentFingerprints.size} knownCount=${knownMenus.size}`);

    if (hasNew && !alreadySent) {
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
      trackSignal('menu.newDetected');
      await appendLogEntry('menu-check', 'notification', 'fired',
        'new menus detected');
      return BackgroundTask.BackgroundTaskResult.Success;
    }

    await appendLogEntry('menu-check', 'guard', 'no_notification',
      `hasNew=${hasNew} alreadySent=${alreadySent}`);
    return BackgroundTask.BackgroundTaskResult.Success;
  } catch (e) {
    await appendLogEntry('menu-check', 'error', 'task_error',
      e instanceof Error ? e.message : String(e));
    return BackgroundTask.BackgroundTaskResult.Failed;
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

  await BackgroundTask.registerTaskAsync(TASK_NAME, {
    minimumInterval: 15, // 15 minutes (OS may choose longer)
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

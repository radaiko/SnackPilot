import { Platform } from 'react-native';
import * as TaskManager from 'expo-task-manager';
import * as Location from 'expo-location';
import * as Notifications from 'expo-notifications';
import { useLocationStore } from '../store/locationStore';
import { useOrderStore } from '../store/orderStore';
import { isSameDay, viennaMinutes, viennaToday } from './dateUtils';
import {
  GEOFENCE_TASK_NAME,
  BACKGROUND_ORDER_SYNC_TASK,
  NOTIFICATION_HOUR,
  NOTIFICATION_MINUTE,
} from './constants';

// Only define tasks on native platforms
if (Platform.OS !== 'web') {
  // --- Geofence task: update isAtCompany flag ---
  interface GeofencingTaskData {
    eventType: Location.GeofencingEventType;
    region: Location.LocationRegion;
  }

  TaskManager.defineTask<GeofencingTaskData>(
    GEOFENCE_TASK_NAME,
    ({ data, error }) => {
      if (error) return;
      const { eventType } = data;
      if (eventType === Location.GeofencingEventType.Enter) {
        useLocationStore.getState().setIsAtCompany(true);
      } else if (eventType === Location.GeofencingEventType.Exit) {
        useLocationStore.getState().setIsAtCompany(false);
      }
    }
  );

  // --- Background order sync task ---
  const BackgroundTask = require('expo-background-task');

  TaskManager.defineTask(BACKGROUND_ORDER_SYNC_TASK, async () => {
    try {
      // Only check if we have a company location configured
      if (!useLocationStore.getState().hasCompanyLocation()) {
        return BackgroundTask.BackgroundTaskResult.Success;
      }

      // Load cached orders (no network calls to avoid concurrent scraping)
      await useOrderStore.getState().loadCachedOrders();

      // Check if we should fire a notification
      await checkAndNotify();

      return BackgroundTask.BackgroundTaskResult.Success;
    } catch {
      return BackgroundTask.BackgroundTaskResult.Failed;
    }
  });
}

/**
 * Check order/location state and send notification if needed.
 * Only fires within a 30-minute window around the configured
 * notification time (NOTIFICATION_HOUR:NOTIFICATION_MINUTE) in Vienna timezone.
 */
async function checkAndNotify(): Promise<void> {
  const targetMinutes = NOTIFICATION_HOUR * 60 + NOTIFICATION_MINUTE;
  const currentMinutes = viennaMinutes();
  // Only fire within ±15 minutes of the target time
  if (Math.abs(currentMinutes - targetMinutes) > 15) return;

  const { isAtCompany } = useLocationStore.getState();
  const orders = useOrderStore.getState().orders;
  const today = viennaToday();
  const hasOrderToday = orders.some((o) => isSameDay(o.date, today));

  if (isAtCompany && !hasOrderToday) {
    await Notifications.scheduleNotificationAsync({
      content: {
        title: 'SnackPilot',
        body: 'Du bist im Büro, hast aber noch nicht bestellt!',
        sound: 'default',
      },
      trigger: null,
    });
  } else if (!isAtCompany && hasOrderToday) {
    await Notifications.scheduleNotificationAsync({
      content: {
        title: 'SnackPilot',
        body: 'Du hast heute bestellt, bist aber nicht im Büro!',
        sound: 'default',
      },
      trigger: null,
    });
  }
}

export { checkAndNotify };

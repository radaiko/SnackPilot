import { Platform } from 'react-native';
import * as TaskManager from 'expo-task-manager';
import * as Location from 'expo-location';
import * as Notifications from 'expo-notifications';
import { useLocationStore } from '../store/locationStore';
import { useOrderStore } from '../store/orderStore';
import { useAuthStore } from '../store/authStore';
import { isSameDay } from './dateUtils';
import {
  GEOFENCE_TASK_NAME,
  BACKGROUND_ORDER_SYNC_TASK,
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
      // Only sync if we have a company location configured
      if (!useLocationStore.getState().hasCompanyLocation()) {
        return BackgroundTask.BackgroundTaskResult.Success;
      }

      // Try to login if not authenticated
      const authState = useAuthStore.getState();
      if (authState.status !== 'authenticated') {
        await authState.loginWithSaved();
      }

      // Refresh orders
      if (useAuthStore.getState().status === 'authenticated') {
        await useOrderStore.getState().fetchOrders();
      }

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
 * Called from the background sync task.
 */
async function checkAndNotify(): Promise<void> {
  const { isAtCompany } = useLocationStore.getState();
  const orders = useOrderStore.getState().orders;
  const today = new Date();
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

import { Platform } from 'react-native';
import * as TaskManager from 'expo-task-manager';
import * as Location from 'expo-location';
import { useLocationStore } from '../store/locationStore';
import { useOrderStore } from '../store/orderStore';
import { isSameDay, viennaToday } from './dateUtils';
import { checkDailyReminder } from './dailyReminderCheck';
import { appendLogEntry } from './notificationLogStorage';
import {
  scheduleGeofenceNotification,
  cancelGeofenceNotification,
} from './notificationService';
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
    async ({ data, error }) => {
      if (error) {
        await appendLogEntry('geofence', 'error', 'task_error', error.message);
        return;
      }
      const { eventType } = data;
      if (eventType === Location.GeofencingEventType.Enter) {
        useLocationStore.getState().setIsAtCompany(true);
        await appendLogEntry('geofence', 'info', 'region_enter', 'isAtCompany=true');
        try {
          // Load cached orders first — on cold start the Zustand store is empty
          await useOrderStore.getState().loadCachedOrders();
          const orders = useOrderStore.getState().orders;
          const today = viennaToday();
          const hasOrderToday = orders.some((o) => isSameDay(o.date, today));

          if (hasOrderToday) {
            await appendLogEntry('geofence', 'guard', 'has_order_today',
              `ordersLoaded=${orders.length}`);
            return;
          }

          // Schedule notification for the configured time (or fire immediately if past)
          const scheduled = await scheduleGeofenceNotification();
          await appendLogEntry('geofence', 'info', 'notification_scheduled',
            `scheduled=${scheduled} ordersLoaded=${orders.length}`);
        } catch (e) {
          await appendLogEntry('geofence', 'error', 'enter_notify_error',
            e instanceof Error ? e.message : String(e));
        }
      } else if (eventType === Location.GeofencingEventType.Exit) {
        useLocationStore.getState().setIsAtCompany(false);
        await appendLogEntry('geofence', 'info', 'region_exit', 'isAtCompany=false');
        // Cancel any pending geofence notification — user left the office
        try {
          await cancelGeofenceNotification();
          await appendLogEntry('geofence', 'info', 'notification_cancelled', 'region_exit');
        } catch (e) {
          await appendLogEntry('geofence', 'error', 'exit_cancel_error',
            e instanceof Error ? e.message : String(e));
        }
      }
    }
  );

  // --- Background order sync task ---
  const BackgroundTask = require('expo-background-task');

  TaskManager.defineTask(BACKGROUND_ORDER_SYNC_TASK, async () => {
    try {
      await appendLogEntry('order-sync', 'info', 'task_start');

      // Load cached orders (no network calls to avoid concurrent scraping)
      await useOrderStore.getState().loadCachedOrders();

      // Daily order reminder check
      try {
        await checkDailyReminder();
      } catch (e) {
        await appendLogEntry('order-sync', 'error', 'reminder_check_error',
          e instanceof Error ? e.message : String(e));
      }

      await appendLogEntry('order-sync', 'info', 'task_complete');
      return BackgroundTask.BackgroundTaskResult.Success;
    } catch (e) {
      await appendLogEntry('order-sync', 'error', 'task_fatal',
        e instanceof Error ? e.message : String(e));
      return BackgroundTask.BackgroundTaskResult.Failed;
    }
  });
}

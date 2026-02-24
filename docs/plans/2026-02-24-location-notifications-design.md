# Location-Based Order Notifications

**Issue**: https://github.com/radaiko/SnackPilot/issues/25
**Date**: 2026-02-24
**Status**: Approved

## Overview

When the user saves their company location, the app registers a 500m geofence. At 8:45am Vienna time, it checks geofence state + cached order data and sends a local notification in German. Removing the saved location disables everything. Mobile only.

## Settings UI

In the Settings tab, a new "Standort-Benachrichtigungen" section:

- **No location saved**: Button "Aktuellen Standort als Firmenstandort setzen". Tapping requests location permission, captures current GPS, saves it, registers geofence.
- **Location saved**: Shows "Firmenstandort gesetzt" and a "Standort entfernen" button. Tapping clears saved location and unregisters geofence.

## Geofencing

- `expo-location` geofencing + `expo-task-manager`
- Single geofence: saved lat/lng, 500m radius
- Background task listens for ENTER/EXIT events, updates persisted `isAtCompany` flag in AsyncStorage

## Order Data Freshness

- `expo-background-fetch` refreshes order data periodically (~15 min, OS-controlled)
- Background fetch task: auto-login if needed, call `fetchOrders()`, cache to AsyncStorage
- Falls back to existing cache if background fetch hasn't run recently

## Notification Logic

Scheduled check at 8:45am Europe/Vienna daily:

| At company? | Ordered today? | Action |
|---|---|---|
| Yes | No | "Du bist im Büro, hast aber noch nicht bestellt!" |
| Yes | Yes | Nothing |
| No | Yes | "Du hast heute bestellt, bist aber nicht im Büro!" |
| No | No | Nothing |

Tapping either notification opens the app (no deep linking).

## New Dependencies

- `expo-location` — geofencing + one-shot location capture
- `expo-task-manager` — background geofence events + background fetch
- `expo-background-fetch` — periodic order data refresh
- `expo-notifications` — local notifications

## New Files

| File | Purpose |
|---|---|
| `src-rn/store/locationStore.ts` | Zustand store: saved company location, `isAtCompany` flag, persist to AsyncStorage |
| `src-rn/utils/notificationService.ts` | Register/unregister geofence, schedule 8:45am check, send notifications, background task definitions |

## Integration Points

- `app/_layout.tsx`: Initialize notification permissions + restore geofence on app start (if location saved)
- `app/(tabs)/settings.tsx`: Add "Standort-Benachrichtigungen" section
- `app.json` / `app.config.ts`: Add background modes (location, fetch, remote-notification) for iOS

## Constraints

- No map UI — captures current position only
- No custom time or radius settings (8:45am, 500m fixed)
- No desktop/web support — hidden on non-mobile platforms
- No modification to web scraping logic
- Location being set IS the toggle — no separate enable/disable switch

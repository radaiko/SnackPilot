# Daily Order Reminder Notification — Design

**Issue**: #31
**Date**: 2026-02-25

## Overview

A scheduled local notification that fires once daily at a user-configured time, showing what the user has ordered today. Piggybacks on the existing `BACKGROUND_ORDER_SYNC_TASK` infrastructure. Native-only (iOS + Android), web stubs for desktop.

## Decisions

- **MVP scope**: One-time local notification (not Live Activities / persistent widgets). Live Activities deferred to future iteration.
- **No default time**: User must explicitly enable and configure the reminder time in settings.
- **No order = no notification**: Silent skip when there's no order for today.
- **Reuse existing background task**: The `BACKGROUND_ORDER_SYNC_TASK` (~15 min interval) already runs and loads orders. Add a time-window check for the configured reminder time.
- **Time picker granularity**: 15-minute increments (e.g., 11:00, 11:15, 11:30, 11:45).
- **Multiple orders**: Single notification listing all orders for the day.
- **Tap action**: Opens the Orders tab.
- **Settings grouping**: Location notifications and this reminder share a unified "Benachrichtigungen" section.

## Data Flow

```
BACKGROUND_ORDER_SYNC_TASK runs (~15 min interval)
  -> read reminderEnabled + reminderHour + reminderMinute from AsyncStorage
  -> if disabled -> skip
  -> check viennaMinutes() within +/-15 min of configured time
  -> load cached orders (already done by existing task)
  -> filter for today's orders
  -> if no orders today -> skip
  -> if already sent today (dailyReminderSentDate === today) -> skip
  -> fire notification with all today's orders
  -> set dailyReminderSentDate = today
```

The `dailyReminderSentDate` resets naturally — when the date changes, today no longer matches the stored date.

## Storage

New AsyncStorage keys (in new `reminderStorage.ts`, following `menuChangeStorage.ts` pattern):

| Key | Type | Purpose |
|-----|------|---------|
| `daily_reminder_enabled` | `"true"` / `"false"` | Feature toggle |
| `daily_reminder_hour` | `"0"` - `"23"` | User-configured hour |
| `daily_reminder_minute` | `"0"` - `"45"` | User-configured minute (15-min increments) |
| `daily_reminder_sent_date` | `"YYYY-MM-DD"` | Last date notification was sent (prevents duplicates) |

## Notification Content

- **Title**: "Deine Bestellung heute"
- **Body**: All orders joined with newlines, e.g.:
  ```
  MENU II - Wiener Schnitzel
  SUPPE & SALAT - Tomatensuppe
  ```
- **Channel**: `order-reminders` (existing Android channel from location notifications)
- **Tap action**: Deep link to `/(tabs)/orders`

## Settings UI

The existing "Standort-Benachrichtigungen" section becomes "Benachrichtigungen" with two sub-features:

```
+-- Benachrichtigungen -------------------------+
|                                               |
|  Bestell-Erinnerung                           |
|  [Toggle: Ein/Aus]                            |
|  (when enabled:)                              |
|  Uhrzeit: [11:00 v] (15-min picker)          |
|                                               |
|  -- separator --                              |
|                                               |
|  Standort-Benachrichtigungen                  |
|  (existing location UI unchanged)             |
|                                               |
+-----------------------------------------------+
```

Enabling the toggle requests notification permissions if not already granted. Section is only shown on native (`isNative()`).

## Files to Create/Modify

| File | Action |
|------|--------|
| `src-rn/utils/reminderStorage.ts` | **New** — AsyncStorage helpers for reminder settings + sent flag |
| `src-rn/utils/notificationTasks.ts` | **Modify** — add daily reminder check to `BACKGROUND_ORDER_SYNC_TASK` handler |
| `src-rn/utils/constants.ts` | **Modify** — add default reminder constants if needed |
| `app/(tabs)/settings.tsx` | **Modify** — restructure into "Benachrichtigungen" section, add reminder toggle + time picker |
| `__tests__/utils/reminderStorage.test.ts` | **New** — tests for storage helpers |
| `__tests__/utils/notificationTasks.test.ts` | **Modify** — add test cases for reminder logic |

## Out of Scope

- No new background task registration (reuses existing)
- No new permissions beyond what's already requested
- No foreground/in-app reminder (background notification only)
- No Live Activities (deferred to future iteration)

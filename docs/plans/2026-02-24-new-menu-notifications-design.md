# New Menu Notifications — Design

**Issue:** [#13](https://github.com/radaiko/SnackPilot/issues/13) — Notification on new imported menus

**Goal:** Notify the user (iOS + Android) when the canteen imports new or changed menus, so they don't miss ordering.

**Architecture:** Background fetch periodically scrapes the menu page, compares against known menu fingerprints, and fires a local push notification if new/changed menus are detected. Foreground detection covers the case where background fetch hasn't run yet, showing an in-app toast instead.

**Tech Stack:** `expo-notifications`, `expo-background-fetch`, `expo-task-manager`

---

## Detection Logic

Track known menus as a `Map<string, string>` in AsyncStorage:
- Key: menu ID
- Value: content fingerprint (`title|subtitle|allergens` concatenated)

On each menu fetch (background or foreground):
1. Compute fingerprints for all fetched menus
2. Compare against stored `knownMenus` map
3. New ID → new menu. Same ID but different fingerprint → changed menu
4. If any new/changed AND `notificationSent === false` → notify

## Notification Behavior

- **One notification per batch.** Once fired, `notificationSent = true`. No repeated notifications.
- **Background:** OS local push notification (generic message: "New menus have been added. Open SnackPilot to check them out.")
- **Foreground:** In-app toast/snackbar at top of Menus screen
- **Acknowledgment:** Opening the Menus tab updates `knownMenus` with all current fingerprints and resets `notificationSent = false`
- Tapping the OS notification deep-links to the Menus tab

## Background Fetch

- Registered at module load time via `TaskManager.defineTask()`
- OS controls frequency (15 min to hours, depending on user app usage patterns)
- Runs headless JS: reads credentials from secure storage → logs in → fetches menus → compares → notifies
- If user is not logged in, task skips silently

## Notification Permissions

- Requested after first successful login (from root layout)
- If denied, detection still runs but no notification fires. No re-prompting.
- No user-facing settings toggle (YAGNI)

## Foreground Detection

- Runs on Menus tab focus, after `fetchMenus()` / `refreshAvailability()` completes
- Same comparison logic as background
- If new/changed menus AND `notificationSent === false` → show toast
- Covers the case where background fetch hasn't run or the user opens the app first

## Data Flow

```
BACKGROUND:
  OS wakes app → backgroundMenuCheck task
    → read credentials (secure storage)
    → login to Gourmet
    → fetch menus
    → compute fingerprints
    → compare against knownMenus (AsyncStorage)
    → new/changed AND notificationSent === false?
        → yes: fire local notification, set notificationSent = true
        → no: done

FOREGROUND:
  Menus tab focus → fetchMenus()
    → compute fingerprints
    → compare against knownMenus (AsyncStorage)
    → new/changed AND notificationSent === false?
        → yes: show toast, set notificationSent = true
        → no: done
    → update knownMenus with all current fingerprints
    → reset notificationSent = false
```

## File Changes

**New files:**
- `src/app/src-rn/utils/backgroundMenuCheck.ts` — background task definition, registration, fingerprint comparison, AsyncStorage helpers for `knownMenus` and `notificationSent`
- `src/app/src-rn/components/NewMenuToast.tsx` — lightweight toast/snackbar component

**Modified files:**
- `src/app/app/_layout.tsx` — call `registerBackgroundMenuCheck()`, request notification permissions after login
- `src/app/app/(tabs)/index.tsx` — foreground detection after menu fetch, show toast, update `knownMenus` on tab focus
- `src/app/src-rn/store/menuStore.ts` — expose helper to compute menu fingerprints
- `src/app/app.json` — add `expo-notifications`, `expo-background-fetch`, `expo-task-manager` plugins
- `src/app/package.json` — new dependencies

## Constraints

- Web scraping logic is NOT modified — reuses existing `gourmetApi` and `gourmetClient`
- No external server or push service needed — all local notifications
- Mobile only (iOS + Android) — desktop and web are unaffected
- Background fetch timing is OS-controlled and not guaranteed — foreground detection is the safety net

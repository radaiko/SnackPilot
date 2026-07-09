# Platform services (storage, background, permissions, credential takeover)

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

This doc covers the platform-integration layer: secure credential storage (and the
exact on-disk format needed for in-place **credential takeover** from v1), unencrypted
key-value storage, background task registration, notification permissions/channels,
location permissions, localization usage, and the `app.json` config (bundle identifiers,
OS permissions, plugins) that v2 must reproduce.

**Cross-references — do not restate here:**
- Notification *content, scheduling thresholds, and reminder logic*: see
  `03-features/notifications-location.md`, `notifications-daily-reminder.md`,
  `notifications-cancel-reminder.md`, `notifications-new-menu.md`, `notification-log.md`.
- Cache serialization formats and cache keys' *semantics*: see `03-features/caching.md`.
- Login state machines / auth flow: see `03-features/settings.md`; HTTP request specs for
  Gourmet and Ventopay: `01-gourmet-scraping.md`, `02-ventopay-scraping.md`.
- Analytics payloads (the only consumer of localization): `03-features/analytics.md`.

**Platform scope (v2):** v1 is Expo/React Native across ios, android, desktop (Tauri),
and web. **Desktop and web are dropped in v2.** This doc documents the native (iOS/Android)
behavior. Where a v1 file has a `.web.ts` sibling or a desktop/web code branch, a
"Dropped in v2" note records what is removed.

---

## 1. Credential secure storage — CRITICAL for takeover

### 1.1 What is stored, and under which keys

v1 stores exactly **four** credential strings in the platform secure store. Nothing else
uses secure storage (verified: no other `SecureStore.*` call exists outside the wrapper).

| Secure-store key (string) | Value | Written by | (v1 provenance) |
|---|---|---|---|
| `gourmet_username` | Gourmet/Kantine username (plaintext) | Gourmet auth store `saveCredentials` | (v1: src/app/src-rn/utils/constants.ts:12 `CREDENTIALS_KEY_USER`) |
| `gourmet_password` | Gourmet/Kantine password (plaintext) | Gourmet auth store `saveCredentials` | (v1: src/app/src-rn/utils/constants.ts:13 `CREDENTIALS_KEY_PASS`) |
| `ventopay_username` | Ventopay/Automaten username (plaintext) | Ventopay auth store `saveCredentials` | (v1: src/app/src-rn/store/ventopayAuthStore.ts:8) |
| `ventopay_password` | Ventopay/Automaten password (plaintext) | Ventopay auth store `saveCredentials` | (v1: src/app/src-rn/store/ventopayAuthStore.ts:9) |

The values are the raw username/password strings as typed by the user (no hashing, no
encoding at the application layer). Gourmet keys are named constants in `constants.ts`;
the Ventopay keys are **string literals inside the Ventopay auth store** (not the shared
constants file) — copy them verbatim.

### 1.2 Application-layer wrapper behavior (v1: src/app/src-rn/utils/secureStorage.ts)

All access goes through a thin wrapper with a fixed options object:

```
STORE_OPTIONS = { keychainAccessible: AFTER_FIRST_UNLOCK }
```
(v1: secureStorage.ts:6-8). `keychainAccessible` is set to `AFTER_FIRST_UNLOCK`; no
`keychainService`, no `requireAuthentication`, no `accessGroup`, no `authenticationPrompt`
is set. So those take their library defaults (see §1.3 / §1.4). The comment explains the
choice: `AFTER_FIRST_UNLOCK` "allows background tasks to read credentials even when the
device is locked (after it was unlocked at least once since boot)" — this is **required**
for the background **menu-check** task (§4.3), the sole background job that reads
credentials while locked (via `secureStorage.getItem`, backgroundMenuCheck.ts:37-38). The
order-sync task reads cached orders from AsyncStorage, not credentials (explicit "no
network calls" comment, notificationTasks.ts:86), so it does not depend on keychain
accessibility.

Wrapper operations:
- `getItem(key)` → read with `STORE_OPTIONS`.
- `setItem(key, value)` → write with `STORE_OPTIONS`.
- `deleteItem(key)` → delete with `STORE_OPTIONS`.

Auth-store operations that call the wrapper (v1: authStore.ts:73-88, ventopayAuthStore.ts:71-86):
- `saveCredentials(username, password)` → `setItem(<user_key>, username)` then `setItem(<pass_key>, password)`.
- `getSavedCredentials()` → read both; **returns `null` if either is missing/empty**.
- `clearCredentials()` → delete both keys.

Value semantics: `getSavedCredentials` treats empty string as absent (`if (!username || !password) return null`). Login-with-saved: if no creds, sets status `no_credentials` and returns false without any network call (v1: authStore.ts:55-62).

### 1.3 Keychain-accessibility migration (one-time, foreground)

`migrateKeychainAccessibility(keys)` (v1: secureStorage.ts:17-36) upgrades pre-existing
items from the old default accessibility (`WHEN_UNLOCKED`) to `AFTER_FIRST_UNLOCK`:

1. Read guard flag from **unencrypted** storage key `@secureStorage:migratedAfterFirstUnlock`
   (AsyncStorage). If it equals `"1"`, return immediately (runs once).
2. For each key: read it **with no options** (matches the old default accessibility). If a
   value is present, `deleteItemAsync(key)` then `setItemAsync(key, value, STORE_OPTIONS)`
   (re-save with `AFTER_FIRST_UNLOCK`). Errors are swallowed (item may not exist).
3. Set `@secureStorage:migratedAfterFirstUnlock = "1"`.

It is invoked on app start (native only) with exactly these keys, in this order, before
attempting saved-login (v1: app/_layout.tsx:38-46):
```
['gourmet_username', 'gourmet_password', 'ventopay_username', 'ventopay_password']
```
The comment notes it "Must be called while the app is in the foreground."

**v2 note:** v2 owns its keychain items from day one and can write `AFTER_FIRST_UNLOCK`
directly, so a fresh v2 install needs no migration. This routine matters only for the
**takeover** path where v2 reads items a v1 install left behind — v2 must be prepared to
find items with either `WHEN_UNLOCKED` or `AFTER_FIRST_UNLOCK` accessibility (accessibility
is not part of the lookup query on either platform, so it does not affect reads).

**Dropped in v2:** `secureStorage.web.ts` — the web build backs the same API with
`localStorage` and `migrateKeychainAccessibility` is a no-op (v1: secureStorage.web.ts:49).

---

### 1.4 iOS Keychain item format (expo-secure-store 55.0.15 native, Swift)

Source: `node_modules/expo-secure-store/ios/SecureStoreModule.swift` (not in git; the
appendix records this dependency). These are the **exact coordinates** of every v1
credential item, needed to read them from a v2 Swift app running under the same app
identity.

Item attributes written per credential (v1: SecureStoreModule.swift `query`/`set` at :90-192):

| Keychain attribute | Value for v1 items | Notes |
|---|---|---|
| `kSecClass` | `kSecClassGenericPassword` | Generic password item (:181) |
| `kSecAttrService` | `"app:no-auth"` | `keychainService ?? "app"` (v1 never sets it → `"app"`), then `":no-auth"` appended because `requireAuthentication == false` (:173-176) |
| `kSecAttrAccount` | UTF-8 bytes of the key string, e.g. `Data("gourmet_username".utf8)` | (:178, :184) |
| `kSecAttrGeneric` | Same UTF-8 bytes of the key string | (:178, :183) |
| `kSecAttrAccessible` | `kSecAttrAccessibleAfterFirstUnlock` | From `keychainAccessible = AFTER_FIRST_UNLOCK` (enum raw value `0`) mapped at :196-197 |
| `kSecAttrAccessGroup` | *(absent)* | `accessGroup` is nil → item lives in the app's **default keychain access group** (`$(AppIdentifierPrefix)$(bundleIdentifier)`) |
| `kSecValueData` | Credential string as UTF-8 `Data`, **plaintext** | Keychain encrypts at rest; no app-level crypto (:92-93) |

Key facts for takeover:
- The **service** string is literally `app:no-auth` (not the bundle id, not `SnackPilot`).
  This is because v1 never passes `keychainService`, so the library default `"app"` is used
  and the `:no-auth` suffix is appended for non-authenticated items.
- The **account** and **generic** attributes are the UTF-8 bytes of the JS key name
  (`gourmet_username`, `gourmet_password`, `ventopay_username`, `ventopay_password`).
- The value is stored **plaintext** in the keychain data blob (the OS keychain provides
  encryption); a v2 reader gets the credential back directly with `.utf8` decoding.
- Because no access group is set, the item is in the default access group. A v2 Swift app
  can read it **only if it runs under the same bundle identifier + team prefix** as v1
  (`dev.radaiko.gourmetclient`, see §7) — i.e. an in-place App Store update, not a new app
  record. (See openQuestions on team/prefix.)

Read fallback chain the library uses (v2 should mirror it to be safe): `get` searches, in
order, service `app:no-auth`, then `app:auth`, then legacy `app` (no suffix), returning the
first hit (v1: SecureStoreModule.swift:74-86 via `searchKeyChain` with `requireAuthentication`
= false, then true, then nil). v1 only ever *writes* `app:no-auth`, but a robust v2 reader
should try all three service names with `kSecClass = kSecClassGenericPassword`,
`kSecAttrAccount = <utf8 key>`, `kSecMatchLimit = kSecMatchLimitOne`, `kSecReturnData = true`.

Delete semantics (for reference): `deleteValueWithKeyAsync` issues three `SecItemDelete`
calls for the legacy (`app`), `app:auth`, and `app:no-auth` services (:44-50).

**Not used by v1 (documented so v2 knows it can ignore them):** `requireAuthentication`
(biometric-gated items using `SecAccessControlCreateWithFlags(..., .biometryCurrentSet, ...)`
and requiring `NSFaceIDUsageDescription`), `authenticationPrompt`, and `accessGroup` are all
supported by the library but never exercised — v1 stores unauthenticated items only.

---

### 1.5 Android storage format (expo-secure-store 55.0.15 native, Kotlin)

Source: `node_modules/expo-secure-store/android/.../securestore/`. On Android the value is
**encrypted with an AES-256-GCM key held in the AndroidKeyStore**, and the ciphertext +
metadata are stored as a JSON string in a private `SharedPreferences` file.

#### SharedPreferences file
- File name: `SecureStore` → on-device path `/data/data/dev.radaiko.gourmetclient/shared_prefs/SecureStore.xml`, mode `MODE_PRIVATE` (v1: SecureStoreModule.kt:366-368, `SHARED_PREFERENCES_NAME = "SecureStore"`).

#### Entry key format
Each item's SharedPreferences entry key is `"$keychainService-$key"` (v1: SecureStoreModule.kt:374-376 `createKeychainAwareKey`). Since v1 never sets `keychainService`, it defaults to `DEFAULT_KEYSTORE_ALIAS = "key_v1"` (v1: SecureStoreOptions.kt:10, SecureStoreModule.kt:385). Therefore the four entry keys are:
```
key_v1-gourmet_username
key_v1-gourmet_password
key_v1-ventopay_username
key_v1-ventopay_password
```
(Reads also fall back to a bare `key` for legacy items, but v1 only writes the prefixed form — v1: SecureStoreModule.kt:92-97.)

#### Entry value (JSON object, stored as a string)
Written by `AESEncryptor.createEncryptedItemWithCipher` + `saveEncryptedItem`
(v1: AESEncryptor.kt:93-108, SecureStoreModule.kt:227-241). JSON properties:

| JSON key | Meaning | Value / format |
|---|---|---|
| `ct` | Ciphertext | Base64 **NO_WRAP**, AES-256-GCM output of the UTF-8 plaintext (AESEncryptor.kt:100, `CIPHERTEXT_PROPERTY = "ct"` :139) |
| `iv` | GCM initialization vector | Base64 **NO_WRAP**, fresh random IV per write (AESEncryptor.kt:101, `IV_PROPERTY = "iv"` :140) |
| `tlen` | GCM authentication tag length, **bits** | Integer from the cipher's `GCMParameterSpec.tLen` (Android default 128); reads reject `< 96` (AESEncryptor.kt:102, 120-128; `GCM_AUTHENTICATION_TAG_LENGTH_PROPERTY = "tlen"` :141) |
| `scheme` | Encryption scheme id | Literal string `"aes"` (SecureStoreModule.kt:205, `AESEncryptor.NAME = "aes"` :136) |
| `usesKeystoreSuffix` | Alias uses the extended suffix form | `true` (SecureStoreModule.kt:229, `USES_KEYSTORE_SUFFIX_PROPERTY = "usesKeystoreSuffix"` :384) |
| `keystoreAlias` | The keychainService this entry belongs to | `"key_v1"` (SecureStoreModule.kt:232, `KEYSTORE_ALIAS_PROPERTY = "keystoreAlias"` :383) |
| `requireAuthentication` | Whether the entry is biometric-gated | `false` (SecureStoreModule.kt:233, `AuthenticationHelper.REQUIRE_AUTHENTICATION_PROPERTY = "requireAuthentication"`) |

Example shape (values illustrative):
```json
{"ct":"<base64>","iv":"<base64>","tlen":128,"scheme":"aes","usesKeystoreSuffix":true,"keystoreAlias":"key_v1","requireAuthentication":false}
```

#### KeyStore key (AndroidKeyStore)
- Provider: `AndroidKeyStore` (`KEYSTORE_PROVIDER`, SecureStoreModule.kt:381).
- Key alias (the actual symmetric key protecting the ciphertext):
  `getExtendedKeyStoreAlias = "<AES_CIPHER>:<keychainService>:<suffix>"`. With
  `AES_CIPHER = "AES/GCM/NoPadding"`, `keychainService = "key_v1"`, and (because
  `requireAuthentication == false`) `suffix = UNAUTHENTICATED_KEYSTORE_SUFFIX =
  "keystoreUnauthenticated"`, the alias is:
  ```
  AES/GCM/NoPadding:key_v1:keystoreUnauthenticated
  ```
  (v1: AESEncryptor.kt:34-51; SecureStoreModule.kt:386-387.)
- Key generation params (`KeyGenParameterSpec`, AESEncryptor.kt:55-73):
  - Purposes: `ENCRYPT | DECRYPT`
  - Key size: `AES_KEY_SIZE_BITS = 256`
  - Block mode: `BLOCK_MODE_GCM`
  - Encryption padding: `ENCRYPTION_PADDING_NONE`
  - `setUserAuthenticationRequired(false)`
- Cipher transform: `AES/GCM/NoPadding` (`AES_CIPHER`, AESEncryptor.kt:137).
- Decrypt: `GCMParameterSpec(tlen, iv)`, `Cipher.DECRYPT_MODE` with the keystore secret key
  (AESEncryptor.kt:110-133).

**Takeover implication (Android):** the AES key is **non-exportable** and lives in the
hardware/TEE-backed AndroidKeyStore. A v2 Kotlin app can decrypt the four credential
entries **only** if it runs as the **same package** (`dev.radaiko.gourmetclient`, signed
with the same key) so the AndroidKeyStore alias survives an in-place update. A reinstall
wipes both `SharedPreferences` and the keystore. To read the values, v2 must reproduce the
decryption: load `AndroidKeyStore`, get the SecretKey under alias
`AES/GCM/NoPadding:key_v1:keystoreUnauthenticated`, parse the JSON, Base64-decode `ct`/`iv`,
and `AES/GCM/NoPadding`-decrypt with `GCMParameterSpec(tlen, iv)` → UTF-8. (Equivalently,
v2 could keep depending on an expo-secure-store-compatible reader.)

**Legacy/hybrid scheme (read-only, not produced by this app):** on API < 23 the library
uses `HybridAESEncryptor` (`scheme = "hybrid"`, RSA-wrapped AES key under alias
`RSA/None/PKCS1Padding:<service>...`, extra JSON prop `esk`). v1's minSDK/newArch targets
never write this; v2 can ignore it unless supporting extremely old devices (v1:
HybridAESEncryptor.kt).

---

## 2. Unencrypted key-value storage (AsyncStorage)

Everything that is **not** a credential is stored **unencrypted** via AsyncStorage
(`@react-native-async-storage/async-storage` 2.2.0; on Android an SQLite-backed or
SharedPreferences-backed store, on iOS a plist/manifest — implementation detail, but the
data is **not** in the secure store). v2 may use any plain persistent KV store; the key
strings and value formats matter only if v2 wants to read v1's leftover data (most are
caches/flags safe to discard). Value *formats* and semantics are owned by the referenced
feature docs; listed here for completeness of the storage inventory.

| AsyncStorage key (string) | Purpose | Owning doc | (v1 provenance) |
|---|---|---|---|
| `@secureStorage:migratedAfterFirstUnlock` | Keychain-migration guard (`"1"` = done) | this doc §1.3 | secureStorage.ts:10 |
| `company-location` | Persisted geofence company location (zustand-persist JSON: `{ state: { companyLocation, isAtCompany }, version }`) | 03-features/notifications-location | locationStore.ts:39-41 |
| `menus_items` | Cached menu items | 03-features/caching, menus | menuStore.ts:10 |
| `orders_list` | Cached orders | 03-features/caching, orders | orderStore.ts:9 |
| `billing_<key>` (prefix `billing_`) | Cached Gourmet bills per query | 03-features/caching, billing | billingStore.ts:8 |
| `ventopay_billing_<key>` (prefix `ventopay_billing_`) | Cached Ventopay bills per query | 03-features/caching, billing | billingStore.ts:9 |
| `known_menu_fingerprints` | Menu fingerprints for new-menu detection | 03-features/notifications-new-menu | menuChangeStorage.ts:4 |
| `menu_notification_sent` | New-menu notification dedupe flag | 03-features/notifications-new-menu | menuChangeStorage.ts:5 |
| `daily_reminder_enabled` | Daily reminder on/off | 03-features/notifications-daily-reminder | reminderStorage.ts:3 |
| `daily_reminder_time` | Reminder time JSON `{ hour, minute }` | 03-features/notifications-daily-reminder | reminderStorage.ts:4 |
| `daily_reminder_sent_date` | Written after a daily reminder is *scheduled*; never read (vestigial — dropped in v2) | 03-features/notifications-daily-reminder | reminderStorage.ts:5 |
| `notification_debug_log_entries` | Notification debug log (JSON array) | 03-features/notification-log | notificationLogStorage.ts:4 |
| `notification_debug_log_activated_until` | Debug-log activation expiry (epoch ms string) | 03-features/notification-log | notificationLogStorage.ts:5 |
| `theme-preference` | Persisted theme preference + accent color (zustand-persist JSON: `{ state: { preference, accentColor }, version }`) | 03-features/themes | themeStore.ts:35 |

Two stores use zustand `persist`: the location store (name `company-location`) and the
theme store (name `theme-preference`, owned by 03-features/themes); both use
`createJSONStorage(() => AsyncStorage)` (v1: locationStore.ts:20-42, themeStore.ts:16-38).

---

## 3. Platform detection (v1: src/app/src-rn/utils/platform.ts)

v1 distinguishes `'ios' | 'android' | 'desktop' | 'web'`:
- `getAppPlatform()`: if `Platform.OS === 'web'`, return `'desktop'` when
  `window.__TAURI_INTERNALS__` or `window.__TAURI__` exists, else `'web'`; otherwise return
  `Platform.OS` (ios/android).
- `isDesktop()` = platform is `'desktop'`; `isWeb()` = `Platform.OS === 'web'`;
  `isNative()` = `Platform.OS !== 'web'`.
- `useFlatStyle` = `Platform.OS === 'android' || isDesktop()` — Android/desktop use flat
  (opaque) styles instead of glass/blur (owned by 04-ui-ux / 03-features/themes).
- `isCompactDesktop` = `isDesktop()`.

**Dropped in v2:** the `'desktop'` and `'web'` branches. In v2 the platform is always
`ios` or `android`; `isNative()` is always true, `isWeb()`/`isDesktop()` are always false,
`useFlatStyle` reduces to "is Android", `isCompactDesktop` is always false. Keep the
Android-vs-iOS style distinction (`useFlatStyle`) — that is a live requirement.

---

## 4. Background tasks

v1 registers **three** background jobs. Two use `expo-background-task` (OS-scheduled
periodic refresh, backed by `BGTaskScheduler` on iOS and WorkManager on Android); one uses
`expo-location` geofencing. Tasks must be *defined* at module load (import side effect) and
*registered* separately.

### 4.1 Geofence task — `COMPANY_GEOFENCE_TASK`
(v1: constants.ts:31 `GEOFENCE_TASK_NAME`; notificationService.ts:49-75; notificationTasks.ts:27-77)
- **Registration/start:** `Location.startGeofencingAsync('COMPANY_GEOFENCE_TASK', [region])`
  where `region = { identifier: 'company', latitude, longitude, radius: 500, notifyOnEnter: true, notifyOnExit: true }`.
  `radius` = `COMPANY_GEOFENCE_RADIUS_M = 500` meters (constants.ts:33).
- **Idempotency:** before starting, check `Location.hasStartedGeofencingAsync(...)`; if
  already running, skip (restarting re-fires Enter events and causes spurious notifications
  — notificationService.ts:53-56).
- **Requires:** a saved company location (`useLocationStore.companyLocation`); if none, no-op.
- **Handler behavior (Enter):** set `isAtCompany = true`, load cached orders, cancel any
  pending cancel-reminder, and — if no order exists for today — schedule the geofence
  "no order" notification. **(Exit):** set `isAtCompany = false`, cancel the geofence
  notification, reload cached orders, run cancel-reminder check. Full behavior/log strings
  are owned by `03-features/notifications-location.md`; the platform requirement here is the
  task name, region parameters, and idempotent start/stop.
- **Stop:** `stopGeofencing()` calls `stopGeofencingAsync` only if currently running.

### 4.2 Background order-sync task — `BACKGROUND_ORDER_SYNC_TASK`
(v1: constants.ts:32; notificationService.ts:77-94; notificationTasks.ts:82-112)
- **Registration:** first check `BackgroundTask.getStatusAsync()`; if
  `BackgroundTaskStatus.Restricted`, do not register. Else, if not already registered,
  `BackgroundTask.registerTaskAsync('BACKGROUND_ORDER_SYNC_TASK', { minimumInterval: 15 })`.
- **`minimumInterval: 15`** — minutes (OS may run it less often).
- **Unregister:** `unregisterBackgroundSync()` unregisters only if registered.
- **Handler:** loads cached orders (no network — comment: "no network calls to avoid
  concurrent scraping"), runs daily-reminder check, runs cancel-reminder check, returns
  `BackgroundTaskResult.Success` / `.Failed`. Reminder logic is owned by the reminder
  feature docs.

### 4.3 Background menu-check task — `BACKGROUND_MENU_CHECK`
(v1: src/app/src-rn/utils/backgroundMenuCheck.ts:18, 110-128)
- **Registration:** if not already registered,
  `BackgroundTask.registerTaskAsync('BACKGROUND_MENU_CHECK', { minimumInterval: 15 })` (minutes).
- **Handler:** headless (no React) — reads credentials from secure storage, **skips demo
  credentials** (never sends them to the live server), logs in, fetches menus, compares
  fingerprints, fires a "Neue Menüs verfügbar" notification if new and not already sent.
  This is the one background task that performs network scraping and reads credentials
  (hence the `AFTER_FIRST_UNLOCK` accessibility requirement in §1). Detection/notification
  logic is owned by `03-features/notifications-new-menu.md`.

### 4.4 Registration wiring & lifecycle (v1: app/_layout.tsx)
- All tasks are **defined** by importing `notificationTasks` and `backgroundMenuCheck` at
  startup (side-effect imports; `backgroundMenuCheck` is `require`'d only on native —
  _layout.tsx:26-28, 9).
- `notificationTasks.ts` guards its `defineTask` calls with `if (Platform.OS !== 'web')`
  (native-only).
- On mount (native only): `setupNotificationHandler()` + `setupAndroidChannel()`
  (_layout.tsx:69-73).
- When a company location is set: `enableNotifications()` → `startGeofencing()` +
  `registerBackgroundSync()` (_layout.tsx:75-78).
- When the daily reminder is enabled (independent of location): `registerBackgroundSync()`
  (_layout.tsx:81-93).
- Menu-check registration + notification permission request happen once the Gourmet auth
  status becomes `authenticated` (_layout.tsx:49-59), best-effort (errors swallowed).
- `enableNotifications()` = start geofencing + register order-sync;
  `disableNotifications()` = stop geofencing + unregister order-sync + cancel **ALL**
  scheduled notifications — `cancelDailyNotification()` is implemented as
  `cancelAllScheduledNotificationsAsync()` (notificationService.ts:258-260), so it clears
  every pending geofence/cancel-reminder/daily notification, not just the daily one
  (called from notifications.tsx:181 on disable; notificationService.ts:293-302).

### 4.5 OS scheduler configuration (must match app.json — §7)
- iOS `Info.plist.UIBackgroundModes` = `["location", "processing"]`.
- iOS `BGTaskSchedulerPermittedIdentifiers` = `["com.expo.modules.backgroundtask.processing"]`
  — the single background-task identifier expo-background-task registers under. v2's native
  `BGTaskScheduler` registration must use the same permitted identifier (or v2's own,
  declared in Info.plist) for order-sync and menu-check to run.
- Android manifest permissions include `RECEIVE_BOOT_COMPLETED` and `WAKE_LOCK` (for
  WorkManager rescheduling across reboots) and the foreground-service permissions for
  background location — see §7.

**Dropped in v2:** `notificationTasks.web.ts` (web has no background tasks).

---

## 5. Notifications — permissions, handler, channels

Notification **scheduling and content** are owned by the feature docs. This section
captures the platform mechanics v2 must reproduce.

### 5.1 Permission request flow
Two request paths exist (both must be preserved as behaviors):

1. `requestNotificationPermissions()` in notificationService.ts:40-47:
   - If existing permission is already `granted`, return `true` (no prompt).
   - Else request with iOS options `{ allowAlert: true, allowBadge: true, allowSound: true }`.
   - Return `status === 'granted'`.
2. `requestNotificationPermissions()` in backgroundMenuCheck.ts:134-142:
   - Web returns `false` (dropped in v2).
   - If existing `granted`, return `true`.
   - Else request with **no** iOS options object.
   - Return `status === 'granted'`. Called after first login (via _layout.tsx:54).

v2 requirement: request notification authorization with alert + badge + sound; treat an
already-granted state as success without re-prompting.

### 5.2 Notification handler (foreground presentation)
`setupNotificationHandler()` (notificationService.ts:262-271) sets the handler so incoming
notifications present as: `shouldShowBanner: true`, `shouldShowList: true`,
`shouldPlaySound: true`, `shouldSetBadge: false`. (No app-icon badge is ever set.)

### 5.3 Android notification channels
Two channels are created (Android only):

| Channel id | Name (German, user-visible) | Importance | Vibration | Created by (v1) |
|---|---|---|---|---|
| `order-reminders` | `Bestellungs-Erinnerungen` | `HIGH` | `[0, 250, 250, 250]` | notificationService.ts:273-281 (`setupAndroidChannel`); id from constants.ts:36 `NOTIFICATION_CHANNEL_ID` |
| `menu-updates` | `Neue Menüs` | `DEFAULT` | *(none)* | backgroundMenuCheck.ts:21-26 (created at module load) |

Channel routing in v1 is inconsistent (a v1 quirk — decide deliberately whether v2
reproduces it or routes all notifications to the named channels):
- **Date-triggered** reminder/geofence notifications set `channelId: order-reminders` via
  the trigger (notificationService.ts:119-123, 173-177, 225-229, 249-254).
- **Immediate-fire** variants (`trigger: null`) carry **no** channelId — the geofence
  past-target fire (notificationService.ts:126-136) and the cancel-reminder post-08:45
  fire (notificationService.ts:179-185) — so on Android they land in expo-notifications'
  fallback default channel, not `order-reminders`.
- The **new-menu** notification sets `channelId: menu-updates` inside `content`, not the
  trigger (backgroundMenuCheck.ts:73-81), where expo-notifications expects it on the
  trigger — so it too may not route to `menu-updates` as intended.

On iOS there are no channels (the `channelId` field is ignored). v2 must create the two
Android channels with these exact ids, names, importance, and vibration pattern so
upgraded installs keep consistent channel settings.

### 5.4 Notification identifiers & deep-link data (reference)
Scheduled-notification identifiers are constants (constants.ts:37-39): geofence
`geofence-no-order-reminder`, daily `daily-order-reminder`, cancel `cancel-order-reminder`.
Notification `data.screen` deep links used: `/(tabs)/orders` (daily & cancel reminders),
`/(tabs)` (new-menu). Timing thresholds (`NOTIFICATION_HOUR = 8`, `NOTIFICATION_MINUTE = 45`
→ 08:45; cancel deadline `09:00`; geofence "too late" cutoff 14:00) and the full German
notification texts live in the notification feature docs — not restated here.

**Dropped in v2:** `notificationService.web.ts` (web notification stubs).

---

## 6. Location — permissions & position

(v1: src/app/src-rn/utils/notificationService.ts:22-38, 283-291)

- `requestLocationPermissions()`:
  1. `Location.requestForegroundPermissionsAsync()` — if not `granted`, return `false`.
  2. `Location.requestBackgroundPermissionsAsync()` — if `granted`, return `true`; else
     return `false`. Comment: "iOS may not show the 'Always Allow' prompt" (background
     grant often requires the user to go to Settings). So the app must handle
     foreground-granted-but-background-denied by directing the user to system settings.
- `hasBackgroundLocationPermission()`: `getBackgroundPermissionsAsync().status === 'granted'`
  (non-prompting check, used to decide whether to send the user to Settings).
- `getCurrentPosition()`: `Location.getCurrentPositionAsync({ accuracy: Accuracy.High })`,
  returns `{ latitude, longitude }` (used when the user captures their company location).

iOS usage-description strings (from the expo-location plugin config, app.json:99-107):
- `locationAlwaysAndWhenInUsePermission`:
  `"SnackPilot nutzt deinen Standort, um dich an Bestellungen zu erinnern, wenn du im Büro bist."`
- `locationWhenInUsePermission`:
  `"SnackPilot nutzt deinen Standort, um deinen Firmenstandort zu speichern."`
- Plugin flags: `isIosBackgroundLocationEnabled: true`, `isAndroidBackgroundLocationEnabled: true`,
  `isAndroidForegroundServiceEnabled: true`.

Background location is required for the geofence task (§4.1); "Always Allow" (iOS) /
`ACCESS_BACKGROUND_LOCATION` (Android) is what lets Enter/Exit fire while the app is
backgrounded.

---

## 7. app.json configuration relevant to v2

(v1: src/app/app.json) The following config is not RN-specific and must be carried into
v2's native project files (Info.plist / entitlements / AndroidManifest / Gradle). Values are
exact.

### 7.1 App identity
| Field | Value | Takeover relevance |
|---|---|---|
| `name` | `SnackPilot` | Display name |
| `slug` | `GourmetApp` | Expo project slug (EAS); N/A to v2 native build |
| `scheme` | `snackpilot` | Deep-link URL scheme — v2 must keep for notification deep links & universal links |
| `version` | `1.4.5` | Marketing version at baseline |
| `orientation` | `portrait` | Lock to portrait |
| `userInterfaceStyle` | `automatic` | Follow system light/dark |
| `newArchEnabled` | `true` | RN new architecture (N/A to native v2) |
| `owner` | `radaiko` | Expo/EAS org (N/A to v2 native) |
| `extra.eas.projectId` | `efb12eb3-0729-4ea2-a3db-8026d95db7d3` | EAS project id (N/A to v2 native) |

### 7.2 iOS
- `ios.bundleIdentifier`: **`dev.radaiko.gourmetclient`** — v2 **must keep this exact bundle
  id** (and ship under the same team) for keychain credential takeover (§1.4). Note it does
  not match the `snackpilot`/`SnackPilot` naming — it retains the original GourmetClient id.
- `ios.buildNumber`: `1`
- `ios.supportsTablet`: `false`
- `ios.infoPlist.ITSAppUsesNonExemptEncryption`: `false`
- `ios.infoPlist.UIBackgroundModes`: `["location", "processing"]`
- `ios.infoPlist.BGTaskSchedulerPermittedIdentifiers`: `["com.expo.modules.backgroundtask.processing"]`

### 7.3 Android
- `android.package`: **`dev.radaiko.gourmetclient`** — v2 **must keep this exact package
  name** (signed with the same key) for AndroidKeyStore/SharedPreferences credential
  takeover (§1.5).
- `android.edgeToEdgeEnabled`: `true`
- `android.predictiveBackGestureEnabled`: `false`
- `android.adaptiveIcon`: foreground `./assets/icons/adaptive-icon-orange.png`, background `#F0F0F2` (icons owned by 03-features/themes)
- `android.permissions`:
  ```
  ACCESS_COARSE_LOCATION
  ACCESS_FINE_LOCATION
  ACCESS_BACKGROUND_LOCATION
  FOREGROUND_SERVICE
  FOREGROUND_SERVICE_LOCATION
  RECEIVE_BOOT_COMPLETED
  WAKE_LOCK
  ```

### 7.4 Plugins (Expo config plugins → native effects v2 must reproduce)
| Plugin | v2-relevant effect |
|---|---|
| `expo-router` | File-based navigation (RN-specific; v2 uses native navigation) |
| `expo-secure-store` | The credential store described in §1 (v2 reimplements natively) |
| `expo-font` | Custom font loading (see 04-ui-ux) |
| `expo-notifications` (`{ icon: ./assets/icons/icon-orange.png, color: #FF6B35 }`) | Android notification small-icon + accent color `#FF6B35` |
| `@g9k/expo-dynamic-app-icon` (`emerald`/`berry`/`golden`/`ocean` variants) | Alternate app icons per theme — owned by 03-features/themes; v2 needs native alternate-icon support |
| `expo-location` (config in §6) | Location permission strings + background location |
| `expo-background-task` | Background task scheduling (§4) |
| `expo-localization` | Locale/timezone lookup (§8) |
| `expo-mail-composer` | Compose email (used for support/feedback; see 04-ui-ux / settings) |

### 7.5 Splash & icons
- `splash`: `{ image: ./assets/splash-icon.png, resizeMode: contain, backgroundColor: #ffffff }`
- `icon`: `./assets/icons/icon-orange.png`
- Theme icon assets are owned by 03-features/themes and 07-release.

**Dropped in v2:** the entire `web` block (`web.output: single`, `web.bundler: metro`,
`web.favicon`), and any Tauri/desktop packaging (not in app.json — see appendix "Dropped").

---

## 8. Localization (expo-localization)

`expo-localization` is used in **exactly one place**: the analytics provider
(v1: src/app/src-rn/components/AnalyticsProvider.tsx:6, 79-80, 103, 108-115). It reads:
- `getLocales()[0]` → `languageTag`, `languageCode`, `regionCode`, `textDirection`.
- `getCalendars()[0]` → `timeZone` (falls back to `Intl.DateTimeFormat().resolvedOptions().timeZone`).

These feed TelemetryDeck default-payload fields (locale, language, region, layout direction,
time zone). There is **no in-app localization / i18n** — all user-facing strings are
hard-coded German. v2 only needs system locale/timezone lookup for analytics parity; see
`03-features/analytics.md` for the exact payload keys. The app is not translated, so no
resource-bundle localization is required.

---

## 9. Summary of v2 takeover requirements

1. **iOS:** ship v2 under bundle id `dev.radaiko.gourmetclient` (same team) so the default
   keychain access group is shared. Read `kSecClassGenericPassword` items with service
   `app:no-auth` (fallback `app:auth`, then `app`), account = UTF-8 of
   `gourmet_username` / `gourmet_password` / `ventopay_username` / `ventopay_password`;
   value is plaintext UTF-8. Write back with accessibility `AfterFirstUnlock`.
2. **Android:** ship v2 as package `dev.radaiko.gourmetclient` signed with the same key so
   the `SecureStore` SharedPreferences file and the AndroidKeyStore alias survive an
   in-place update. Read entries `key_v1-<keyname>` from `shared_prefs/SecureStore.xml`,
   parse the JSON, and AES-256-GCM-decrypt `ct` using IV `iv` and tag length `tlen` with the
   AndroidKeyStore secret key aliased `AES/GCM/NoPadding:key_v1:keystoreUnauthenticated`.
3. Reproduce background task identifiers/intervals (§4), notification channels (§5.3),
   permission flows (§5.1, §6), and the app.json OS config (§7).

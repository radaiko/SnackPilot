#ifndef PLATFORM_SHIMS_H
#define PLATFORM_SHIMS_H

#include "Embedding.h"
#include <stddef.h>
#include <stdint.h>

// The Rust core owns every platform decision (design D4: the Swift side is a
// dumb shell). Swift registers one C function table at launch with
// snackpilot_ios_register_shims; each entry mirrors one Platform trait
// capability, exactly like the Android JNI bridge.
//
// Asynchronous operations (permission prompts, one-shot location fixes) take
// a request id and complete later on snackpilot_ios_complete. The payload is
// read synchronously inside that call; Rust frees nothing on this side.

typedef struct {
  int32_t kind;             // 0 = fire now, 1 = schedule at, 2 = cancel pending
  const char *id;           // notification identifier (v1 UNNotificationRequest id)
  const char *title;
  const char *body;
  const char *channel_id;   // ignored on iOS, mirrors Android
  int64_t fire_at_epoch_ms;
} SnackPilotWireCommand;

typedef uint8_t (*SnackPilotKeychainSetFn)(const char *key, const char *value);
typedef uint8_t (*SnackPilotKeychainGetFn)(const char *key, char **out_value);
typedef uint8_t (*SnackPilotKeychainDeleteFn)(const char *key);
typedef uint8_t (*SnackPilotNotifyFn)(const SnackPilotWireCommand *cmd);
typedef uint8_t (*SnackPilotNotifyPermissionFn)(uint64_t request_id, void *ctx);
typedef uint8_t (*SnackPilotLocationAuthorisationFn)(uint64_t request_id, void *ctx);
typedef uint8_t (*SnackPilotLocationCurrentFn)(uint64_t request_id, void *ctx);
typedef uint8_t (*SnackPilotGeofenceStartFn)(double latitude, double longitude,
                                              uint64_t request_id, void *ctx);
typedef uint8_t (*SnackPilotGeofenceStopFn)(void);
typedef uint8_t (*SnackPilotGeofenceIsMonitoringFn)(void);
typedef uint8_t (*SnackPilotTrackFn)(const char *event_json);
typedef uint8_t (*SnackPilotOpenEmailFn)(const char *to, const char *subject,
                                         const char *body);
typedef uint8_t (*SnackPilotOpenSettingsFn)(void);
typedef uint8_t (*SnackPilotOpenAppSettingsFn)(void);
typedef uint8_t (*SnackPilotSetAppIconFn)(const char *icon);

// Every entry is required; size guards against table drift between the two
// sides (must equal the number of function fields plus one).
typedef struct {
  uint32_t size;
  SnackPilotKeychainSetFn keychain_set;
  SnackPilotKeychainGetFn keychain_get;
  SnackPilotKeychainDeleteFn keychain_delete;
  SnackPilotNotifyFn notify;
  SnackPilotNotifyPermissionFn notify_permission;
  SnackPilotLocationAuthorisationFn location_authorisation;
  SnackPilotLocationCurrentFn location_current;
  SnackPilotGeofenceStartFn geofence_start;
  SnackPilotGeofenceStopFn geofence_stop;
  SnackPilotGeofenceIsMonitoringFn geofence_is_monitoring;
  SnackPilotTrackFn track;
  SnackPilotOpenEmailFn open_email;
  SnackPilotOpenSettingsFn open_settings;
  SnackPilotOpenAppSettingsFn open_app_settings;
  SnackPilotSetAppIconFn set_app_icon;
} SnackPilotIosShimTable;

/// Registers the shim table plus the storage directory the Rust core uses.
/// Returns 1 on success, 0 on a second registration or an invalid table.
uint8_t snackpilot_ios_register_shims(const SnackPilotIosShimTable *table,
                                      const char *storage_dir);

/// Completes a pending asynchronous request started by one of the *_with_id
/// shims. ok = 1 carries an optional payload, ok = 0 carries the error
/// message. The payload is read before this call returns.
void snackpilot_ios_complete(uint64_t request_id, uint8_t ok, const char *payload);

/// CoreLocation enter/exit edge; forwards to the Rust geofence delegate.
void snackpilot_ios_on_geofence_event(uint8_t inside);

/// Runs the evidence platform exercise on a background thread; every step
/// logs to stderr tagged "[3.5]".
void snackpilot_ios_run_debug_exercise(void);

#endif  // PLATFORM_SHIMS_H

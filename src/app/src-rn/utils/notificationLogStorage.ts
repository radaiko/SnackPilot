import AsyncStorage from '@react-native-async-storage/async-storage';

// ─── Keys ───────────────────────────────────────────────────────────────────
const LOG_ENTRIES_KEY = 'notification_debug_log_entries';
const LOG_ACTIVATED_UNTIL_KEY = 'notification_debug_log_activated_until';

// ─── Data model ─────────────────────────────────────────────────────────────
export type LogSubsystem = 'geofence' | 'order-sync' | 'daily-reminder' | 'menu-check';
export type LogLevel = 'info' | 'guard' | 'error' | 'notification';

export interface NotificationLogEntry {
  /** ISO 8601 timestamp */
  ts: string;
  subsystem: LogSubsystem;
  level: LogLevel;
  /** Short machine-readable tag, e.g. "time_guard_fail" */
  event: string;
  /** Human-readable context, e.g. "currentMin=510 targetMin=525 delta=15" */
  detail?: string;
}

const MAX_ENTRIES = 200;
const LOG_WINDOW_MS = 24 * 60 * 60 * 1000;

// ─── Activation window ───────────────────────────────────────────────────────

export async function getLogActivatedUntil(): Promise<number | null> {
  const raw = await AsyncStorage.getItem(LOG_ACTIVATED_UNTIL_KEY);
  if (!raw) return null;
  const n = Number(raw);
  return isNaN(n) ? null : n;
}

export async function activateLog(): Promise<void> {
  const until = Date.now() + LOG_WINDOW_MS;
  await AsyncStorage.setItem(LOG_ACTIVATED_UNTIL_KEY, String(until));
  await AsyncStorage.removeItem(LOG_ENTRIES_KEY);
}

export async function clearLog(): Promise<void> {
  await AsyncStorage.removeItem(LOG_ACTIVATED_UNTIL_KEY);
  await AsyncStorage.removeItem(LOG_ENTRIES_KEY);
}

export async function isLogActive(): Promise<boolean> {
  const until = await getLogActivatedUntil();
  if (until === null) return false;
  return Date.now() < until;
}

// ─── Log entries ─────────────────────────────────────────────────────────────

export async function getLogEntries(): Promise<NotificationLogEntry[]> {
  const raw = await AsyncStorage.getItem(LOG_ENTRIES_KEY);
  if (!raw) return [];
  try {
    return JSON.parse(raw) as NotificationLogEntry[];
  } catch {
    return [];
  }
}

/**
 * Append a log entry. No-op if the activation window has expired or is not set.
 * Caps total entries at MAX_ENTRIES (oldest are dropped).
 * Fire-and-forget safe: errors are swallowed so callers never throw.
 *
 * Uses multiGet to batch both reads into a single AsyncStorage call,
 * reducing (but not eliminating) race conditions between concurrent appends.
 * Concurrent writes may cause entry loss, which is acceptable for diagnostic logs.
 */
export async function appendLogEntry(
  subsystem: LogSubsystem,
  level: LogLevel,
  event: string,
  detail?: string,
): Promise<void> {
  try {
    const results = await AsyncStorage.multiGet([LOG_ACTIVATED_UNTIL_KEY, LOG_ENTRIES_KEY]);
    const rawUntil = results[0][1];
    const rawEntries = results[1][1];

    // Check activation window
    const until = rawUntil ? Number(rawUntil) : null;
    if (!until || isNaN(until) || Date.now() >= until) return;

    // Parse existing entries
    let entries: NotificationLogEntry[] = [];
    if (rawEntries) {
      try { entries = JSON.parse(rawEntries); } catch { /* start fresh */ }
    }

    const entry: NotificationLogEntry = {
      ts: new Date().toISOString(),
      subsystem,
      level,
      event,
      ...(detail !== undefined ? { detail } : {}),
    };

    const updated = [...entries, entry].slice(-MAX_ENTRIES);
    await AsyncStorage.setItem(LOG_ENTRIES_KEY, JSON.stringify(updated));
  } catch {
    // Never throw from a logging call
  }
}

// ─── Email export ─────────────────────────────────────────────────────────────

export function formatLogForEmail(entries: NotificationLogEntry[]): string {
  if (entries.length === 0) return '(keine Einträge aufgezeichnet)';
  return entries
    .map(
      (e) =>
        `[${e.ts}] [${e.subsystem}] [${e.level.toUpperCase()}] ${e.event}` +
        (e.detail ? `\n  ${e.detail}` : ''),
    )
    .join('\n');
}

jest.mock('@react-native-async-storage/async-storage', () => {
  const store: Record<string, string> = {};
  return {
    __esModule: true,
    default: {
      getItem: jest.fn((key: string) => Promise.resolve(store[key] ?? null)),
      setItem: jest.fn((key: string, value: string) => { store[key] = value; return Promise.resolve(); }),
      removeItem: jest.fn((key: string) => { delete store[key]; return Promise.resolve(); }),
      clear: jest.fn(() => { Object.keys(store).forEach(k => delete store[k]); return Promise.resolve(); }),
      multiGet: jest.fn((keys: string[]) =>
        Promise.resolve(keys.map((k) => [k, store[k] ?? null] as [string, string | null]))
      ),
    },
  };
});

// jest.config.js maps '.*/utils/notificationLogStorage$' to a manual mock for all
// other suites. Requiring with an explicit .ts extension dodges that mapping so
// this suite exercises the real implementation.
const storage = require('../../utils/notificationLogStorage.ts') as
  typeof import('../../utils/notificationLogStorage');

const AsyncStorage = require('@react-native-async-storage/async-storage').default;

const UNTIL_KEY = 'notification_debug_log_activated_until';
const ENTRIES_KEY = 'notification_debug_log_entries';

beforeEach(async () => {
  await AsyncStorage.clear();
  jest.clearAllMocks();
});

describe('notificationLogStorage', () => {
  describe('getLogActivatedUntil', () => {
    it('returns null when never activated', async () => {
      expect(await storage.getLogActivatedUntil()).toBeNull();
    });

    it('returns the stored timestamp', async () => {
      await AsyncStorage.setItem(UNTIL_KEY, '1234567890');
      expect(await storage.getLogActivatedUntil()).toBe(1234567890);
    });

    it('returns null for a corrupt (non-numeric) value', async () => {
      await AsyncStorage.setItem(UNTIL_KEY, 'not-a-number');
      expect(await storage.getLogActivatedUntil()).toBeNull();
    });
  });

  describe('activateLog', () => {
    it('sets the activation window and clears old entries', async () => {
      await AsyncStorage.setItem(ENTRIES_KEY, '[{"old":true}]');
      const before = Date.now();

      await storage.activateLog(2);

      const until = await storage.getLogActivatedUntil();
      expect(until).toBeGreaterThanOrEqual(before + 2 * 60 * 60 * 1000);
      expect(until).toBeLessThanOrEqual(Date.now() + 2 * 60 * 60 * 1000);
      expect(await AsyncStorage.getItem(ENTRIES_KEY)).toBeNull();
    });

    it('defaults to a 24 hour window', async () => {
      const before = Date.now();
      await storage.activateLog();
      const until = await storage.getLogActivatedUntil();
      expect(until).toBeGreaterThanOrEqual(before + 24 * 60 * 60 * 1000);
    });
  });

  describe('clearLog', () => {
    it('removes activation window and entries', async () => {
      await storage.activateLog();
      await storage.appendLogEntry('geofence', 'info', 'test');

      await storage.clearLog();

      expect(await storage.getLogActivatedUntil()).toBeNull();
      expect(await storage.getLogEntries()).toEqual([]);
    });
  });

  describe('isLogActive', () => {
    it('is false when never activated', async () => {
      expect(await storage.isLogActive()).toBe(false);
    });

    it('is true within the activation window', async () => {
      await storage.activateLog(1);
      expect(await storage.isLogActive()).toBe(true);
    });

    it('is false after the window expired', async () => {
      await AsyncStorage.setItem(UNTIL_KEY, String(Date.now() - 1000));
      expect(await storage.isLogActive()).toBe(false);
    });
  });

  describe('getLogEntries', () => {
    it('returns [] when nothing is stored', async () => {
      expect(await storage.getLogEntries()).toEqual([]);
    });

    it('returns [] for corrupt JSON', async () => {
      await AsyncStorage.setItem(ENTRIES_KEY, '{not json');
      expect(await storage.getLogEntries()).toEqual([]);
    });

    it('returns parsed entries', async () => {
      const entries = [{ ts: '2026-01-01T00:00:00Z', subsystem: 'geofence', level: 'info', event: 'e1' }];
      await AsyncStorage.setItem(ENTRIES_KEY, JSON.stringify(entries));
      expect(await storage.getLogEntries()).toEqual(entries);
    });
  });

  describe('appendLogEntry', () => {
    it('is a no-op when the log is not activated', async () => {
      await storage.appendLogEntry('geofence', 'info', 'ignored');
      expect(await storage.getLogEntries()).toEqual([]);
    });

    it('is a no-op when the activation window has expired', async () => {
      await AsyncStorage.setItem(UNTIL_KEY, String(Date.now() - 1000));
      await storage.appendLogEntry('geofence', 'info', 'ignored');
      expect(await storage.getLogEntries()).toEqual([]);
    });

    it('is a no-op when the stored window is corrupt', async () => {
      await AsyncStorage.setItem(UNTIL_KEY, 'garbage');
      await storage.appendLogEntry('geofence', 'info', 'ignored');
      expect(await storage.getLogEntries()).toEqual([]);
    });

    it('appends an entry with timestamp and detail when active', async () => {
      await storage.activateLog();

      await storage.appendLogEntry('order-sync', 'error', 'fetch_failed', 'status=500');

      const entries = await storage.getLogEntries();
      expect(entries).toHaveLength(1);
      expect(entries[0].subsystem).toBe('order-sync');
      expect(entries[0].level).toBe('error');
      expect(entries[0].event).toBe('fetch_failed');
      expect(entries[0].detail).toBe('status=500');
      expect(new Date(entries[0].ts).getTime()).not.toBeNaN();
    });

    it('omits the detail field when not provided', async () => {
      await storage.activateLog();

      await storage.appendLogEntry('daily-reminder', 'guard', 'time_guard_fail');

      const entries = await storage.getLogEntries();
      expect(entries[0]).not.toHaveProperty('detail');
    });

    it('starts fresh when existing entries are corrupt', async () => {
      await storage.activateLog();
      await AsyncStorage.setItem(ENTRIES_KEY, '{corrupt');

      await storage.appendLogEntry('menu-check', 'info', 'new_entry');

      const entries = await storage.getLogEntries();
      expect(entries).toHaveLength(1);
      expect(entries[0].event).toBe('new_entry');
    });

    it('caps stored entries at 200, dropping the oldest', async () => {
      await storage.activateLog();
      const existing = Array.from({ length: 200 }, (_, i) => ({
        ts: '2026-01-01T00:00:00Z', subsystem: 'geofence', level: 'info', event: `e${i}`,
      }));
      await AsyncStorage.setItem(ENTRIES_KEY, JSON.stringify(existing));

      await storage.appendLogEntry('geofence', 'info', 'e200');

      const entries = await storage.getLogEntries();
      expect(entries).toHaveLength(200);
      expect(entries[0].event).toBe('e1');
      expect(entries[199].event).toBe('e200');
    });

    it('never throws when storage fails', async () => {
      AsyncStorage.multiGet.mockRejectedValueOnce(new Error('disk full'));
      await expect(storage.appendLogEntry('geofence', 'info', 'x')).resolves.toBeUndefined();
    });
  });

  describe('formatLogForEmail', () => {
    it('returns a placeholder for an empty log', () => {
      expect(storage.formatLogForEmail([])).toBe('(keine Einträge aufgezeichnet)');
    });

    it('formats entries with and without detail', () => {
      const text = storage.formatLogForEmail([
        { ts: '2026-01-01T08:00:00Z', subsystem: 'geofence', level: 'info', event: 'enter' },
        { ts: '2026-01-01T08:05:00Z', subsystem: 'order-sync', level: 'error', event: 'fail', detail: 'status=500' },
      ]);

      expect(text).toBe(
        '[2026-01-01T08:00:00Z] [geofence] [INFO] enter\n' +
        '[2026-01-01T08:05:00Z] [order-sync] [ERROR] fail\n  status=500'
      );
    });
  });
});

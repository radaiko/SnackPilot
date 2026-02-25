jest.mock('@react-native-async-storage/async-storage', () => {
  const store: Record<string, string> = {};
  return {
    __esModule: true,
    default: {
      getItem: jest.fn((key: string) => Promise.resolve(store[key] ?? null)),
      setItem: jest.fn((key: string, value: string) => { store[key] = value; return Promise.resolve(); }),
      removeItem: jest.fn((key: string) => { delete store[key]; return Promise.resolve(); }),
      clear: jest.fn(() => { Object.keys(store).forEach((k) => { delete store[k]; }); return Promise.resolve(); }),
    },
  };
});

import {
  getReminderEnabled,
  setReminderEnabled,
  getReminderTime,
  setReminderTime,
  getReminderSentDate,
  setReminderSentDate,
} from '../../utils/reminderStorage';

beforeEach(async () => {
  const AsyncStorage = require('@react-native-async-storage/async-storage').default;
  await AsyncStorage.clear();
  jest.clearAllMocks();
});

describe('reminderStorage', () => {
  describe('reminderEnabled', () => {
    it('returns false when nothing stored', async () => {
      expect(await getReminderEnabled()).toBe(false);
    });

    it('persists true', async () => {
      await setReminderEnabled(true);
      expect(await getReminderEnabled()).toBe(true);
    });

    it('persists false after true', async () => {
      await setReminderEnabled(true);
      await setReminderEnabled(false);
      expect(await getReminderEnabled()).toBe(false);
    });
  });

  describe('reminderTime', () => {
    it('returns null when nothing stored', async () => {
      expect(await getReminderTime()).toBeNull();
    });

    it('persists hour and minute', async () => {
      await setReminderTime(11, 30);
      expect(await getReminderTime()).toEqual({ hour: 11, minute: 30 });
    });

    it('overwrites previous time', async () => {
      await setReminderTime(11, 30);
      await setReminderTime(12, 0);
      expect(await getReminderTime()).toEqual({ hour: 12, minute: 0 });
    });
  });

  describe('reminderSentDate', () => {
    it('returns null when nothing stored', async () => {
      expect(await getReminderSentDate()).toBeNull();
    });

    it('persists a date string', async () => {
      await setReminderSentDate('2026-02-25');
      expect(await getReminderSentDate()).toBe('2026-02-25');
    });
  });
});

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
  getKnownMenus,
  setKnownMenus,
  getNotificationSent,
  setNotificationSent,
} from '../../utils/menuChangeStorage';

beforeEach(async () => {
  const AsyncStorage = require('@react-native-async-storage/async-storage').default;
  await AsyncStorage.clear();
  jest.clearAllMocks();
});

describe('menuChangeStorage', () => {
  describe('knownMenus', () => {
    it('returns empty map when nothing stored', async () => {
      const result = await getKnownMenus();
      expect(result).toEqual(new Map());
    });

    it('persists and retrieves a map', async () => {
      const map = new Map([['a', 'fp1'], ['b', 'fp2']]);
      await setKnownMenus(map);
      const result = await getKnownMenus();
      expect(result).toEqual(map);
    });
  });

  describe('notificationSent', () => {
    it('returns false when nothing stored', async () => {
      expect(await getNotificationSent()).toBe(false);
    });

    it('persists true', async () => {
      await setNotificationSent(true);
      expect(await getNotificationSent()).toBe(true);
    });

    it('persists false after true', async () => {
      await setNotificationSent(true);
      await setNotificationSent(false);
      expect(await getNotificationSent()).toBe(false);
    });
  });
});

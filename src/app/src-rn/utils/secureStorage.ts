import * as SecureStore from 'expo-secure-store';
import AsyncStorage from '@react-native-async-storage/async-storage';

// AFTER_FIRST_UNLOCK allows background tasks to read credentials
// even when the device is locked (after it was unlocked at least once since boot).
const STORE_OPTIONS: SecureStore.SecureStoreOptions = {
  keychainAccessible: SecureStore.AFTER_FIRST_UNLOCK,
};

const MIGRATION_KEY = '@secureStorage:migratedAfterFirstUnlock';

/**
 * Migrates existing keychain items from default accessibility (WHEN_UNLOCKED)
 * to AFTER_FIRST_UNLOCK. Must be called while the app is in the foreground.
 * Safe to call multiple times — runs only once.
 */
export async function migrateKeychainAccessibility(keys: string[]): Promise<void> {
  const already = await AsyncStorage.getItem(MIGRATION_KEY);
  if (already === '1') return;

  for (const key of keys) {
    try {
      // Read with no options (matches the old default accessibility)
      const value = await SecureStore.getItemAsync(key);
      if (value != null) {
        // Delete the old item and re-save with AFTER_FIRST_UNLOCK
        await SecureStore.deleteItemAsync(key);
        await SecureStore.setItemAsync(key, value, STORE_OPTIONS);
      }
    } catch {
      // Item may not exist yet — that's fine
    }
  }

  await AsyncStorage.setItem(MIGRATION_KEY, '1');
}

export async function getItem(key: string): Promise<string | null> {
  return SecureStore.getItemAsync(key, STORE_OPTIONS);
}

export async function setItem(key: string, value: string): Promise<void> {
  await SecureStore.setItemAsync(key, value, STORE_OPTIONS);
}

export async function deleteItem(key: string): Promise<void> {
  await SecureStore.deleteItemAsync(key, STORE_OPTIONS);
}

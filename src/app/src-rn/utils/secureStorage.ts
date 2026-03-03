import * as SecureStore from 'expo-secure-store';

// AFTER_FIRST_UNLOCK allows background tasks to read credentials
// even when the device is locked (after it was unlocked at least once since boot).
const STORE_OPTIONS: SecureStore.SecureStoreOptions = {
  keychainAccessible: SecureStore.AFTER_FIRST_UNLOCK,
};

export async function getItem(key: string): Promise<string | null> {
  return SecureStore.getItemAsync(key, STORE_OPTIONS);
}

export async function setItem(key: string, value: string): Promise<void> {
  await SecureStore.setItemAsync(key, value, STORE_OPTIONS);
}

export async function deleteItem(key: string): Promise<void> {
  await SecureStore.deleteItemAsync(key, STORE_OPTIONS);
}

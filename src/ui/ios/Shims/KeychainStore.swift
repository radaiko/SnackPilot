import Foundation
import Security

/// iOS Keychain-backed credential storage.
///
/// Ported from v1 `src/ios/SnackPilot/KeychainStore.swift` (task 3.5): the
/// generic-password items keyed by service `app:no-auth` are byte-compatible
/// with the v1 app, so upgrading users keep their session.
enum KeychainStore {
    private static let service = "app:no-auth"

    private static func query(for key: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
        ]
    }

    /// Stores (or replaces) a value. Returns false instead of throwing so the
    /// Rust caller sees a typed failure rather than a Swift crash.
    static func set(key: String, value: String) -> Bool {
        let data = Data(value.utf8)
        let base = query(for: key)
        SecItemDelete(base as CFDictionary)
        var attributes = base
        attributes[kSecAttrGeneric as String] = Data(key.utf8) // v1 sets this to the key bytes too
        attributes[kSecValueData as String] = data
        attributes[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlock
        let status = SecItemAdd(attributes as CFDictionary, nil)
        if status != errSecSuccess {
            NSLog("%@", "[3.5] SecItemAdd(\(key)) failed: OSStatus \(status)")
        }
        return status == errSecSuccess
    }

    /// Returns nil when the key is absent or undecodable.
    static func get(key: String) -> String? {
        var base = query(for: key)
        base[kSecReturnData as String] = true
        base[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: AnyObject?
        guard SecItemCopyMatching(base as CFDictionary, &result) == errSecSuccess,
              let data = result as? Data else {
            return nil
        }
        return String(data: data, encoding: .utf8)
    }

    static func delete(key: String) -> Bool {
        let status = SecItemDelete(query(for: key) as CFDictionary)
        return status == errSecSuccess || status == errSecItemNotFound
    }
}

import Foundation
import Security

/// v1-compatible Keychain credential storage (05-platform-services §1.1, §1.4). Uses the exact
/// coordinates expo-secure-store 55.0.15 wrote under, so a v2 build running under the same app
/// identity transparently reads credentials a v1 install left behind (in-place takeover) and
/// writes them back in the same format. Values are plaintext UTF-8; the Keychain encrypts at rest.
enum KeychainStore {
    /// The four credential keys (verbatim, §1.1 / settings §3.5).
    static let gourmetUsername = "gourmet_username"
    static let gourmetPassword = "gourmet_password"
    static let ventopayUsername = "ventopay_username"
    static let ventopayPassword = "ventopay_password"

    /// v1's service string: library default "app" + ":no-auth" for unauthenticated items (§1.4).
    private static let service = "app:no-auth"

    private static func baseQuery(_ key: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
        ]
    }

    static func set(_ key: String, _ value: String) {
        SecItemDelete(baseQuery(key) as CFDictionary)
        var attrs = baseQuery(key)
        attrs[kSecAttrGeneric as String] = Data(key.utf8) // v1 sets this to the key bytes too
        attrs[kSecValueData as String] = Data(value.utf8)
        attrs[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlock
        SecItemAdd(attrs as CFDictionary, nil)
    }

    static func get(_ key: String) -> String? {
        var query = baseQuery(key)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: AnyObject?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess,
              let data = result as? Data,
              let str = String(data: data, encoding: .utf8) else { return nil }
        return str
    }

    static func delete(_ key: String) {
        SecItemDelete(baseQuery(key) as CFDictionary)
    }

    // MARK: Credential pairs

    /// Save both keys (settings §3.6 `saveCredentials`).
    static func saveGourmet(username: String, password: String) {
        set(gourmetUsername, username)
        set(gourmetPassword, password)
    }

    /// Read both keys; `nil` unless BOTH are non-empty (settings §3.6 `getSavedCredentials`).
    static func savedGourmet() -> (username: String, password: String)? {
        guard let u = get(gourmetUsername), let p = get(gourmetPassword),
              !u.isEmpty, !p.isEmpty else { return nil }
        return (u, p)
    }

    /// Save both Ventopay keys (settings §3.6 `saveCredentials`, Automaten pair).
    static func saveVentopay(username: String, password: String) {
        set(ventopayUsername, username)
        set(ventopayPassword, password)
    }

    /// Read both Ventopay keys; `nil` unless BOTH are non-empty (settings §3.6).
    static func savedVentopay() -> (username: String, password: String)? {
        guard let u = get(ventopayUsername), let p = get(ventopayPassword),
              !u.isEmpty, !p.isEmpty else { return nil }
        return (u, p)
    }
}

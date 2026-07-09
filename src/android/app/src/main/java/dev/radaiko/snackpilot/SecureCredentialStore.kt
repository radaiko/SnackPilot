package dev.radaiko.snackpilot

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import org.json.JSONObject
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * v1-compatible secure credential storage (05-platform-services §1.1, §1.5). Reproduces the
 * exact expo-secure-store 55.0.15 on-disk format — a `SecureStore` SharedPreferences file with
 * `key_v1-<key>` entries holding an AES-256-GCM JSON envelope, keyed by an AndroidKeyStore
 * secret under alias `AES/GCM/NoPadding:key_v1:keystoreUnauthenticated`. Because v2 runs under
 * the same package/signature, this both persists v2's own credentials and transparently reads
 * (takes over) credentials a v1 install left behind after an in-place update. Plaintext values.
 */
class SecureCredentialStore(context: Context) {
    private val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    fun set(key: String, value: String) {
        val cipher = Cipher.getInstance(TRANSFORM)
        cipher.init(Cipher.ENCRYPT_MODE, getOrCreateKey())
        val ct = cipher.doFinal(value.toByteArray(Charsets.UTF_8))
        val json = JSONObject().apply {
            put("ct", Base64.encodeToString(ct, Base64.NO_WRAP))
            put("iv", Base64.encodeToString(cipher.iv, Base64.NO_WRAP))
            put("tlen", 128)
            put("scheme", "aes")
            put("usesKeystoreSuffix", true)
            put("keystoreAlias", KEYSTORE_ALIAS)
            put("requireAuthentication", false)
        }
        prefs.edit().putString(entryKey(key), json.toString()).apply()
    }

    fun get(key: String): String? {
        val raw = prefs.getString(entryKey(key), null) ?: return null
        return try {
            val json = JSONObject(raw)
            val ct = Base64.decode(json.getString("ct"), Base64.NO_WRAP)
            val iv = Base64.decode(json.getString("iv"), Base64.NO_WRAP)
            val tlen = json.optInt("tlen", 128)
            if (tlen < 96) return null // matches v1's reject rule
            val cipher = Cipher.getInstance(TRANSFORM)
            cipher.init(Cipher.DECRYPT_MODE, getOrCreateKey(), GCMParameterSpec(tlen, iv))
            String(cipher.doFinal(ct), Charsets.UTF_8)
        } catch (e: Exception) {
            null
        }
    }

    fun delete(key: String) {
        prefs.edit().remove(entryKey(key)).apply()
    }

    // Gourmet credential pair (settings §3.6).
    fun saveGourmet(username: String, password: String) {
        set(GOURMET_USERNAME, username)
        set(GOURMET_PASSWORD, password)
    }

    /** `null` unless BOTH keys are present and non-empty (settings §3.6). */
    fun savedGourmet(): Pair<String, String>? {
        val u = get(GOURMET_USERNAME)
        val p = get(GOURMET_PASSWORD)
        return if (!u.isNullOrEmpty() && !p.isNullOrEmpty()) u to p else null
    }

    private fun getOrCreateKey(): SecretKey {
        val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (ks.getEntry(KEY_ALIAS, null) as? KeyStore.SecretKeyEntry)?.let { return it.secretKey }
        val kg = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        kg.init(
            KeyGenParameterSpec.Builder(
                KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setKeySize(256)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setUserAuthenticationRequired(false)
                .build()
        )
        return kg.generateKey()
    }

    private fun entryKey(key: String) = "$KEYSTORE_ALIAS-$key"

    companion object {
        const val GOURMET_USERNAME = "gourmet_username"
        const val GOURMET_PASSWORD = "gourmet_password"
        const val VENTOPAY_USERNAME = "ventopay_username"
        const val VENTOPAY_PASSWORD = "ventopay_password"

        private const val PREFS = "SecureStore"
        private const val KEYSTORE_ALIAS = "key_v1"
        private const val TRANSFORM = "AES/GCM/NoPadding"
        private const val KEY_ALIAS = "AES/GCM/NoPadding:key_v1:keystoreUnauthenticated"
    }
}

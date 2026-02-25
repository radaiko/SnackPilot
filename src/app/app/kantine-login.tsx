import { useEffect, useMemo, useState } from 'react';
import {
  KeyboardAvoidingView,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { Ionicons } from '@expo/vector-icons';
import { router } from 'expo-router';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { isCompactDesktop } from '../src-rn/utils/platform';
import { useAuthStore } from '../src-rn/store/authStore';
import { useTheme } from '../src-rn/theme/useTheme';
import { useDialog } from '../src-rn/components/DialogProvider';
import { Colors } from '../src-rn/theme/colors';
import {
  inputField,
  buttonPrimary,
  buttonDanger,
} from '../src-rn/theme/platformStyles';

export default function KantineLoginScreen() {
  const { colors } = useTheme();
  const { alert } = useDialog();
  const insets = useSafeAreaInsets();
  const styles = useMemo(() => createStyles(colors), [colors]);

  const {
    status: gourmetStatus,
    userInfo,
    login: gourmetLogin,
    logout: gourmetLogout,
    saveCredentials: gourmetSaveCredentials,
    getSavedCredentials: gourmetGetSavedCredentials,
  } = useAuthStore();

  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    (async () => {
      const creds = await gourmetGetSavedCredentials();
      if (creds) {
        setUsername(creds.username);
        setPassword(creds.password);
      }
    })();
  }, [gourmetGetSavedCredentials]);

  const handleSave = async () => {
    if (!username || !password) {
      alert('Fehler', 'Bitte Benutzername und Passwort eingeben');
      return;
    }
    setSaving(true);
    await gourmetSaveCredentials(username, password);
    const success = await gourmetLogin(username, password);
    setSaving(false);
    if (success) {
      alert('Gespeichert', 'Kantine-Zugangsdaten sicher gespeichert');
    } else {
      const error = useAuthStore.getState().error;
      alert('Login fehlgeschlagen', error || 'Anmeldung nicht möglich. Bitte Zugangsdaten prüfen.');
    }
  };

  const handleLogout = async () => {
    await gourmetLogout();
  };

  return (
    <KeyboardAvoidingView style={{ flex: 1 }} behavior={Platform.OS === 'ios' ? 'padding' : undefined}>
      <ScrollView
        style={[styles.container, { paddingTop: insets.top }]}
        contentContainerStyle={styles.content}
        keyboardShouldPersistTaps="handled"
      >
        <Pressable style={styles.backButton} onPress={() => router.back()}>
          <Ionicons name="chevron-back" size={24} color={colors.primary} />
          <Text style={styles.backText}>Einstellungen</Text>
        </Pressable>

        <Text style={styles.pageTitle}>Kantine-Zugangsdaten</Text>

        <View style={styles.inputGroup}>
          <Text style={styles.label}>Benutzername</Text>
          <TextInput
            style={styles.input}
            value={username}
            onChangeText={setUsername}
            placeholder="Benutzername eingeben"
            placeholderTextColor={colors.textTertiary}
            autoCapitalize="none"
            autoCorrect={false}
          />
        </View>

        <View style={styles.inputGroup}>
          <Text style={styles.label}>Passwort</Text>
          <TextInput
            style={styles.input}
            value={password}
            onChangeText={setPassword}
            placeholder="Passwort eingeben"
            placeholderTextColor={colors.textTertiary}
            secureTextEntry
            autoCapitalize="none"
            autoCorrect={false}
          />
        </View>

        <Pressable
          style={[styles.button, styles.buttonPrimary]}
          onPress={handleSave}
          disabled={saving}
        >
          <Text style={styles.buttonPrimaryText}>
            {saving ? 'Speichern...' : 'Speichern'}
          </Text>
        </Pressable>

        {gourmetStatus === 'authenticated' && (
          <View style={styles.sessionSection}>
            <Text style={styles.sessionInfo}>
              Angemeldet als: {userInfo?.username}
            </Text>
            <Pressable style={styles.buttonDanger} onPress={handleLogout}>
              <Text style={styles.buttonDangerText}>Abmelden</Text>
            </Pressable>
          </View>
        )}
      </ScrollView>
    </KeyboardAvoidingView>
  );
}

const createStyles = (c: Colors) =>
  StyleSheet.create({
    container: {
      flex: 1,
      backgroundColor: c.background,
    },
    content: {
      padding: 20,
      paddingBottom: 100,
    },
    backButton: {
      flexDirection: 'row',
      alignItems: 'center',
      marginBottom: 8,
      marginLeft: -6,
    },
    backText: {
      fontSize: 17,
      color: c.primary,
    },
    pageTitle: {
      fontSize: 28,
      fontWeight: '700',
      color: c.textPrimary,
      marginBottom: 24,
    },
    inputGroup: {
      marginBottom: isCompactDesktop ? 10 : 16,
    },
    label: {
      fontSize: isCompactDesktop ? 12 : 13,
      fontWeight: '600',
      color: c.textSecondary,
      marginBottom: isCompactDesktop ? 4 : 6,
    },
    input: {
      fontSize: isCompactDesktop ? 13 : 15,
      color: c.textPrimary,
      ...inputField(c),
    },
    button: {
      paddingVertical: isCompactDesktop ? 8 : 14,
      borderRadius: isCompactDesktop ? 4 : 14,
      alignItems: 'center',
    },
    buttonPrimary: {
      ...buttonPrimary(c),
    },
    buttonPrimaryText: {
      color: '#fff',
      fontWeight: '700',
      fontSize: isCompactDesktop ? 13 : 15,
    },
    buttonDanger: {
      alignSelf: isCompactDesktop ? 'flex-start' as const : undefined,
      alignItems: 'center' as const,
      paddingVertical: isCompactDesktop ? 8 : 14,
      paddingHorizontal: isCompactDesktop ? 16 : 24,
      ...buttonDanger(c),
    },
    buttonDangerText: {
      color: '#fff',
      fontWeight: '700',
      fontSize: isCompactDesktop ? 13 : 15,
    },
    sessionSection: {
      marginTop: isCompactDesktop ? 10 : 16,
    },
    sessionInfo: {
      fontSize: isCompactDesktop ? 13 : 14,
      color: c.textSecondary,
      marginBottom: isCompactDesktop ? 8 : 12,
    },
  });

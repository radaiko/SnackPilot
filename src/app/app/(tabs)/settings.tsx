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
import { useFlatStyle, isCompactDesktop, isNative } from '../../src-rn/utils/platform';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useAuthStore } from '../../src-rn/store/authStore';
import { useVentopayAuthStore } from '../../src-rn/store/ventopayAuthStore';
import { isDesktop } from '../../src-rn/utils/platform';
import { useLocationStore } from '../../src-rn/store/locationStore';
import {
  requestLocationPermissions,
  requestNotificationPermissions,
  getCurrentPosition,
  enableNotifications,
  disableNotifications,
} from '../../src-rn/utils/notificationService';
import { useUpdateStore, applyUpdate } from '../../src-rn/utils/desktopUpdater';
import { useTheme } from '../../src-rn/theme/useTheme';
import { useDesktopLayout } from '../../src-rn/hooks/useDesktopLayout';
import { useDialog } from '../../src-rn/components/DialogProvider';
import { useThemeStore, ThemePreference } from '../../src-rn/store/themeStore';
import { useAnalyticsId } from '../../src-rn/hooks/useAnalyticsId';
import { Colors, ACCENT_COLORS, AccentColorId } from '../../src-rn/theme/colors';
import {
  inputField,
  buttonPrimary,
  buttonDanger,
  bannerSurface,
  cardSurface,
} from '../../src-rn/theme/platformStyles';

const THEME_OPTIONS: { value: ThemePreference; label: string }[] = [
  { value: 'system', label: 'System' },
  { value: 'light', label: 'Hell' },
  { value: 'dark', label: 'Dunkel' },
];

export default function SettingsScreen() {
  const { colors } = useTheme();
  const { alert } = useDialog();
  const insets = useSafeAreaInsets();
  const { isWideLayout } = useDesktopLayout();
  const styles = useMemo(() => createStyles(colors), [colors]);

  const themePreference = useThemeStore((s) => s.preference);
  const setThemePreference = useThemeStore((s) => s.setPreference);
  const accentColor = useThemeStore((s) => s.accentColor);
  const setAccentColor = useThemeStore((s) => s.setAccentColor);

  // Gourmet auth
  const {
    status: gourmetStatus,
    userInfo,
    login: gourmetLogin,
    logout: gourmetLogout,
    saveCredentials: gourmetSaveCredentials,
    getSavedCredentials: gourmetGetSavedCredentials,
  } = useAuthStore();

  // Ventopay auth
  const {
    status: ventopayStatus,
    login: ventopayLogin,
    logout: ventopayLogout,
    saveCredentials: ventopaySaveCredentials,
    getSavedCredentials: ventopayGetSavedCredentials,
  } = useVentopayAuthStore();

  // Gourmet form state
  const [gUsername, setGUsername] = useState('');
  const [gPassword, setGPassword] = useState('');
  const [gSaving, setGSaving] = useState(false);

  // Ventopay form state
  const [vUsername, setVUsername] = useState('');
  const [vPassword, setVPassword] = useState('');
  const [vSaving, setVSaving] = useState(false);

  // Analytics
  const analyticsId = useAnalyticsId();

  // Desktop update state
  const pendingVersion = useUpdateStore((s) => s.pendingVersion);

  // Location notifications (mobile only)
  const companyLocation = useLocationStore((s) => s.companyLocation);
  const setCompanyLocation = useLocationStore((s) => s.setCompanyLocation);
  const clearCompanyLocation = useLocationStore((s) => s.clearCompanyLocation);
  const [locationSaving, setLocationSaving] = useState(false);

  const handleSetLocation = async () => {
    setLocationSaving(true);
    try {
      const locGranted = await requestLocationPermissions();
      if (!locGranted) {
        alert('Berechtigung fehlt', 'Standortzugriff (immer) wird für diese Funktion benötigt. Bitte in den Einstellungen aktivieren.');
        setLocationSaving(false);
        return;
      }
      const notifGranted = await requestNotificationPermissions();
      if (!notifGranted) {
        alert('Berechtigung fehlt', 'Benachrichtigungen werden für diese Funktion benötigt. Bitte in den Einstellungen aktivieren.');
        setLocationSaving(false);
        return;
      }
      const position = await getCurrentPosition();
      setCompanyLocation(position.latitude, position.longitude);
      await enableNotifications();
      alert('Gespeichert', 'Firmenstandort gesetzt. Du wirst um 8:45 benachrichtigt, wenn du im Büro bist und nicht bestellt hast.');
    } catch {
      alert('Fehler', 'Standort konnte nicht ermittelt werden.');
    }
    setLocationSaving(false);
  };

  const handleRemoveLocation = async () => {
    clearCompanyLocation();
    await disableNotifications();
  };

  // Load saved credentials on mount
  useEffect(() => {
    (async () => {
      const gCreds = await gourmetGetSavedCredentials();
      if (gCreds) {
        setGUsername(gCreds.username);
        setGPassword(gCreds.password);
      }
      const vCreds = await ventopayGetSavedCredentials();
      if (vCreds) {
        setVUsername(vCreds.username);
        setVPassword(vCreds.password);
      }
    })();
  }, [gourmetGetSavedCredentials, ventopayGetSavedCredentials]);

  // Gourmet handlers
  const handleGourmetSave = async () => {
    if (!gUsername || !gPassword) {
      alert('Fehler', 'Bitte Benutzername und Passwort eingeben');
      return;
    }
    setGSaving(true);
    await gourmetSaveCredentials(gUsername, gPassword);
    const success = await gourmetLogin(gUsername, gPassword);
    setGSaving(false);
    if (success) {
      alert('Gespeichert', 'Kantine-Zugangsdaten sicher gespeichert');
    } else {
      const error = useAuthStore.getState().error;
      alert('Login fehlgeschlagen', error || 'Anmeldung nicht möglich. Bitte Zugangsdaten prüfen.');
    }
  };

  const handleGourmetLogout = async () => {
    await gourmetLogout();
  };

  // Ventopay handlers
  const handleVentopaySave = async () => {
    if (!vUsername || !vPassword) {
      alert('Fehler', 'Bitte Benutzername und Passwort eingeben');
      return;
    }
    setVSaving(true);
    await ventopaySaveCredentials(vUsername, vPassword);
    const success = await ventopayLogin(vUsername, vPassword);
    setVSaving(false);
    if (success) {
      alert('Gespeichert', 'Automaten-Zugangsdaten sicher gespeichert');
    } else {
      const error = useVentopayAuthStore.getState().error;
      alert('Login fehlgeschlagen', error || 'Anmeldung nicht möglich. Bitte Zugangsdaten prüfen.');
    }
  };

  const handleVentopayLogout = async () => {
    await ventopayLogout();
  };

  const gourmetCard = (
    <View style={isWideLayout ? styles.desktopCard : undefined}>
      <Text style={styles.sectionTitle}>Kantine-Zugangsdaten</Text>

      <View style={styles.inputGroup}>
        <Text style={styles.label}>Benutzername</Text>
        <TextInput
          style={styles.input}
          value={gUsername}
          onChangeText={setGUsername}
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
          value={gPassword}
          onChangeText={setGPassword}
          placeholder="Passwort eingeben"
          placeholderTextColor={colors.textTertiary}
          secureTextEntry
          autoCapitalize="none"
          autoCorrect={false}
        />
      </View>

      <Pressable
        style={[styles.button, styles.buttonPrimary]}
        onPress={handleGourmetSave}
        disabled={gSaving}
      >
        <Text style={styles.buttonPrimaryText}>
          {gSaving ? 'Speichern...' : 'Speichern'}
        </Text>
      </Pressable>

      {gourmetStatus === 'authenticated' && (
        <View style={styles.sessionSection}>
          <Text style={styles.sessionInfo}>
            Angemeldet als: {userInfo?.username}
          </Text>
          <Pressable style={styles.buttonDanger} onPress={handleGourmetLogout}>
            <Text style={styles.buttonDangerText}>Abmelden</Text>
          </Pressable>
        </View>
      )}
    </View>
  );

  const ventopayCard = (
    <View style={isWideLayout ? styles.desktopCard : undefined}>
      {!isWideLayout && <View style={styles.divider} />}
      <Text style={styles.sectionTitle}>Automaten-Zugangsdaten</Text>
      <Text style={styles.sectionSubtitle}>Für Automaten und Kassenabrechnungen</Text>

      <View style={styles.inputGroup}>
        <Text style={styles.label}>Benutzername</Text>
        <TextInput
          style={styles.input}
          value={vUsername}
          onChangeText={setVUsername}
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
          value={vPassword}
          onChangeText={setVPassword}
          placeholder="Passwort eingeben"
          placeholderTextColor={colors.textTertiary}
          secureTextEntry
          autoCapitalize="none"
          autoCorrect={false}
        />
      </View>

      <Pressable
        style={[styles.button, styles.buttonPrimary]}
        onPress={handleVentopaySave}
        disabled={vSaving}
      >
        <Text style={styles.buttonPrimaryText}>
          {vSaving ? 'Speichern...' : 'Speichern'}
        </Text>
      </Pressable>

      {ventopayStatus === 'authenticated' && (
        <View style={styles.sessionSection}>
          <Text style={styles.sessionInfo}>Automaten-Sitzung aktiv</Text>
          <Pressable style={styles.buttonDanger} onPress={handleVentopayLogout}>
            <Text style={styles.buttonDangerText}>Abmelden</Text>
          </Pressable>
        </View>
      )}
    </View>
  );

  const ACCENT_OPTIONS = Object.entries(ACCENT_COLORS) as [AccentColorId, typeof ACCENT_COLORS[AccentColorId]][];

  const appearanceCard = (
    <View style={isWideLayout ? styles.desktopCard : styles.appearanceSection}>
      {!isWideLayout && <View style={styles.divider} />}
      <Text style={styles.sectionTitle}>Darstellung</Text>
      <View style={styles.themeRow}>
        {THEME_OPTIONS.map((opt) => (
          <Pressable
            key={opt.value}
            style={[
              styles.themeOption,
              themePreference === opt.value && styles.themeOptionActive,
            ]}
            onPress={() => setThemePreference(opt.value)}
          >
            <Text
              style={[
                styles.themeOptionText,
                themePreference === opt.value && styles.themeOptionTextActive,
              ]}
            >
              {opt.label}
            </Text>
          </Pressable>
        ))}
      </View>

      <Text style={styles.accentLabel}>Akzentfarbe</Text>
      <View style={styles.accentRow}>
        {ACCENT_OPTIONS.map(([id, config]) => (
          <Pressable
            key={id}
            style={styles.accentOption}
            onPress={() => setAccentColor(id)}
          >
            <View
              style={[
                styles.accentCircle,
                { backgroundColor: config.light.primary },
                accentColor === id && { borderColor: config.light.primary, borderWidth: 3 },
              ]}
            >
              {accentColor === id && (
                <Ionicons name="checkmark" size={isCompactDesktop ? 16 : 20} color="#fff" />
              )}
            </View>
            <Text
              style={[
                styles.accentOptionText,
                accentColor === id && { color: colors.primary },
              ]}
            >
              {config.label}
            </Text>
          </Pressable>
        ))}
      </View>
    </View>
  );

  const privacyCard = (
    <View style={styles.privacyRow}>
      <Pressable
        onPress={() => alert(
          'Datenschutz',
          'Diese App erfasst anonyme Nutzungsanalysen, Fehlerberichte und Sitzungsaufzeichnungen zur Verbesserung der Benutzererfahrung. Alle Daten werden in der EU über PostHog verarbeitet und gespeichert. Es werden keine persönlichen Inhalte (Passwörter, Menüauswahl oder Abrechnungsdaten) erfasst. Texteingaben werden in Sitzungsaufzeichnungen automatisch maskiert.'
        )}
      >
        <Text style={styles.privacyLink}>Datenschutz</Text>
      </Pressable>
      {analyticsId && (
        <Pressable
          onPress={() => alert(
            'Analytics-ID',
            `Deine anonyme Analytics-ID:\n\n${analyticsId}\n\nGib diese ID an, wenn du die Löschung deiner Analysedaten beantragen möchtest.`
          )}
        >
          <Text style={styles.privacyLink}>Analytics-ID</Text>
        </Pressable>
      )}
    </View>
  );

  const updatesCard = isDesktop() && pendingVersion ? (
    <View style={isWideLayout ? styles.desktopCard : undefined}>
      {!isWideLayout && <View style={styles.divider} />}
      <Text style={styles.sectionTitle}>Updates</Text>
      <Text style={styles.updateAvailableText}>
        Version {pendingVersion} ist bereit zur Installation.
      </Text>
      <View style={styles.buttonRow}>
        <Pressable
          style={[styles.button, styles.buttonPrimary]}
          onPress={applyUpdate}
        >
          <Text style={styles.buttonPrimaryText}>Jetzt aktualisieren</Text>
        </Pressable>
      </View>
      <Text style={styles.updateHintText}>
        Das Update wird auch beim nächsten Neustart automatisch angewendet.
      </Text>
    </View>
  ) : null;

  const locationCard = isNative() ? (
    <View style={isWideLayout ? styles.desktopCard : undefined}>
      {!isWideLayout && <View style={styles.divider} />}
      <Text style={styles.sectionTitle}>Standort-Benachrichtigungen</Text>
      <Text style={styles.sectionSubtitle}>
        Erinnerung um 8:45 basierend auf deinem Standort
      </Text>

      {companyLocation ? (
        <View>
          <Text style={styles.sessionInfo}>
            Firmenstandort gesetzt
          </Text>
          <Pressable style={styles.buttonDanger} onPress={handleRemoveLocation}>
            <Text style={styles.buttonDangerText}>Standort entfernen</Text>
          </Pressable>
        </View>
      ) : (
        <Pressable
          style={[styles.button, styles.buttonPrimary]}
          onPress={handleSetLocation}
          disabled={locationSaving}
        >
          <Text style={styles.buttonPrimaryText}>
            {locationSaving ? 'Standort wird ermittelt...' : 'Aktuellen Standort als Firmenstandort setzen'}
          </Text>
        </Pressable>
      )}
    </View>
  ) : null;

  return (
    <KeyboardAvoidingView style={{ flex: 1 }} behavior={Platform.OS === 'ios' ? 'padding' : undefined}>
    <ScrollView style={[styles.container, { paddingTop: insets.top }]} contentContainerStyle={isWideLayout ? styles.contentDesktop : styles.content} keyboardShouldPersistTaps="handled">
      {isWideLayout ? (
        <>
          <View style={styles.desktopRow}>
            {gourmetCard}
            {ventopayCard}
          </View>
          <View style={styles.desktopRow}>
            {appearanceCard}
            {updatesCard}
          </View>
          {locationCard}
          {privacyCard}
        </>
      ) : (
        <>
          {gourmetCard}
          {ventopayCard}
          {appearanceCard}
          {locationCard}
          {updatesCard}
          {privacyCard}
        </>
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
    contentDesktop: {
      padding: 16,
      paddingBottom: 40,
      maxWidth: 900,
      alignSelf: 'center' as const,
      width: '100%',
    },
    desktopRow: {
      flexDirection: 'row' as const,
      gap: 12,
      marginBottom: 12,
    },
    desktopCard: {
      flex: 1,
      padding: 14,
      ...cardSurface(c),
    },
    sectionTitle: {
      fontSize: isCompactDesktop ? 15 : 22,
      fontWeight: '600',
      color: c.textPrimary,
      marginBottom: isCompactDesktop ? 10 : 16,
    },
    sectionSubtitle: {
      fontSize: isCompactDesktop ? 12 : 13,
      color: c.textTertiary,
      marginTop: isCompactDesktop ? -8 : -12,
      marginBottom: isCompactDesktop ? 10 : 16,
    },
    divider: {
      height: 1,
      backgroundColor: useFlatStyle ? c.border : c.glassHighlight,
      marginVertical: 24,
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
    buttonRow: {
      flexDirection: 'row',
      gap: isCompactDesktop ? 8 : 12,
      marginTop: isCompactDesktop ? 6 : 8,
    },
    button: {
      flex: 1,
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
    appearanceSection: {
      marginTop: 32,
    },
    themeRow: {
      flexDirection: 'row',
      gap: 10,
    },
    themeOption: {
      flex: 1,
      paddingVertical: isCompactDesktop ? 7 : 12,
      alignItems: 'center',
      ...bannerSurface(c),
    },
    themeOptionActive: {
      backgroundColor: useFlatStyle ? c.primarySurface : c.glassPrimary,
      borderColor: useFlatStyle ? c.primary : undefined,
      borderBottomColor: c.primary,
    },
    themeOptionText: {
      fontSize: isCompactDesktop ? 12 : 14,
      fontWeight: '600',
      color: c.textSecondary,
    },
    themeOptionTextActive: {
      color: c.primary,
    },
    updateAvailableText: {
      fontSize: isCompactDesktop ? 13 : 14,
      color: c.textSecondary,
      marginBottom: isCompactDesktop ? 8 : 12,
    },
    updateHintText: {
      fontSize: isCompactDesktop ? 11 : 12,
      color: c.textTertiary,
      marginTop: isCompactDesktop ? 6 : 8,
    },
    privacyRow: {
      flexDirection: 'row' as const,
      justifyContent: 'center' as const,
      gap: 24,
    },
    privacyLink: {
      fontSize: isCompactDesktop ? 12 : 14,
      color: c.textTertiary,
      textAlign: 'center' as const,
      paddingVertical: 16,
    },
    accentLabel: {
      fontSize: isCompactDesktop ? 12 : 13,
      fontWeight: '600',
      color: c.textSecondary,
      marginTop: isCompactDesktop ? 14 : 20,
      marginBottom: isCompactDesktop ? 8 : 12,
    },
    accentRow: {
      flexDirection: 'row',
      justifyContent: 'flex-start',
      gap: isCompactDesktop ? 12 : 16,
    },
    accentOption: {
      alignItems: 'center',
      gap: isCompactDesktop ? 4 : 6,
    },
    accentCircle: {
      width: isCompactDesktop ? 28 : 36,
      height: isCompactDesktop ? 28 : 36,
      borderRadius: isCompactDesktop ? 14 : 18,
      borderWidth: 2,
      borderColor: 'transparent',
      alignItems: 'center',
      justifyContent: 'center',
    },
    accentCheck: {
      color: '#fff',
      fontSize: isCompactDesktop ? 13 : 16,
      fontWeight: '700',
    },
    accentOptionText: {
      fontSize: isCompactDesktop ? 10 : 12,
      color: c.textTertiary,
      fontWeight: '500',
    },
  });

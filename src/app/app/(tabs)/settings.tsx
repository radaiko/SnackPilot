import { useCallback, useMemo } from 'react';
import {
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { Ionicons } from '@expo/vector-icons';
import { router, useFocusEffect } from 'expo-router';
import { useFlatStyle, isCompactDesktop, isNative } from '../../src-rn/utils/platform';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useAuthStore } from '../../src-rn/store/authStore';
import { useVentopayAuthStore } from '../../src-rn/store/ventopayAuthStore';
import { isDesktop } from '../../src-rn/utils/platform';
import { useUpdateStore, applyUpdate } from '../../src-rn/utils/desktopUpdater';
import { useTheme } from '../../src-rn/theme/useTheme';
import { useDesktopLayout } from '../../src-rn/hooks/useDesktopLayout';
import { useDialog } from '../../src-rn/components/DialogProvider';
import { useThemeStore } from '../../src-rn/store/themeStore';
import { Colors } from '../../src-rn/theme/colors';
import {
  buttonPrimary,
  bannerSurface,
  cardSurface,
} from '../../src-rn/theme/platformStyles';
import { trackSignal } from '../../src-rn/utils/analytics';

const THEME_LABELS: Record<string, string> = {
  system: 'System',
  light: 'Hell',
  dark: 'Dunkel',
};

export default function SettingsScreen() {
  const { colors } = useTheme();
  const { alert } = useDialog();
  const insets = useSafeAreaInsets();
  const { isWideLayout } = useDesktopLayout();
  const styles = useMemo(() => createStyles(colors), [colors]);

  const themePreference = useThemeStore((s) => s.preference);

  const gourmetStatus = useAuthStore((s) => s.status);
  const gourmetUsername = useAuthStore((s) => s.userInfo?.username);
  const ventopayStatus = useVentopayAuthStore((s) => s.status);

  const pendingVersion = useUpdateStore((s) => s.pendingVersion);

  useFocusEffect(
    useCallback(() => {
      trackSignal('screen.viewed', { screen: 'settings' });
    }, [])
  );

  const gourmetCard = (
    <View style={isWideLayout ? styles.desktopCard : undefined}>
      <Pressable style={styles.navRow} onPress={() => router.push('/kantine-login')}>
        <View style={{ flex: 1 }}>
          <Text style={styles.sectionTitle}>Kantine-Zugangsdaten</Text>
          <Text style={styles.navHint}>
            {gourmetStatus === 'authenticated'
              ? `Angemeldet als ${gourmetUsername}`
              : 'Nicht angemeldet'}
          </Text>
        </View>
        <Ionicons name="chevron-forward" size={20} color={colors.textTertiary} />
      </Pressable>
    </View>
  );

  const ventopayCard = (
    <View style={isWideLayout ? styles.desktopCard : undefined}>
      {!isWideLayout && <View style={styles.divider} />}
      <Pressable style={styles.navRow} onPress={() => router.push('/automaten-login')}>
        <View style={{ flex: 1 }}>
          <Text style={styles.sectionTitle}>Automaten-Zugangsdaten</Text>
          <Text style={styles.navHint}>
            {ventopayStatus === 'authenticated'
              ? 'Sitzung aktiv'
              : 'Nicht angemeldet'}
          </Text>
        </View>
        <Ionicons name="chevron-forward" size={20} color={colors.textTertiary} />
      </Pressable>
    </View>
  );

  const appearanceCard = (
    <View style={isWideLayout ? styles.desktopCard : undefined}>
      {!isWideLayout && <View style={styles.divider} />}
      <Pressable style={styles.navRow} onPress={() => router.push('/appearance')}>
        <View style={{ flex: 1 }}>
          <Text style={styles.sectionTitle}>Darstellung</Text>
          <Text style={styles.navHint}>
            {THEME_LABELS[themePreference] || themePreference}
          </Text>
        </View>
        <Ionicons name="chevron-forward" size={20} color={colors.textTertiary} />
      </Pressable>
    </View>
  );

  const notificationCard = isNative() ? (
    <View style={isWideLayout ? styles.desktopCard : undefined}>
      {!isWideLayout && <View style={styles.divider} />}
      <Pressable style={styles.navRow} onPress={() => router.push('/notifications')}>
        <View style={{ flex: 1 }}>
          <Text style={styles.sectionTitle}>Benachrichtigungen</Text>
          <Text style={styles.navHint}>Erinnerungen und Standort-Benachrichtigungen</Text>
        </View>
        <Ionicons name="chevron-forward" size={20} color={colors.textTertiary} />
      </Pressable>
    </View>
  ) : null;

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

  const privacyCard = (
    <View style={styles.privacyRow}>
      <Pressable
        onPress={() => alert(
          'Datenschutz',
          'Diese App erfasst anonyme Nutzungsstatistiken zur Verbesserung der Benutzererfahrung. Die Analyse erfolgt über TelemetryDeck — einen datenschutzfreundlichen, cookielosen Dienst. Es werden keine persönlichen Daten, Passwörter, Menüauswahl oder Abrechnungsdaten erfasst.'
        )}
      >
        <Text style={styles.privacyLink}>Datenschutz</Text>
      </Pressable>
    </View>
  );

  return (
    <ScrollView style={[styles.container, { paddingTop: insets.top }]} contentContainerStyle={isWideLayout ? styles.contentDesktop : styles.content}>
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
          {notificationCard}
          {privacyCard}
        </>
      ) : (
        <>
          {gourmetCard}
          {ventopayCard}
          {appearanceCard}
          {notificationCard}
          {updatesCard}
          {privacyCard}
        </>
      )}
    </ScrollView>
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
      fontSize: isCompactDesktop ? 15 : 18,
      fontWeight: '600',
      color: c.textPrimary,
    },
    divider: {
      height: 1,
      backgroundColor: useFlatStyle ? c.border : c.glassHighlight,
      marginVertical: 24,
    },
    navRow: {
      flexDirection: 'row' as const,
      alignItems: 'center' as const,
      paddingVertical: isCompactDesktop ? 4 : 8,
    },
    navHint: {
      fontSize: isCompactDesktop ? 11 : 13,
      color: c.textTertiary,
      marginTop: 2,
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
  });

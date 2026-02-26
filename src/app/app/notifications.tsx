import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Linking,
  Pressable,
  ScrollView,
  StyleSheet,
  Switch,
  Text,
  View,
} from 'react-native';
import { Ionicons } from '@expo/vector-icons';
import { router, useFocusEffect } from 'expo-router';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useFlatStyle, isCompactDesktop, isNative } from '../src-rn/utils/platform';
import { useLocationStore } from '../src-rn/store/locationStore';
import {
  requestLocationPermissions,
  requestNotificationPermissions,
  hasBackgroundLocationPermission,
  getCurrentPosition,
  enableNotifications,
  disableNotifications,
  registerBackgroundSync,
} from '../src-rn/utils/notificationService';
import {
  getReminderEnabled,
  setReminderEnabled,
  getReminderTime,
  setReminderTime,
} from '../src-rn/utils/reminderStorage';
import {
  activateLog,
  clearLog,
  getLogActivatedUntil,
  getLogEntries,
  formatLogForEmail,
  NotificationLogEntry,
} from '../src-rn/utils/notificationLogStorage';
import { useTheme } from '../src-rn/theme/useTheme';
import { useDialog } from '../src-rn/components/DialogProvider';
import { Colors } from '../src-rn/theme/colors';
import { trackSignal } from '../src-rn/utils/analytics';
import {
  buttonPrimary,
  buttonDanger,
  bannerSurface,
} from '../src-rn/theme/platformStyles';

const TIME_OPTIONS: { hour: number; minute: number; label: string }[] = [];
for (let h = 11; h <= 13; h++) {
  for (let m = 0; m < 60; m += 15) {
    TIME_OPTIONS.push({
      hour: h,
      minute: m,
      label: `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`,
    });
  }
}

export default function NotificationsScreen() {
  const { colors } = useTheme();
  const { alert } = useDialog();
  const insets = useSafeAreaInsets();
  const styles = useMemo(() => createStyles(colors), [colors]);

  // Location notifications
  const companyLocation = useLocationStore((s) => s.companyLocation);
  const setCompanyLocation = useLocationStore((s) => s.setCompanyLocation);
  const clearCompanyLocation = useLocationStore((s) => s.clearCompanyLocation);
  const [locationSaving, setLocationSaving] = useState(false);

  // Daily reminder state
  const [reminderEnabled, setReminderEnabledState] = useState(false);
  const [reminderHour, setReminderHour] = useState(11);
  const [reminderMinute, setReminderMinute] = useState(0);

  // Debug log state
  const [logActivatedUntil, setLogActivatedUntil] = useState<number | null>(null);
  const [logEntries, setLogEntries] = useState<NotificationLogEntry[]>([]);

  const logIsActive = logActivatedUntil !== null && Date.now() < logActivatedUntil;
  const logIsExpired = logActivatedUntil !== null && Date.now() >= logActivatedUntil;

  const loadLogState = useCallback(async () => {
    const until = await getLogActivatedUntil();
    setLogActivatedUntil(until);
    if (until !== null) {
      const entries = await getLogEntries();
      setLogEntries(entries);
    }
  }, []);

  useEffect(() => {
    if (!isNative()) return;
    (async () => {
      const enabled = await getReminderEnabled();
      setReminderEnabledState(enabled);
      const time = await getReminderTime();
      if (time) {
        setReminderHour(time.hour);
        setReminderMinute(time.minute);
      }
    })();
    loadLogState();
  }, [loadLogState]);

  // Refresh log state when screen is focused (user may come back after 24h)
  useFocusEffect(
    useCallback(() => {
      loadLogState();
    }, [loadLogState])
  );

  const handleReminderToggle = async (newValue: boolean) => {
    if (newValue) {
      const granted = await requestNotificationPermissions();
      if (!granted) {
        alert('Berechtigung fehlt', 'Benachrichtigungen werden für diese Funktion benötigt. Bitte in den Einstellungen aktivieren.');
        return;
      }
      await setReminderTime(reminderHour, reminderMinute);
      await registerBackgroundSync();
    }
    await setReminderEnabled(newValue);
    setReminderEnabledState(newValue);
    trackSignal('notification.reminderToggled', {
      enabled: String(newValue),
      hour: String(reminderHour),
      minute: String(reminderMinute),
    });
  };

  const handleReminderTimeChange = async (hour: number, minute: number) => {
    setReminderHour(hour);
    setReminderMinute(minute);
    await setReminderTime(hour, minute);
  };

  const openAppSettings = () => {
    Linking.openSettings();
  };

  const handleSetLocation = async () => {
    setLocationSaving(true);
    try {
      const locGranted = await requestLocationPermissions();
      if (!locGranted) {
        // Check if foreground was granted but background wasn't (iOS "While Using" only)
        const hasBg = await hasBackgroundLocationPermission();
        if (!hasBg) {
          alert(
            'Standort „Immer" erforderlich',
            'Für Standort-Benachrichtigungen muss der Standortzugriff auf „Immer" gesetzt werden.\n\nBitte öffne die Einstellungen und wähle unter Standort „Immer" aus.',
          );
          openAppSettings();
        }
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
      trackSignal('notification.locationSet');
    } catch {
      alert('Fehler', 'Standort konnte nicht ermittelt werden.');
    }
    setLocationSaving(false);
  };

  const handleRemoveLocation = async () => {
    clearCompanyLocation();
    await disableNotifications();
  };

  const handleActivateLog = async () => {
    await activateLog();
    await loadLogState();
  };

  const handleSendLog = async () => {
    const body = formatLogForEmail(logEntries);
    const expiryStr = logActivatedUntil
      ? new Date(logActivatedUntil).toLocaleString('de-AT')
      : '';
    const subject = encodeURIComponent(`SnackPilot Notification Log (bis ${expiryStr})`);
    const encodedBody = encodeURIComponent(body);
    const url = `mailto:aiko@spitzbub.app?subject=${subject}&body=${encodedBody}`;
    try {
      await Linking.openURL(url);
    } catch {
      alert('Fehler', 'E-Mail-App konnte nicht geöffnet werden.');
    }
  };

  const handleClearLog = async () => {
    await clearLog();
    setLogActivatedUntil(null);
    setLogEntries([]);
  };

  return (
    <ScrollView
      style={[styles.container, { paddingTop: insets.top }]}
      contentContainerStyle={styles.content}
    >
      <Pressable style={styles.backButton} onPress={() => router.back()}>
        <Ionicons name="chevron-back" size={24} color={colors.primary} />
        <Text style={styles.backText}>Einstellungen</Text>
      </Pressable>

      <Text style={styles.pageTitle}>Benachrichtigungen</Text>

      {/* Daily Reminder */}
      <Text style={styles.sectionTitle}>Bestell-Erinnerung</Text>
      <Text style={styles.sectionHint}>
        Tägliche Erinnerung an deine Bestellung
      </Text>

      <View style={styles.toggleRow}>
        <Text style={styles.label}>Aktiviert</Text>
        <Switch
          value={reminderEnabled}
          onValueChange={handleReminderToggle}
          trackColor={{ false: colors.border, true: colors.primary }}
        />
      </View>

      {reminderEnabled && (
        <View style={styles.timePickerSection}>
          <Text style={styles.label}>Uhrzeit</Text>
          <ScrollView
            horizontal
            showsHorizontalScrollIndicator={false}
            style={styles.timeScroll}
            contentContainerStyle={styles.timeScrollContent}
          >
            {TIME_OPTIONS.map((opt) => {
              const isSelected = opt.hour === reminderHour && opt.minute === reminderMinute;
              return (
                <Pressable
                  key={opt.label}
                  style={[styles.timeChip, isSelected && styles.timeChipActive]}
                  onPress={() => handleReminderTimeChange(opt.hour, opt.minute)}
                >
                  <Text style={[styles.timeChipText, isSelected && styles.timeChipTextActive]}>
                    {opt.label}
                  </Text>
                </Pressable>
              );
            })}
          </ScrollView>
        </View>
      )}

      <View style={styles.divider} />

      {/* Location Notifications */}
      <Text style={styles.sectionTitle}>Standort-Benachrichtigungen</Text>
      <Text style={styles.sectionHint}>
        Erinnerung um 8:45 basierend auf deinem Standort
      </Text>

      {companyLocation ? (
        <View>
          <Text style={styles.statusText}>
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

      <View style={styles.divider} />

      {/* Debug Log */}
      <Text style={styles.sectionTitle}>Benachrichtigungs-Log</Text>
      <Text style={styles.sectionHint}>
        Zeichnet 24 Stunden lang Diagnose-Daten auf, um Probleme mit Benachrichtigungen zu analysieren.
      </Text>

      {logIsActive ? (
        <View>
          <Text style={styles.statusText}>
            Aufzeichnung läuft bis{' '}
            {new Date(logActivatedUntil!).toLocaleString('de-AT', {
              day: '2-digit',
              month: '2-digit',
              hour: '2-digit',
              minute: '2-digit',
            })}
            {logEntries.length > 0 ? ` (${logEntries.length} Einträge)` : ''}
          </Text>
        </View>
      ) : logIsExpired ? (
        <View>
          <Text style={styles.statusText}>
            Aufzeichnung abgeschlossen ({logEntries.length} Einträge).
          </Text>
          <Pressable
            style={[styles.button, styles.buttonPrimary, styles.logSendButton]}
            onPress={handleSendLog}
          >
            <Ionicons name="mail-outline" size={isCompactDesktop ? 14 : 16} color="#fff" />
            <Text style={styles.buttonPrimaryText}>Log per E-Mail senden</Text>
          </Pressable>
          <Pressable style={styles.logDiscardButton} onPress={handleClearLog}>
            <Text style={styles.logDiscardText}>Log verwerfen</Text>
          </Pressable>
        </View>
      ) : (
        <Pressable
          style={[styles.button, styles.buttonPrimary]}
          onPress={handleActivateLog}
        >
          <Text style={styles.buttonPrimaryText}>Log aktivieren (24 Stunden)</Text>
        </Pressable>
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
    sectionTitle: {
      fontSize: isCompactDesktop ? 15 : 18,
      fontWeight: '600',
      color: c.textPrimary,
      marginBottom: 4,
    },
    sectionHint: {
      fontSize: isCompactDesktop ? 11 : 13,
      color: c.textTertiary,
      marginBottom: isCompactDesktop ? 10 : 16,
    },
    divider: {
      height: 1,
      backgroundColor: useFlatStyle ? c.border : c.glassHighlight,
      marginVertical: 24,
    },
    toggleRow: {
      flexDirection: 'row',
      alignItems: 'center',
      justifyContent: 'space-between',
      marginBottom: isCompactDesktop ? 8 : 12,
    },
    label: {
      fontSize: isCompactDesktop ? 12 : 15,
      fontWeight: '600',
      color: c.textSecondary,
    },
    timePickerSection: {
      marginBottom: isCompactDesktop ? 10 : 16,
    },
    timeScroll: {
      marginTop: isCompactDesktop ? 4 : 8,
    },
    timeScrollContent: {
      gap: isCompactDesktop ? 6 : 8,
    },
    timeChip: {
      paddingHorizontal: isCompactDesktop ? 10 : 14,
      paddingVertical: isCompactDesktop ? 6 : 10,
      ...bannerSurface(c),
    },
    timeChipActive: {
      backgroundColor: useFlatStyle ? c.primarySurface : c.glassPrimary,
      borderColor: useFlatStyle ? c.primary : undefined,
      borderBottomColor: c.primary,
    },
    timeChipText: {
      fontSize: isCompactDesktop ? 12 : 14,
      fontWeight: '600',
      color: c.textSecondary,
    },
    timeChipTextActive: {
      color: c.primary,
    },
    statusText: {
      fontSize: isCompactDesktop ? 13 : 14,
      color: c.textSecondary,
      marginBottom: isCompactDesktop ? 8 : 12,
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
    logSendButton: {
      flexDirection: 'row',
      gap: isCompactDesktop ? 6 : 8,
      justifyContent: 'center',
    },
    logDiscardButton: {
      alignItems: 'center' as const,
      paddingVertical: isCompactDesktop ? 8 : 12,
      marginTop: isCompactDesktop ? 4 : 8,
    },
    logDiscardText: {
      fontSize: isCompactDesktop ? 12 : 14,
      color: c.textTertiary,
    },
  });

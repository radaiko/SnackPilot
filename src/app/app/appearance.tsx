import { useMemo } from 'react';
import {
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { Ionicons } from '@expo/vector-icons';
import { router } from 'expo-router';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useFlatStyle, isCompactDesktop } from '../src-rn/utils/platform';
import { useTheme } from '../src-rn/theme/useTheme';
import { useThemeStore, ThemePreference } from '../src-rn/store/themeStore';
import { Colors, ACCENT_COLORS, AccentColorId } from '../src-rn/theme/colors';
import { bannerSurface, cardSurface } from '../src-rn/theme/platformStyles';

const THEME_OPTIONS: { value: ThemePreference; label: string; icon: string }[] = [
  { value: 'system', label: 'System', icon: 'phone-portrait-outline' },
  { value: 'light', label: 'Hell', icon: 'sunny-outline' },
  { value: 'dark', label: 'Dunkel', icon: 'moon-outline' },
];

const ACCENT_OPTIONS = Object.entries(ACCENT_COLORS) as [AccentColorId, typeof ACCENT_COLORS[AccentColorId]][];

export default function AppearanceScreen() {
  const { colors } = useTheme();
  const insets = useSafeAreaInsets();
  const styles = useMemo(() => createStyles(colors), [colors]);

  const themePreference = useThemeStore((s) => s.preference);
  const setThemePreference = useThemeStore((s) => s.setPreference);
  const accentColor = useThemeStore((s) => s.accentColor);
  const setAccentColor = useThemeStore((s) => s.setAccentColor);

  return (
    <ScrollView
      style={[styles.container, { paddingTop: insets.top }]}
      contentContainerStyle={styles.content}
    >
      <Pressable style={styles.backButton} onPress={() => router.back()}>
        <Ionicons name="chevron-back" size={24} color={colors.primary} />
        <Text style={styles.backText}>Einstellungen</Text>
      </Pressable>

      <Text style={styles.pageTitle}>Darstellung</Text>

      <View style={styles.card}>
        <Text style={styles.sectionTitle}>Design</Text>
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
              <Ionicons
                name={opt.icon as any}
                size={isCompactDesktop ? 18 : 22}
                color={themePreference === opt.value ? colors.primary : colors.textSecondary}
                style={styles.themeIcon}
              />
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
      </View>

      <View style={styles.card}>
        <Text style={styles.sectionTitle}>Akzentfarbe</Text>
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
    card: {
      padding: isCompactDesktop ? 14 : 20,
      marginBottom: isCompactDesktop ? 12 : 16,
      ...cardSurface(c),
    },
    sectionTitle: {
      fontSize: isCompactDesktop ? 15 : 18,
      fontWeight: '600',
      color: c.textPrimary,
      marginBottom: isCompactDesktop ? 10 : 14,
    },
    themeRow: {
      flexDirection: 'row',
      gap: 10,
    },
    themeOption: {
      flex: 1,
      paddingVertical: isCompactDesktop ? 10 : 16,
      alignItems: 'center',
      ...bannerSurface(c),
    },
    themeOptionActive: {
      backgroundColor: useFlatStyle ? c.primarySurface : c.glassPrimary,
      borderColor: useFlatStyle ? c.primary : undefined,
      borderBottomColor: c.primary,
    },
    themeIcon: {
      marginBottom: isCompactDesktop ? 4 : 6,
    },
    themeOptionText: {
      fontSize: isCompactDesktop ? 12 : 14,
      fontWeight: '600',
      color: c.textSecondary,
    },
    themeOptionTextActive: {
      color: c.primary,
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
      width: isCompactDesktop ? 28 : 40,
      height: isCompactDesktop ? 28 : 40,
      borderRadius: isCompactDesktop ? 14 : 20,
      borderWidth: 2,
      borderColor: 'transparent',
      alignItems: 'center',
      justifyContent: 'center',
    },
    accentOptionText: {
      fontSize: isCompactDesktop ? 10 : 12,
      color: c.textTertiary,
      fontWeight: '500',
    },
  });

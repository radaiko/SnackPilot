import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ActivityIndicator,
  Animated,
  FlatList,
  PanResponder,
  Platform,
  Pressable,
  StyleSheet,
  Text,
  useWindowDimensions,
  View,
} from 'react-native';
import { useNavigation } from 'expo-router';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useAuthStore } from '../../src-rn/store/authStore';
import { useMenuStore, OrderProgress } from '../../src-rn/store/menuStore';
import { useOrderStore } from '../../src-rn/store/orderStore';
import { MenuCard } from '../../src-rn/components/MenuCard';
import { DayNavigator } from '../../src-rn/components/DayNavigator';
import { DateListPanel } from '../../src-rn/components/DateListPanel';
import { LoadingOverlay } from '../../src-rn/components/LoadingOverlay';
import { GourmetMenuItem, GourmetMenuCategory } from '../../src-rn/types/menu';
import { formatGourmetDate, localDateKey } from '../../src-rn/utils/dateUtils';
import { isCompactDesktop } from '../../src-rn/utils/platform';
import { useTheme } from '../../src-rn/theme/useTheme';
import { useDesktopLayout } from '../../src-rn/hooks/useDesktopLayout';
import { Colors } from '../../src-rn/theme/colors';
import { tintedBanner, buttonPrimary, fab as fabStyle } from '../../src-rn/theme/platformStyles';
import { computeFingerprints, detectNewMenus } from '../../src-rn/utils/menuFingerprint';
import { trackSignal } from '../../src-rn/utils/analytics';
import {
  getKnownMenus,
  setKnownMenus,
  getNotificationSent,
  setNotificationSent,
} from '../../src-rn/utils/menuChangeStorage';
import { NewMenuToast } from '../../src-rn/components/NewMenuToast';

const ORDER_PROGRESS_LABELS: Record<NonNullable<OrderProgress>, string> = {
  adding: 'Wird in den Warenkorb gelegt...',
  confirming: 'Bestellung wird bestätigt...',
  cancelling: 'Bestellung wird storniert...',
  refreshing: 'Menüs werden aktualisiert...',
};

const CATEGORY_ORDER = [
  GourmetMenuCategory.Menu1,
  GourmetMenuCategory.Menu2,
  GourmetMenuCategory.Menu3,
  GourmetMenuCategory.SoupAndSalad,
  GourmetMenuCategory.Unknown,
];

export default function MenusScreen() {
  const { colors } = useTheme();
  const insets = useSafeAreaInsets();
  const { isWideLayout, panelWidth } = useDesktopLayout();
  const styles = useMemo(() => createStyles(colors), [colors]);

  const navigation = useNavigation();
  const { status: authStatus } = useAuthStore();
  const {
    items,
    loading,
    refreshing,
    error,
    selectedDate,
    pendingOrders,
    pendingCancellations,
    orderProgress,
    fetchMenus,
    refreshAvailability,
    setSelectedDate,
    togglePendingOrder,
    submitOrders,
    getAvailableDates,
    getMenusForDate,
    getPendingCount,
    getPendingCancellationCount,
  } = useMenuStore();

  const { orders, fetchOrders } = useOrderStore();

  const [showToast, setShowToast] = useState(false);

  const triggerRefresh = useCallback(() => {
    const auth = useAuthStore.getState().status;
    if (auth !== 'authenticated') return;

    const { loadCachedMenus } = useMenuStore.getState();
    const { loadCachedOrders } = useOrderStore.getState();

    // Load cache first for instant display
    Promise.all([loadCachedMenus(), loadCachedOrders()]).catch(() => {}).finally(() => {
      const cached = useMenuStore.getState().items.length > 0;
      const refreshPromise = cached ? refreshAvailability() : fetchMenus();

      // After menu fetch completes, check for new menus
      refreshPromise?.then(async () => {
        try {
          const currentItems = useMenuStore.getState().items;
          if (currentItems.length === 0) return;

          const currentFingerprints = computeFingerprints(currentItems);
          const knownMenus = await getKnownMenus();
          const notificationSent = await getNotificationSent();

          if (detectNewMenus(currentFingerprints, knownMenus) && !notificationSent) {
            setShowToast(true);
            await setNotificationSent(true);
          }

          // Acknowledge: update known menus and reset notification flag
          await setKnownMenus(currentFingerprints);
          await setNotificationSent(false);
        } catch {
          // Silent — don't break menu loading for fingerprint errors
        }
      });

      fetchOrders();
    });
  }, [fetchMenus, refreshAvailability, fetchOrders]);

  useEffect(() => {
    const unsubscribe = navigation.addListener('focus', () => {
      trackSignal('screen.viewed', { screen: 'menus' });
      triggerRefresh();
    });
    return unsubscribe;
  }, [navigation, triggerRefresh]);

  useEffect(() => {
    if (authStatus === 'authenticated') {
      triggerRefresh();
    }
  }, [authStatus, triggerRefresh]);

  const dates = getAvailableDates();
  const menuItems = getMenusForDate(selectedDate);
  const pendingCount = getPendingCount();
  const cancellationCount = getPendingCancellationCount();
  const newOrderCount = pendingCount - cancellationCount;

  const fabLabel = useMemo(() => {
    if (cancellationCount > 0 && newOrderCount > 0) {
      return `Änderungen bestätigen (${pendingCount})`;
    }
    if (cancellationCount > 0) {
      return `Stornieren (${cancellationCount})`;
    }
    return `Bestellen (${newOrderCount})`;
  }, [pendingCount, cancellationCount, newOrderCount]);

  // Collect ordered categories for the selected date (marks items as ordered in the UI)
  const orderedCategories = useMemo(() => {
    const orderedCats = new Set<GourmetMenuCategory>();
    for (const item of menuItems) {
      if (item.ordered) orderedCats.add(item.category);
    }
    const selectedKey = selectedDate.toDateString();
    for (const o of orders) {
      if (o.date.toDateString() === selectedKey) {
        orderedCats.add(o.title as GourmetMenuCategory);
      }
    }
    return orderedCats;
  }, [menuItems, orders, selectedDate]);

  const grouped = CATEGORY_ORDER.map((cat) => ({
    category: cat,
    items: menuItems.filter((item) => item.category === cat),
  })).filter((group) => group.items.length > 0);

  const isPending = useCallback(
    (item: GourmetMenuItem) => {
      const key = `${item.id}|${localDateKey(item.day)}`;
      return pendingOrders.has(key);
    },
    [pendingOrders]
  );

  const isPendingCancel = useCallback(
    (item: GourmetMenuItem) => {
      const key = `${item.id}|${localDateKey(item.day)}`;
      return pendingCancellations.has(key);
    },
    [pendingCancellations]
  );

  // ── Swipe-between-days gesture ──
  const { width: screenWidth } = useWindowDimensions();
  const translateX = useRef(new Animated.Value(0)).current;
  const currentIndex = dates.findIndex(
    (d) => d.toDateString() === selectedDate.toDateString()
  );
  // Use refs so PanResponder callbacks always see latest values
  const currentIndexRef = useRef(currentIndex);
  currentIndexRef.current = currentIndex;
  const datesRef = useRef(dates);
  datesRef.current = dates;

  const SWIPE_THRESHOLD = 50;

  const panResponder = useMemo(
    () =>
      PanResponder.create({
        onMoveShouldSetPanResponder: (_, gs) =>
          Math.abs(gs.dx) > Math.abs(gs.dy) && Math.abs(gs.dx) > 10,
        onPanResponderMove: (_, gs) => {
          const idx = currentIndexRef.current;
          const len = datesRef.current.length;
          let dx = gs.dx;
          // Rubber-band resistance at edges
          if ((idx <= 0 && dx > 0) || (idx >= len - 1 && dx < 0)) {
            dx *= 0.3;
          }
          translateX.setValue(dx);
        },
        onPanResponderRelease: (_, gs) => {
          const idx = currentIndexRef.current;
          const d = datesRef.current;
          if (gs.dx > SWIPE_THRESHOLD && idx > 0) {
            // Swipe right → previous day
            Animated.timing(translateX, {
              toValue: screenWidth,
              duration: 180,
              useNativeDriver: true,
            }).start(() => {
              setSelectedDate(d[idx - 1]);
              translateX.setValue(-screenWidth);
              Animated.spring(translateX, {
                toValue: 0,
                useNativeDriver: true,
                tension: 65,
                friction: 11,
              }).start();
            });
          } else if (gs.dx < -SWIPE_THRESHOLD && idx < d.length - 1) {
            // Swipe left → next day
            Animated.timing(translateX, {
              toValue: -screenWidth,
              duration: 180,
              useNativeDriver: true,
            }).start(() => {
              setSelectedDate(d[idx + 1]);
              translateX.setValue(screenWidth);
              Animated.spring(translateX, {
                toValue: 0,
                useNativeDriver: true,
                tension: 65,
                friction: 11,
              }).start();
            });
          } else {
            // Below threshold — snap back
            Animated.spring(translateX, {
              toValue: 0,
              useNativeDriver: true,
            }).start();
          }
        },
      }),
    [translateX, screenWidth, setSelectedDate]
  );

  if (authStatus === 'idle' || authStatus === 'loading') {
    return (
      <View style={styles.center}>
        <LoadingOverlay />
      </View>
    );
  }

  if (authStatus === 'error' || authStatus === 'no_credentials') {
    return (
      <View style={styles.center}>
        <Text style={styles.errorText}>Nicht angemeldet</Text>
        <Text style={styles.hintText}>Gehe zu Einstellungen, um Zugangsdaten einzugeben</Text>
      </View>
    );
  }

  const menuContent = (
    <>
      {refreshing && !orderProgress && (
        <View style={styles.refreshBanner}>
          <ActivityIndicator size="small" color={colors.primary} />
          <Text style={styles.refreshBannerText}>Aktualisiere...</Text>
        </View>
      )}

      {orderProgress && (
        <View style={styles.orderProgressBanner}>
          <ActivityIndicator size="small" color="#fff" />
          <Text style={styles.orderProgressText}>
            {ORDER_PROGRESS_LABELS[orderProgress]}
          </Text>
        </View>
      )}

      {loading && !orderProgress && <LoadingOverlay />}

      {error && (
        <View style={styles.errorBanner}>
          <Text style={styles.errorBannerText}>{error}</Text>
        </View>
      )}

      <FlatList
        data={grouped}
        keyExtractor={(group) => group.category}
        contentContainerStyle={isWideLayout ? styles.listDesktop : styles.list}
        renderItem={({ item: group }) => (
          <View style={styles.categoryGroup}>
            {group.category !== GourmetMenuCategory.SoupAndSalad && (
              <Text style={styles.categoryTitle}>{group.category}</Text>
            )}
            {group.items.map((item) => {
              const isOrdered = item.ordered || orderedCategories.has(item.category);
              return (
                <MenuCard
                  key={`${item.id}-${formatGourmetDate(item.day)}`}
                  item={item}
                  isSelected={isPending(item)}
                  ordered={isOrdered}
                  isPendingCancel={isPendingCancel(item)}
                  onToggle={() => togglePendingOrder(item.id, item.day)}
                />
              );
            })}
          </View>
        )}
        ListEmptyComponent={
          !loading ? (
            <View style={styles.center}>
              <Text style={styles.emptyText}>Keine Menüs verfügbar</Text>
            </View>
          ) : null
        }
      />
    </>
  );

  if (isWideLayout) {
    return (
      <View style={styles.container}>
        <NewMenuToast visible={showToast} onDismiss={() => setShowToast(false)} />
        <View style={styles.desktopRow}>
          {dates.length > 0 && (
            <DateListPanel
              dates={dates}
              selectedDate={selectedDate}
              onSelectDate={setSelectedDate}
              width={panelWidth}
            />
          )}
          <View style={styles.desktopMain}>
            {menuContent}
          </View>
        </View>

        {pendingCount > 0 && !orderProgress && (
          <Pressable style={styles.fabDesktop} onPress={submitOrders}>
            <Text style={styles.fabText}>{fabLabel}</Text>
          </Pressable>
        )}
      </View>
    );
  }

  return (
    <View style={[styles.container, { paddingTop: insets.top }]}>
      <NewMenuToast visible={showToast} onDismiss={() => setShowToast(false)} />
      {dates.length > 0 && (
        <DayNavigator
          dates={dates}
          selectedDate={selectedDate}
          onSelectDate={setSelectedDate}
        />
      )}

      <Animated.View
        {...panResponder.panHandlers}
        style={[styles.swipeContainer, { transform: [{ translateX }] }]}
      >
        {menuContent}
      </Animated.View>

      {pendingCount > 0 && !orderProgress && (
        <Pressable style={styles.fab} onPress={submitOrders}>
          <Text style={styles.fabText}>{fabLabel}</Text>
        </Pressable>
      )}
    </View>
  );
}

const createStyles = (c: Colors) =>
  StyleSheet.create({
    container: {
      flex: 1,
      backgroundColor: c.background,
    },
    swipeContainer: {
      flex: 1,
    },
    center: {
      flex: 1,
      justifyContent: 'center',
      alignItems: 'center',
      padding: 20,
      backgroundColor: c.background,
    },
    list: {
      padding: 16,
      paddingBottom: 100,
    },
    listDesktop: {
      padding: 12,
      paddingBottom: 40,
    },
    desktopRow: {
      flex: 1,
      flexDirection: 'row',
    },
    desktopMain: {
      flex: 1,
    },
    categoryGroup: {
      marginBottom: isCompactDesktop ? 10 : 16,
    },
    categoryTitle: {
      fontSize: isCompactDesktop ? 14 : 22,
      fontWeight: '600',
      color: c.primary,
      letterSpacing: 0.5,
      marginBottom: isCompactDesktop ? 4 : 8,
      paddingLeft: 4,
    },
    refreshBanner: {
      flexDirection: 'row',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 8,
      paddingVertical: 8,
      marginHorizontal: 16,
      marginTop: 8,
      ...tintedBanner(c, c.glassPrimary),
    },
    refreshBannerText: {
      fontSize: 12,
      color: c.primary,
      fontWeight: '500',
    },
    orderProgressBanner: {
      flexDirection: 'row',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 8,
      paddingVertical: 10,
      marginHorizontal: 16,
      marginTop: 8,
      ...buttonPrimary(c),
    },
    orderProgressText: {
      fontSize: 13,
      color: '#fff',
      fontWeight: '600',
    },
    errorText: {
      fontSize: 18,
      fontWeight: '600',
      color: c.error,
    },
    hintText: {
      fontSize: 14,
      color: c.textTertiary,
      marginTop: 8,
    },
    emptyText: {
      fontSize: 16,
      color: c.textTertiary,
    },
    errorBanner: {
      padding: 12,
      marginHorizontal: 16,
      marginTop: 8,
      ...tintedBanner(c, c.glassError),
    },
    errorBannerText: {
      color: c.errorText,
      fontSize: 13,
    },
    fab: {
      position: 'absolute',
      bottom: Platform.OS === 'android' ? 24 : 80,
      right: 24,
      paddingHorizontal: 24,
      paddingVertical: 14,
      ...fabStyle(c),
    },
    fabDesktop: {
      position: 'absolute',
      bottom: 16,
      right: 16,
      paddingHorizontal: 16,
      paddingVertical: 8,
      ...fabStyle(c),
    },
    fabText: {
      color: '#fff',
      fontSize: isCompactDesktop ? 13 : 16,
      fontWeight: '700',
    },
  });

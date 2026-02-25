import { useCallback, useMemo, useState } from 'react';
import {
  FlatList,
  Pressable,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { useFocusEffect } from 'expo-router';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useAuthStore } from '../../src-rn/store/authStore';
import { useOrderStore } from '../../src-rn/store/orderStore';
import { useMenuStore } from '../../src-rn/store/menuStore';
import { isSameDay } from '../../src-rn/utils/dateUtils';
import { useFlatStyle, isCompactDesktop } from '../../src-rn/utils/platform';
import { OrderItem } from '../../src-rn/components/OrderItem';
import { LoadingOverlay } from '../../src-rn/components/LoadingOverlay';
import { DesktopContentWrapper } from '../../src-rn/components/DesktopContentWrapper';
import { OrdersPanel } from '../../src-rn/components/OrdersPanel';
import { useTheme } from '../../src-rn/theme/useTheme';
import { useDesktopLayout } from '../../src-rn/hooks/useDesktopLayout';
import { Colors } from '../../src-rn/theme/colors';
import { tintedBanner, buttonPrimary } from '../../src-rn/theme/platformStyles';
import { useDialog } from '../../src-rn/components/DialogProvider';
import { trackSignal } from '../../src-rn/utils/analytics';

type Tab = 'upcoming' | 'past';

export default function OrdersScreen() {
  const { colors } = useTheme();
  const { confirm } = useDialog();
  const insets = useSafeAreaInsets();
  const { isWideLayout, panelWidth } = useDesktopLayout();
  const styles = useMemo(() => createStyles(colors), [colors]);

  const { status: authStatus } = useAuthStore();
  const {
    loading,
    cancellingId,
    error,
    fetchOrders,
    confirmOrders,
    cancelOrder,
    getUpcomingOrders,
    getPastOrders,
    getUnconfirmedCount,
  } = useOrderStore();

  const [activeTab, setActiveTab] = useState<Tab>('upcoming');
  const { items: menuItems, fetchMenus } = useMenuStore();

  useFocusEffect(
    useCallback(() => {
      trackSignal('screen.viewed', { screen: 'orders' });
      if (authStatus === 'authenticated') {
        const { loadCachedOrders } = useOrderStore.getState();
        loadCachedOrders().catch(() => {}).finally(() => {
          fetchOrders();
          fetchMenus();
        });
      }
    }, [authStatus, fetchOrders, fetchMenus])
  );

  const upcoming = getUpcomingOrders();
  const past = getPastOrders();
  const orders = activeTab === 'upcoming' ? upcoming : past;
  const unconfirmedCount = getUnconfirmedCount();

  // Build lookup: "dateStr|category" → subtitle from menu items
  const menuDescriptions = useMemo(() => {
    const map = new Map<string, string>();
    for (const item of menuItems) {
      if (item.subtitle) {
        const key = `${item.day.toDateString()}|${item.title}`;
        map.set(key, item.subtitle);
      }
    }
    return map;
  }, [menuItems]);

  const handleCancel = async (positionId: string, title: string) => {
    const confirmed = await confirm(
      'Bestellung stornieren',
      `"${title}" stornieren?`,
      'Stornieren',
      'Behalten'
    );
    if (confirmed) {
      cancelOrder(positionId);
    }
  };

  if (authStatus !== 'authenticated') {
    return (
      <View style={styles.center}>
        <Text style={styles.hintText}>Anmeldung erforderlich</Text>
      </View>
    );
  }

  if (isWideLayout) {
    return (
      <View style={styles.container}>
        <View style={styles.desktopRow}>
          <OrdersPanel
            width={panelWidth}
            activeTab={activeTab}
            onSelectTab={setActiveTab}
            upcomingCount={upcoming.length}
            pastCount={past.length}
            unconfirmedCount={unconfirmedCount}
            onConfirm={confirmOrders}
            loading={loading}
          />
          <View style={styles.desktopMain}>
            {loading && <LoadingOverlay />}

            {error && (
              <View style={styles.errorBanner}>
                <Text style={styles.errorText}>{error}</Text>
              </View>
            )}

            <FlatList
              data={orders}
              keyExtractor={(item) => item.positionId}
              contentContainerStyle={styles.listDesktop}
              renderItem={({ item }) => (
                <OrderItem
                  order={item}
                  menuDescription={menuDescriptions.get(`${item.date.toDateString()}|${item.title}`)}
                  isCancelling={cancellingId === item.positionId}
                  onCancel={() => handleCancel(item.positionId, item.title)}
                  canCancel={activeTab === 'upcoming' && cancellingId === null}
                />
              )}
              ListEmptyComponent={
                !loading ? (
                  <View style={styles.center}>
                    <Text style={styles.emptyText}>
                      {activeTab === 'upcoming' ? 'Keine kommenden Bestellungen' : 'Keine vergangenen Bestellungen'}
                    </Text>
                  </View>
                ) : null
              }
            />
          </View>
        </View>
      </View>
    );
  }

  return (
    <DesktopContentWrapper maxWidth={700}>
      <View style={[styles.container, { paddingTop: insets.top }]}>
        <View style={styles.tabs}>
          <Pressable
            style={[styles.tab, activeTab === 'upcoming' && styles.tabActive]}
            onPress={() => setActiveTab('upcoming')}
          >
            <Text style={[styles.tabText, activeTab === 'upcoming' && styles.tabTextActive]}>
              Kommende ({upcoming.length})
            </Text>
          </Pressable>
          <Pressable
            style={[styles.tab, activeTab === 'past' && styles.tabActive]}
            onPress={() => setActiveTab('past')}
          >
            <Text style={[styles.tabText, activeTab === 'past' && styles.tabTextActive]}>
              Vergangene ({past.length})
            </Text>
          </Pressable>
        </View>

        {unconfirmedCount > 0 && activeTab === 'upcoming' && (
          <View style={styles.confirmBanner}>
            <Text style={styles.confirmBannerText}>
              {unconfirmedCount} unbestätigte Bestellung{unconfirmedCount > 1 ? 'en' : ''}
            </Text>
            <Pressable
              style={styles.confirmButton}
              onPress={confirmOrders}
              disabled={loading}
            >
              <Text style={styles.confirmButtonText}>Bestätigen</Text>
            </Pressable>
          </View>
        )}

        {loading && <LoadingOverlay />}

        {error && (
          <View style={styles.errorBanner}>
            <Text style={styles.errorText}>{error}</Text>
          </View>
        )}

        <FlatList
          data={orders}
          keyExtractor={(item) => item.positionId}
          contentContainerStyle={styles.list}
          renderItem={({ item }) => (
            <OrderItem
              order={item}
              menuDescription={menuDescriptions.get(`${item.date.toDateString()}|${item.title}`)}
              isCancelling={cancellingId === item.positionId}
              onCancel={() => handleCancel(item.positionId, item.title)}
              canCancel={activeTab === 'upcoming' && cancellingId === null}
            />
          )}
          ListEmptyComponent={
            !loading ? (
              <View style={styles.center}>
                <Text style={styles.emptyText}>
                  {activeTab === 'upcoming' ? 'Keine kommenden Bestellungen' : 'Keine vergangenen Bestellungen'}
                </Text>
              </View>
            ) : null
          }
        />
      </View>
    </DesktopContentWrapper>
  );
}

const createStyles = (c: Colors) =>
  StyleSheet.create({
    container: {
      flex: 1,
      backgroundColor: c.background,
    },
    center: {
      flex: 1,
      justifyContent: 'center',
      alignItems: 'center',
      padding: 20,
      backgroundColor: c.background,
    },
    tabs: {
      flexDirection: 'row',
      backgroundColor: useFlatStyle ? c.surface : c.glassSurface,
      borderBottomWidth: useFlatStyle ? 1 : 0.5,
      borderBottomColor: useFlatStyle ? c.border : c.glassHighlight,
    },
    tab: {
      flex: 1,
      paddingVertical: isCompactDesktop ? 8 : 14,
      alignItems: 'center',
      borderBottomWidth: isCompactDesktop ? 2 : 3,
      borderBottomColor: 'transparent',
    },
    tabActive: {
      borderBottomColor: c.primary,
    },
    tabText: {
      fontSize: isCompactDesktop ? 12 : 14,
      fontWeight: '600',
      color: c.textTertiary,
    },
    tabTextActive: {
      color: c.primary,
    },
    confirmBanner: {
      flexDirection: 'row',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: isCompactDesktop ? 8 : 12,
      marginHorizontal: isCompactDesktop ? 12 : 16,
      marginTop: isCompactDesktop ? 6 : 8,
      ...tintedBanner(c, c.glassWarning),
    },
    confirmBannerText: {
      color: c.warningText,
      fontSize: isCompactDesktop ? 12 : 14,
      fontWeight: '600',
      flex: 1,
    },
    confirmButton: {
      paddingHorizontal: isCompactDesktop ? 14 : 20,
      paddingVertical: isCompactDesktop ? 5 : 8,
      ...buttonPrimary(c),
    },
    confirmButtonText: {
      color: '#fff',
      fontWeight: '700',
      fontSize: isCompactDesktop ? 12 : 14,
    },
    list: {
      padding: isCompactDesktop ? 12 : 16,
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
    hintText: {
      fontSize: 16,
      color: c.textTertiary,
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
    errorText: {
      color: c.errorText,
      fontSize: 13,
    },
  });

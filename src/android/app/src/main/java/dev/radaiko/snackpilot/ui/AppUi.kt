package dev.radaiko.snackpilot.ui

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material3.AlertDialog
import androidx.core.content.ContextCompat
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.AddCircle
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.DarkMode
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.LightMode
import androidx.compose.material.icons.filled.PhoneAndroid
import androidx.compose.material.icons.filled.RemoveCircle
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.outlined.Circle
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import android.app.TimePickerDialog
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.ui.input.pointer.pointerInput
import dev.radaiko.snackpilot.AccentColor
import dev.radaiko.snackpilot.AppViewModel
import dev.radaiko.snackpilot.BillingSource
import dev.radaiko.snackpilot.ThemePreference
import uniffi.snackpilot_core.LogSubsystem
import uniffi.snackpilot_core.MenuCategory
import uniffi.snackpilot_core.MenuItem
import uniffi.snackpilot_core.MenuSnapshot
import uniffi.snackpilot_core.OrderProgress
import uniffi.snackpilot_core.OrderedMenu
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale

/** Always the 4-tab shell — no login wall (settings §3.7). Un-authenticated users see per-tab
 *  empty states and sign in from the Einstellungen tab. */
@Composable
fun RootScreen(
    vm: AppViewModel,
    autoDemo: Boolean = false,
    initialTab: String? = null,
    autoOrder: Boolean = false,
    autoLog: Boolean = false,
    autoReminder: Boolean = false
) {
    LaunchedEffect(Unit) {
        if (autoDemo) {
            vm.selectedTab = when (initialTab) {
                "orders" -> 1
                "billing" -> 2
                "settings" -> 3
                else -> 0
            }
            vm.debugAutoOrder = autoOrder
            vm.debugAutoLog = autoLog
            vm.debugAutoReminder = autoReminder
            vm.loadDemo()
        } else {
            vm.attemptAutoLogin()
        }
    }
    MainScaffold(vm)
}

@Composable
private fun MainScaffold(vm: AppViewModel) {
    Scaffold(
        bottomBar = {
            NavigationBar {
                NavigationBarItem(
                    selected = vm.selectedTab == 0, onClick = { vm.selectedTab = 0 },
                    icon = { Icon(Icons.Filled.Home, null) }, label = { Text("Menüs") }
                )
                NavigationBarItem(
                    selected = vm.selectedTab == 1, onClick = { vm.selectedTab = 1 },
                    icon = { Icon(Icons.AutoMirrored.Filled.List, null) }, label = { Text("Bestellungen") }
                )
                NavigationBarItem(
                    selected = vm.selectedTab == 2, onClick = { vm.selectedTab = 2 },
                    icon = { Icon(Icons.Filled.DateRange, null) }, label = { Text("Abrechnung") }
                )
                NavigationBarItem(
                    selected = vm.selectedTab == 3, onClick = { vm.selectedTab = 3 },
                    icon = { Icon(Icons.Filled.Settings, null) }, label = { Text("Einstellungen") }
                )
            }
        }
    ) { inner ->
        Column(modifier = Modifier.padding(inner)) {
            when (vm.selectedTab) {
                0 -> MenusScreen(vm)
                1 -> OrdersScreen(vm)
                2 -> BillingScreen(vm)
                3 -> SettingsScreen(vm)
            }
        }
    }
}

/** Fixed category order for grouping within a day (menus §5). */
private val CATEGORY_ORDER = listOf(
    MenuCategory.MENU1, MenuCategory.MENU2, MenuCategory.MENU3,
    MenuCategory.SOUP_AND_SALAD, MenuCategory.UNKNOWN
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MenusScreen(vm: AppViewModel) {
    Column {
        val snapshot = vm.snapshot
        if (snapshot == null || snapshot.items.isEmpty()) {
            // Distinguish a genuine "no menus" result from a fetch/parse failure: on error show the
            // friendly message + a retry, otherwise the neutral empty text (menus error-handling).
            val err = snapshot?.error ?: vm.errorText
            if (!vm.gourmetAuthenticated) {
                NotSignedInState()
            } else if (err != null) {
                ErrorRetryState(err, enabled = !vm.busy) { vm.refreshMenusAsync() }
            } else {
                Text(
                    "Keine Menüs für diesen Zeitraum.",
                    modifier = Modifier.padding(16.dp),
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            return
        }
        val dates = snapshot.availableDates
        val selected = vm.selectedDay
        DayNavigator(dates, selected, onSelect = { vm.selectDay(it) })
        val dayItems = snapshot.items.filter { it.day == selected }
        val cutoff = selected?.let { vm.isOrderingCutoff(it) } ?: false
        LazyColumn(
            // Swipe left/right to step between days (menus §4). detectHorizontalDragGestures only
            // consumes horizontal drags, so the list's vertical scroll and row taps still work.
            modifier = Modifier
                .weight(1f)
                .pointerInput(dates, selected) {
                    var dragTotal = 0f
                    detectHorizontalDragGestures(
                        onDragStart = { dragTotal = 0f },
                        onDragEnd = {
                            val threshold = 60.dp.toPx()
                            val idx = dates.indexOf(selected)
                            if (dragTotal <= -threshold) {          // swipe left → next day
                                if (idx < 0) vm.selectDay(dates[0])
                                else if (idx < dates.size - 1) vm.selectDay(dates[idx + 1])
                            } else if (dragTotal >= threshold) {    // swipe right → previous day
                                if (idx > 0) vm.selectDay(dates[idx - 1])
                            }
                        },
                        onHorizontalDrag = { _, delta -> dragTotal += delta }
                    )
                },
            contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp)
        ) {
            CATEGORY_ORDER.forEach { cat ->
                val group = dayItems.filter { it.category == cat }
                if (group.isNotEmpty()) {
                    // Suppress the SUPPE & SALAT heading (menus §5); other groups show their heading.
                    if (cat != MenuCategory.SOUP_AND_SALAD) {
                        item(key = "h-$selected-$cat") { SectionHeader(categoryHeading(cat)) }
                    }
                    itemsIndexed(group, key = { i, _ -> "$selected-$cat-$i" }) { _, item ->
                        val ordered = vm.isOrdered(item)
                        MenuRow(item, orderState(item, snapshot, ordered), cutoff, ordered) { vm.toggle(item) }
                        HorizontalDivider()
                    }
                }
            }
            if (dayItems.isEmpty()) {
                item(key = "empty-day") {
                    Text("Keine Menüs verfügbar", modifier = Modifier.padding(vertical = 16.dp),
                        color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
        if (vm.hasPendingChanges) {
            Column(modifier = Modifier.fillMaxWidth().padding(12.dp)) {
                if (vm.busy) {
                    vm.orderProgress?.let { phase ->
                        Text(progressLabel(phase), style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(bottom = 8.dp))
                    }
                }
                // Surface a failed submission inline (menus error-handling) — otherwise pending
                // changes silently remain and the user believes the order went through.
                vm.errorText?.let { msg ->
                    Text(msg, color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.padding(bottom = 8.dp))
                }
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedButton(onClick = { vm.clearPending() }, enabled = !vm.busy) { Text("Verwerfen") }
                    Button(
                        onClick = { vm.submitOrdersAsync() },
                        enabled = !vm.busy,
                        modifier = Modifier.weight(1f)
                    ) { Text("Bestellen") }
                }
            }
        }
    }
}

/** Live submit-pipeline phase labels (menus §6.6). */
private fun progressLabel(phase: OrderProgress): String = when (phase) {
    OrderProgress.ADDING -> "Wird in den Warenkorb gelegt …"
    OrderProgress.CONFIRMING -> "Wird bestätigt …"
    OrderProgress.CANCELLING -> "Wird storniert …"
    OrderProgress.REFRESHING -> "Wird aktualisiert …"
}

/** Single-day navigator (menus §4.1): prev/next arrows, localized day label + position
 *  indicator, and a "Heute" affordance when today has menus. */
@Composable
private fun DayNavigator(dates: List<String>, selected: String?, onSelect: (String) -> Unit) {
    if (dates.isEmpty()) return
    val index = dates.indexOf(selected)
    val total = dates.size
    val todayKey = LocalDate.now().toString()
    // Nearest menu day (on-or-after today, else last) — the target of the center-tap / "Heute"
    // (menus §4.1), reachable even on a weekend when today itself has no menu.
    val nearest = dates.firstOrNull { it >= todayKey } ?: dates.last()
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        val prevEnabled = index > 0
        NavArrow("‹", prevEnabled) { if (prevEnabled) onSelect(dates[index - 1]) }
        Column(
            modifier = Modifier.weight(1f).clickable { onSelect(nearest) },
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                selected?.let { dayNavLabel(it) } ?: "—",
                style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold
            )
            Text("${index + 1} / $total", style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        // At index -1 (selected day fell out of the list) the forward arrow jumps to the first day.
        val nextEnabled = index < total - 1
        NavArrow("›", nextEnabled) {
            if (index < 0) onSelect(dates[0]) else if (nextEnabled) onSelect(dates[index + 1])
        }
    }
    // "Heute" only when today is actually a menu day and we're not already on it — otherwise the
    // nearest day is a future day and labeling a jump to it "Heute" would mislead (menus §4.1).
    val todayIsAvailable = dates.contains(todayKey)
    if (todayIsAvailable && selected != todayKey) {
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Center) {
            TextButton(onClick = { onSelect(nearest) }) { Text("Heute") }
        }
    }
}

@Composable
private fun NavArrow(glyph: String, enabled: Boolean, onClick: () -> Unit) {
    TextButton(onClick = onClick, enabled = enabled) {
        Text(glyph, style = MaterialTheme.typography.headlineSmall,
            color = if (enabled) MaterialTheme.colorScheme.primary
                    else MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.3f))
    }
}

private enum class OrderState { NONE, ORDERED, PENDING_ORDER, PENDING_CANCEL }

private fun orderState(item: MenuItem, s: MenuSnapshot, isOrdered: Boolean): OrderState {
    val key = "${item.id}|${item.day}"
    return when {
        s.pendingOrders.contains(key) -> OrderState.PENDING_ORDER
        s.pendingCancellations.contains(key) -> OrderState.PENDING_CANCEL
        isOrdered -> OrderState.ORDERED
        else -> OrderState.NONE
    }
}

@Composable
private fun MenuRow(item: MenuItem, state: OrderState, cutoff: Boolean, isOrdered: Boolean, onToggle: () -> Unit) {
    // Tappability + card state (menus §6.1). A menu-marked ordered item stays tappable (to cancel);
    // an un-ordered item needs to be available before cutoff. A cross-ref-only ordered item (in the
    // orders list but not flagged by the menu HTML) is NOT tappable — cancel it in Bestellungen —
    // so a tap can't create a duplicate order.
    val canInteract = item.ordered || (item.available && !cutoff && !isOrdered)
    val badge: String? = when {
        state == OrderState.PENDING_CANCEL -> "Wird storniert"
        isOrdered -> "Bestellt"
        !item.available -> "Ausverkauft"
        cutoff -> "Geschlossen"
        else -> null
    }
    val rowAlpha = when {
        state == OrderState.PENDING_CANCEL -> 0.55f
        !canInteract -> 0.5f
        else -> 1f
    }
    val base = Modifier.fillMaxWidth()
    val rowModifier = (if (canInteract) base.clickable { onToggle() } else base)
        .alpha(rowAlpha)
        .padding(vertical = 8.dp)
    Row(modifier = rowModifier, verticalAlignment = Alignment.Top) {
        val (icon, tint) = when (state) {
            OrderState.ORDERED -> Icons.Filled.CheckCircle to Color(0xFF4CAF50)
            OrderState.PENDING_ORDER -> Icons.Filled.AddCircle to Color(0xFF2196F3)
            OrderState.PENDING_CANCEL -> Icons.Filled.RemoveCircle to Color(0xFFFF9800)
            OrderState.NONE -> Icons.Outlined.Circle to MaterialTheme.colorScheme.onSurfaceVariant
        }
        Icon(icon, null, tint = tint, modifier = Modifier.padding(end = 12.dp, top = 2.dp))
        Column(modifier = Modifier.weight(1f)) {
            val strike = if (state == OrderState.PENDING_CANCEL) TextDecoration.LineThrough else null
            Text(item.title, style = MaterialTheme.typography.bodyLarge, textDecoration = strike)
            if (item.subtitle.isNotEmpty()) {
                Text(item.subtitle, style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant, textDecoration = strike)
            }
            if (item.allergens.isNotEmpty()) {
                Text("Allergene: ${item.allergens.joinToString(", ")}",
                    style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            if (badge != null) {
                Text(badge, style = MaterialTheme.typography.labelSmall, fontWeight = FontWeight.SemiBold,
                    color = when (badge) {
                        "Bestellt" -> Color(0xFF4CAF50)
                        "Wird storniert" -> Color(0xFFFF9800)
                        else -> MaterialTheme.colorScheme.onSurfaceVariant
                    })
            }
        }
        if (item.price.isNotEmpty()) {
            Text(item.price, style = MaterialTheme.typography.titleSmall)
        }
    }
}

@Composable
private fun OrdersScreen(vm: AppViewModel) {
    Column {
        val split = vm.ordersSplit
        if (split == null || (split.upcoming.isEmpty() && split.past.isEmpty())) {
            // On a fetch/parse failure show the friendly message + retry; otherwise the neutral
            // "no orders yet" hint (orders error-handling).
            val err = vm.ordersError
            if (!vm.gourmetAuthenticated) {
                NotSignedInState()
            } else if (err != null) {
                ErrorRetryState(err, enabled = !vm.busy) { vm.reloadOrdersAsync() }
            } else {
                Text("Bestelle ein Menü im Menüs-Tab.", modifier = Modifier.padding(16.dp),
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            return
        }
        val unconfirmed = split.upcoming.count { !it.approved }
        if (unconfirmed > 0) {
            Card(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)
            ) {
                Column(modifier = Modifier.fillMaxWidth().padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            "$unconfirmed unbestätigte ${if (unconfirmed == 1) "Bestellung" else "Bestellungen"}",
                            modifier = Modifier.weight(1f),
                            style = MaterialTheme.typography.bodyMedium
                        )
                        Button(onClick = { vm.confirmOrdersAsync() }, enabled = !vm.busy) {
                            Text("Bestätigen")
                        }
                    }
                    // Surface a failed confirmation inline (orders error-handling) — otherwise the
                    // user thinks the order is confirmed when it isn't.
                    vm.ordersError?.let { msg ->
                        Text(msg, color = MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodySmall,
                            modifier = Modifier.padding(top = 8.dp))
                    }
                }
            }
        }
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp)
        ) {
            if (split.upcoming.isNotEmpty()) {
                item(key = "uh") { SectionHeader("Anstehend") }
                items(split.upcoming, key = { "u-${it.positionId}" }) { o -> OrderRow(o, cancellable = true, vm) }
            }
            if (split.past.isNotEmpty()) {
                item(key = "ph") { SectionHeader("Vergangen") }
                items(split.past, key = { "p-${it.positionId}" }) { o -> OrderRow(o, cancellable = false, vm) }
            }
        }
    }
}

@Composable
private fun OrderRow(order: OrderedMenu, cancellable: Boolean, vm: AppViewModel) {
    Row(modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
        Column(modifier = Modifier.weight(1f)) {
            Text(order.title, style = MaterialTheme.typography.bodyLarge)
            // Show the actual dish (looked up from the menu); fall back to the order's own
            // subtitle (the weekday) when the menu for that day isn't loaded.
            val dish = vm.dishFor(order)
            if (dish != null) {
                Text(dish, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            } else if (order.subtitle.isNotEmpty()) {
                Text(order.subtitle, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Text(billDateLabel(order.dateEpochMs), style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        when {
            order.approved -> Icon(Icons.Filled.CheckCircle, null, tint = Color(0xFF4CAF50))
            cancellable -> TextButton(onClick = { vm.cancelOrder(order.positionId) }) {
                Text("Stornieren", color = MaterialTheme.colorScheme.error)
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun BillingScreen(vm: AppViewModel) {
    Column {
        // Gate on auth, not monthOptions (which is always 3 static offsets). Un-authenticated →
        // "Anmeldung erforderlich" (settings §3.7).
        if (!vm.gourmetAuthenticated && !vm.ventopayAuthenticated) {
            Text("Anmeldung erforderlich", modifier = Modifier.padding(16.dp),
                color = MaterialTheme.colorScheme.onSurfaceVariant)
            return
        }
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            vm.monthOptions.forEach { m ->
                FilterChip(
                    selected = m.offset == vm.selectedOffset,
                    onClick = { vm.selectMonth(m.offset) },
                    label = { Text(m.label) }
                )
            }
        }
        // Source filter (billing §6.1): Alle / Kantine / Automaten. Presentation-only.
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            SourceChip("Alle", vm.billingSourceFilter == BillingSource.ALL) { vm.setBillingSource(BillingSource.ALL) }
            SourceChip("Kantine", vm.billingSourceFilter == BillingSource.GOURMET) { vm.setBillingSource(BillingSource.GOURMET) }
            SourceChip("Automaten", vm.billingSourceFilter == BillingSource.VENTOPAY) { vm.setBillingSource(BillingSource.VENTOPAY) }
        }
        // Loading hint while the selected month's billing is fetched (e.g. switching to an uncached
        // month over the network) so the list doesn't read as "empty" mid-fetch.
        if (vm.billingLoading) {
            Row(
                modifier = Modifier.fillMaxWidth().padding(8.dp),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically
            ) {
                CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp)
                Spacer(modifier = Modifier.width(8.dp))
                Text("Lädt …", color = MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodySmall)
            }
        }
        // Auth-gate each section so a logged-out account's still-cached billing can't reappear.
        val filter = vm.billingSourceFilter
        val g = vm.gourmetMonth?.takeIf { vm.gourmetAuthenticated && it.bills.isNotEmpty() && filter != BillingSource.VENTOPAY }
        val v = vm.ventopayMonth?.takeIf { vm.ventopayAuthenticated && it.transactions.isNotEmpty() && filter != BillingSource.GOURMET }

        // Merge both sources into one list, newest first (billing §6.2). Gourmet is appended before
        // Ventopay so the stable sort keeps that order on equal timestamps.
        val entries = buildList {
            g?.bills?.forEach { bill ->
                add(BillingEntry("g-${bill.billNr}", bill.billDateEpochMs, BillingSource.GOURMET,
                    "", bill.billing,
                    bill.items.map { BillItemLine(it.count, it.description, it.total) }))
            }
            v?.transactions?.forEach { tx ->
                add(BillingEntry("v-${tx.id}", tx.dateEpochMs, BillingSource.VENTOPAY, tx.restaurant, tx.amount))
            }
        }.sortedByDescending { it.dateEpochMs }

        val gourmetTotal = g?.totalBilling ?: 0.0
        val ventopayTotal = v?.total ?: 0.0
        val total = gourmetTotal + ventopayTotal
        val subsidy = g?.totalSubsidy ?: 0.0

        val scope = rememberCoroutineScope()
        var refreshing by remember { mutableStateOf(false) }
        PullToRefreshBox(
            isRefreshing = refreshing,
            onRefresh = {
                scope.launch {
                    refreshing = true
                    vm.reloadBilling()
                    refreshing = false
                }
            }
        ) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp)
        ) {
            // Surface a fetch/parse failure inline (billing error-handling); pull-to-refresh retries.
            vm.billingError?.let { err ->
                item(key = "billing-error") {
                    Text(err, modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodyMedium)
                }
            }
            if (entries.isNotEmpty()) {
                item(key = "summary") { BillingSummary(total, entries.size, subsidy) }
                items(entries, key = { it.id }) { e ->
                    BillingEntryRow(e)
                    HorizontalDivider()
                }
            } else if (vm.billingError == null && !vm.billingLoading) {
                item(key = "nodata") {
                    Text("Keine Abrechnungsdaten für diesen Monat",
                        modifier = Modifier.padding(vertical = 16.dp),
                        color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
        }
    }
}

/** One unified billing entry from either source (billing §6.2). A Kantine bill carries all its
 *  line items (count × description → total), like v1's BillCard; Automaten carries none. */
private data class BillingEntry(
    val id: String,
    val dateEpochMs: Long,
    val source: BillingSource,
    val description: String,
    val amount: Double,
    val items: List<BillItemLine> = emptyList()
)

private data class BillItemLine(val count: Long, val description: String, val total: Double)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SourceChip(label: String, selected: Boolean, onClick: () -> Unit) {
    FilterChip(selected = selected, onClick = onClick, label = { Text(label) })
}

/** Summary bar (billing §6.3 / §8.1): Gesamt, Belege, and Zuschuss (only when > 0). */
@Composable
private fun BillingSummary(total: Double, count: Int, subsidy: Double) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        SummaryCell("Gesamt", euro(total), MaterialTheme.colorScheme.onSurface)
        SummaryCell("Belege", count.toString(), MaterialTheme.colorScheme.onSurface)
        if (subsidy > 0.0) {
            SummaryCell("Zuschuss", euro(subsidy), Color(0xFF4CAF50))
        }
    }
    HorizontalDivider()
}

@Composable
private fun SummaryCell(label: String, value: String, valueColor: Color) {
    Column {
        Text(label, style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold,
            color = valueColor)
    }
}

/** One row in the merged list: date/time on the left, source badge + amount on the right
 *  (billing §8.3). */
@Composable
private fun BillingEntryRow(e: BillingEntry) {
    Column(modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp)) {
        Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.Top) {
            Column(modifier = Modifier.weight(1f)) {
                Text(billDateFull(e.dateEpochMs), style = MaterialTheme.typography.bodyMedium)
                val time = billTime(e.dateEpochMs)
                if (time.isNotEmpty()) {
                    Text(time, style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                if (e.items.isEmpty() && e.description.isNotEmpty()) {
                    Text(e.description, style = MaterialTheme.typography.labelSmall, maxLines = 1,
                        color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            Column(horizontalAlignment = Alignment.End) {
                val (badgeText, badgeColor) = when (e.source) {
                    BillingSource.VENTOPAY -> "AUTOMATEN" to Color(0xFF4CAF50)
                    else -> "KANTINE" to MaterialTheme.colorScheme.primary
                }
                Text(badgeText, style = MaterialTheme.typography.labelSmall, fontWeight = FontWeight.Bold,
                    color = badgeColor)
                Text(euro(e.amount), style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.primary)
            }
        }
        // Kantine bill line items (billing §6): count × description → item total, like v1.
        if (e.items.isNotEmpty()) {
            Column(modifier = Modifier.fillMaxWidth().padding(top = 4.dp, start = 2.dp)) {
                e.items.forEach { item ->
                    Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Text("${item.count}× ", style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text(item.description, style = MaterialTheme.typography.labelSmall, maxLines = 1,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.weight(1f))
                        Text(euro(item.total), style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }
    }
}

@Composable
private fun SectionHeader(text: String) {
    Text(text, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold,
        modifier = Modifier.padding(vertical = 8.dp))
}

/** Currency in the Austrian-German locale (billing §8.3), e.g. "€ 3,00". */
private fun euro(v: Double): String =
    java.text.NumberFormat.getCurrencyInstance(DE_AT).format(v)

private fun billDateLabel(epochMs: Long): String = try {
    java.time.Instant.ofEpochMilli(epochMs).atZone(java.time.ZoneId.systemDefault()).toLocalDate()
        .format(DateTimeFormatter.ofPattern("EEE, d. MMM", DE_AT))
} catch (e: Exception) {
    ""
}

/** Full receipt date (billing §8.3): short weekday, day, month, year in de-AT. */
private fun billDateFull(epochMs: Long): String = try {
    java.time.Instant.ofEpochMilli(epochMs).atZone(java.time.ZoneId.systemDefault()).toLocalDate()
        .format(DateTimeFormatter.ofPattern("EEE, d. MMM yyyy", DE_AT))
} catch (e: Exception) {
    ""
}

private fun billTime(epochMs: Long): String = try {
    java.time.Instant.ofEpochMilli(epochMs).atZone(java.time.ZoneId.systemDefault())
        .format(DateTimeFormatter.ofPattern("HH:mm", DE_AT))
} catch (e: Exception) {
    ""
}

private enum class SettingsRoute { ROOT, KANTINE, AUTOMATEN, APPEARANCE }

@Composable
private fun SettingsScreen(vm: AppViewModel) {
    var route by remember { mutableStateOf(SettingsRoute.ROOT) }
    // System back pops a sub-screen to the settings root (instead of leaving the app).
    BackHandler(enabled = route != SettingsRoute.ROOT) { route = SettingsRoute.ROOT }
    when (route) {
        SettingsRoute.ROOT -> SettingsRootList(vm) { route = it }
        SettingsRoute.KANTINE -> KantineLoginScreen(vm) { route = SettingsRoute.ROOT }
        SettingsRoute.AUTOMATEN -> AutomatenLoginScreen(vm) { route = SettingsRoute.ROOT }
        SettingsRoute.APPEARANCE -> AppearanceScreen(vm) { route = SettingsRoute.ROOT }
    }
}

/** Current color-scheme preference mapped to its German hint label (settings §2.3). */
private fun themePreferenceLabel(pref: ThemePreference): String = when (pref) {
    ThemePreference.SYSTEM -> "System"
    ThemePreference.LIGHT -> "Hell"
    ThemePreference.DARK -> "Dunkel"
}

@Composable
private fun SettingsRootList(vm: AppViewModel, onNavigate: (SettingsRoute) -> Unit) {
    LaunchedEffect(Unit) { vm.refreshLog() }
    vm.locationDialog?.let { dialog ->
        AlertDialog(
            onDismissRequest = { vm.dismissLocationDialog() },
            confirmButton = { TextButton(onClick = { vm.dismissLocationDialog() }) { Text("OK") } },
            title = { Text(dialog.title) },
            text = { Text(dialog.message) }
        )
    }
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp)
    ) {
        item {
            // Section: Konto
            Text("Konto", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            Spacer(Modifier.padding(2.dp))
            SettingsNavRow(
                title = "Kantine-Zugangsdaten",
                hint = if (vm.gourmetAuthenticated)
                    "Angemeldet als ${vm.userInfo?.username ?: ""}" else "Nicht angemeldet",
                onClick = { onNavigate(SettingsRoute.KANTINE) }
            )
            SettingsNavRow(
                title = "Automaten-Zugangsdaten",
                hint = if (vm.ventopayAuthenticated) "Sitzung aktiv" else "Nicht angemeldet",
                onClick = { onNavigate(SettingsRoute.AUTOMATEN) }
            )
            SettingsNavRow(
                title = "Darstellung",
                hint = themePreferenceLabel(vm.themePreference),
                onClick = { onNavigate(SettingsRoute.APPEARANCE) }
            )

            Spacer(Modifier.padding(8.dp))
            Text("Benachrichtigungen", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Tägliche Bestell-Erinnerung", modifier = Modifier.weight(1f))
                Switch(
                    checked = vm.dailyReminderEnabled,
                    onCheckedChange = { vm.setDailyReminder(it, vm.reminderHour, vm.reminderMinute) }
                )
            }
            if (vm.dailyReminderEnabled) {
                val ctx = LocalContext.current
                TextButton(onClick = {
                    TimePickerDialog(
                        ctx,
                        { _, h, m -> vm.setDailyReminder(true, h, m) },
                        vm.reminderHour, vm.reminderMinute, true
                    ).show()
                }) {
                    Text("Uhrzeit: %02d:%02d".format(vm.reminderHour, vm.reminderMinute))
                }
            }

            Spacer(Modifier.padding(8.dp))
            Text("Standort-Benachrichtigungen", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            Text("Erinnerung um 8:45 basierend auf deinem Standort",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            if (vm.companyLocation == null) {
                val ctx = LocalContext.current
                // Background location is a SEPARATE grant on API 29+ (it can't be bundled with the
                // foreground request on 30+), and the geofence needs it to fire while backgrounded.
                val backgroundPermission = rememberLauncherForActivityResult(
                    ActivityResultContracts.RequestPermission()
                ) { granted ->
                    if (granted) vm.captureAndSaveCompanyLocation() else vm.onLocationPermissionDenied()
                }
                val foregroundPermission = rememberLauncherForActivityResult(
                    ActivityResultContracts.RequestMultiplePermissions()
                ) { grants ->
                    val fine = grants[Manifest.permission.ACCESS_FINE_LOCATION] == true ||
                        grants[Manifest.permission.ACCESS_COARSE_LOCATION] == true
                    when {
                        !fine -> vm.onLocationPermissionDenied()
                        ContextCompat.checkSelfPermission(
                            ctx, Manifest.permission.ACCESS_BACKGROUND_LOCATION
                        ) == PackageManager.PERMISSION_GRANTED -> vm.captureAndSaveCompanyLocation()
                        else -> backgroundPermission.launch(Manifest.permission.ACCESS_BACKGROUND_LOCATION)
                    }
                }
                TextButton(
                    enabled = !vm.locationBusy,
                    onClick = {
                        foregroundPermission.launch(
                            arrayOf(
                                Manifest.permission.ACCESS_FINE_LOCATION,
                                Manifest.permission.ACCESS_COARSE_LOCATION
                            )
                        )
                    }
                ) {
                    Text(
                        if (vm.locationBusy) "Standort wird ermittelt..."
                        else "Aktuellen Standort als Firmenstandort setzen"
                    )
                }
            } else {
                Text("Firmenstandort gesetzt",
                    style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                TextButton(onClick = { vm.clearCompanyLocation() }) {
                    Text("Standort entfernen", color = MaterialTheme.colorScheme.error)
                }
            }

            Spacer(Modifier.padding(8.dp))
            Text("Diagnose", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            Text("Protokoll: ${if (vm.logActive) "Aktiv" else "Inaktiv"}",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                TextButton(onClick = { vm.activateLog() }) { Text("Aktivieren (24 h)") }
                TextButton(onClick = { vm.runMenuCheckAsync() }) { Text("Menü-Check") }
                if (vm.logEntries.isNotEmpty()) {
                    TextButton(onClick = { vm.clearLog() }) {
                        Text("Leeren", color = MaterialTheme.colorScheme.error)
                    }
                }
            }
            // Let a sideloaded tester send the dev the log via the native share sheet when something
            // breaks (diagnostics). Only offered when there is something to share.
            if (vm.logEntries.isNotEmpty()) {
                val shareCtx = LocalContext.current
                TextButton(onClick = {
                    val text = vm.logEntries.joinToString("\n") { e ->
                        listOfNotNull(
                            logSubsystemLabel(e.subsystem),
                            e.event,
                            e.detail?.takeIf { it.isNotEmpty() },
                            e.ts
                        ).joinToString(" · ")
                    }
                    val send = Intent(Intent.ACTION_SEND).apply {
                        type = "text/plain"
                        putExtra(Intent.EXTRA_TEXT, text)
                    }
                    shareCtx.startActivity(Intent.createChooser(send, "Protokoll teilen"))
                }) { Text("Protokoll teilen") }
            }
            if (vm.logEntries.isNotEmpty()) {
                Text("Protokoll-Einträge (${vm.logEntries.size})",
                    style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(top = 8.dp))
            }
        }
        items(vm.logEntries.size) { i ->
            val e = vm.logEntries[i]
            Column(modifier = Modifier.padding(vertical = 4.dp)) {
                Text("${logSubsystemLabel(e.subsystem)} · ${e.event}", style = MaterialTheme.typography.bodySmall)
                e.detail?.takeIf { it.isNotEmpty() }?.let {
                    Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Text(e.ts, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        item {
            Spacer(Modifier.padding(16.dp))
            Text("Core-Version: ${vm.coreVersion}",
                style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
private fun SettingsNavRow(title: String, hint: String, onClick: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().clickable { onClick() }.padding(vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.bodyLarge)
            Text(hint, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
    }
    HorizontalDivider()
}

/** Back control shared by the settings sub-screens (settings §1): chevron + "Einstellungen". */
@Composable
private fun SettingsBackControl(onBack: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().clickable { onBack() }.padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(Icons.AutoMirrored.Filled.ArrowBack, null, tint = MaterialTheme.colorScheme.primary)
        Text("Einstellungen", color = MaterialTheme.colorScheme.primary,
            style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(start = 4.dp))
    }
}

/** Kantine (Gourmet) credentials sub-screen (settings §4). Reuses the former login form. */
@Composable
private fun KantineLoginScreen(vm: AppViewModel, onBack: () -> Unit) {
    var username by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    LaunchedEffect(Unit) {
        vm.savedGourmetCreds()?.let { username = it.first; password = it.second }
    }
    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        SettingsBackControl(onBack)
        Text("Kantine-Zugangsdaten", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
        Spacer(Modifier.padding(8.dp))

        if (vm.gourmetAuthenticated) {
            Text("Angemeldet als: ${vm.userInfo?.username ?: ""}", style = MaterialTheme.typography.bodyLarge)
            if (vm.demoMode) Text("Modus: Demo", color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.padding(8.dp))
            OutlinedButton(onClick = { vm.gourmetLogout() }, modifier = Modifier.fillMaxWidth()) {
                Text("Abmelden", color = MaterialTheme.colorScheme.error)
            }
            return
        }

        OutlinedTextField(
            value = username, onValueChange = { username = it },
            label = { Text("Benutzername") }, singleLine = true, modifier = Modifier.fillMaxWidth()
        )
        Spacer(Modifier.padding(4.dp))
        OutlinedTextField(
            value = password, onValueChange = { password = it },
            label = { Text("Passwort") }, singleLine = true,
            visualTransformation = PasswordVisualTransformation(), modifier = Modifier.fillMaxWidth()
        )
        vm.errorText?.let {
            Spacer(Modifier.padding(4.dp))
            Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
        }
        Spacer(Modifier.padding(8.dp))
        Button(
            onClick = { vm.login(username, password) },
            enabled = !vm.busy && username.isNotEmpty() && password.isNotEmpty(),
            modifier = Modifier.fillMaxWidth()
        ) {
            if (vm.busy) CircularProgressIndicator(modifier = Modifier.padding(2.dp)) else Text("Speichern")
        }
        Spacer(Modifier.padding(4.dp))
        OutlinedButton(onClick = { vm.loadDemo() }, modifier = Modifier.fillMaxWidth()) {
            Text("Demo-Menüs anzeigen")
        }
        Text(
            "Offline-Vorschau mit Beispieldaten — keine Verbindung zum Server.",
            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

/** Automaten (Ventopay) credentials sub-screen (settings §5, 04-ui-ux §3.6). */
@Composable
private fun AutomatenLoginScreen(vm: AppViewModel, onBack: () -> Unit) {
    var username by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    LaunchedEffect(Unit) {
        vm.savedVentopayCreds()?.let { username = it.first; password = it.second }
    }
    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        SettingsBackControl(onBack)
        Text("Automaten-Zugangsdaten", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
        Text("Für Automaten und Kassenabrechnungen",
            style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Spacer(Modifier.padding(8.dp))

        if (vm.ventopayAuthenticated) {
            // Mirror the Kantine session block. Ventopay has no user info, so the status line is
            // "Sitzung aktiv" rather than a username (settings §5).
            Text("Sitzung aktiv", style = MaterialTheme.typography.bodyLarge)
            if (vm.demoMode) Text("Modus: Demo", color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.padding(8.dp))
            OutlinedButton(onClick = { vm.ventopayLogout() }, modifier = Modifier.fillMaxWidth()) {
                Text("Abmelden", color = MaterialTheme.colorScheme.error)
            }
            return
        }

        OutlinedTextField(
            value = username, onValueChange = { username = it },
            label = { Text("Benutzername") }, singleLine = true, modifier = Modifier.fillMaxWidth()
        )
        Spacer(Modifier.padding(4.dp))
        OutlinedTextField(
            value = password, onValueChange = { password = it },
            label = { Text("Passwort") }, singleLine = true,
            visualTransformation = PasswordVisualTransformation(), modifier = Modifier.fillMaxWidth()
        )
        vm.ventopayError?.let {
            Spacer(Modifier.padding(4.dp))
            Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
        }
        Spacer(Modifier.padding(8.dp))
        Button(
            onClick = { vm.ventopayLogin(username, password) },
            enabled = !vm.ventopayBusy && username.isNotEmpty() && password.isNotEmpty(),
            modifier = Modifier.fillMaxWidth()
        ) {
            if (vm.ventopayBusy) CircularProgressIndicator(modifier = Modifier.padding(2.dp)) else Text("Speichern")
        }
    }
}

/** Appearance sub-screen ("Darstellung", themes §5). Two cards — the color-scheme preference and
 *  the accent picker. Every tap applies and persists immediately; there is no save button. */
@Composable
private fun AppearanceScreen(vm: AppViewModel, onBack: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)
    ) {
        SettingsBackControl(onBack)
        Text("Darstellung", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
        Spacer(Modifier.padding(8.dp))

        // Card "Design": System / Hell / Dunkel — icon over label, selected highlighted with the accent.
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("Design", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                Spacer(Modifier.padding(4.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    DesignOption(
                        "System", Icons.Filled.PhoneAndroid,
                        vm.themePreference == ThemePreference.SYSTEM, Modifier.weight(1f)
                    ) { vm.setPreference(ThemePreference.SYSTEM) }
                    DesignOption(
                        "Hell", Icons.Filled.LightMode,
                        vm.themePreference == ThemePreference.LIGHT, Modifier.weight(1f)
                    ) { vm.setPreference(ThemePreference.LIGHT) }
                    DesignOption(
                        "Dunkel", Icons.Filled.DarkMode,
                        vm.themePreference == ThemePreference.DARK, Modifier.weight(1f)
                    ) { vm.setPreference(ThemePreference.DARK) }
                }
            }
        }
        Spacer(Modifier.padding(8.dp))

        // Card "Akzentfarbe": 5 circular swatches in fixed order, each in its LIGHT-mode primary.
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("Akzentfarbe", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                Spacer(Modifier.padding(4.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    AccentColor.entries.forEach { accent ->
                        AccentSwatch(accent, vm.accentColor == accent) { vm.setAccent(accent) }
                    }
                }
            }
        }
    }
}

/** One segment of the "Design" card: an icon above a label; selected → accent-tinted surface,
 *  accent border, accent icon/label; otherwise a neutral outline and secondary content. */
@Composable
private fun DesignOption(
    label: String,
    icon: ImageVector,
    selected: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit
) {
    val accent = MaterialTheme.colorScheme.primary
    val content = if (selected) accent else MaterialTheme.colorScheme.onSurfaceVariant
    val shape = RoundedCornerShape(12.dp)
    val border = if (selected) BorderStroke(1.dp, accent)
                 else BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
    Column(
        modifier = modifier
            .clip(shape)
            .border(border, shape)
            .background(if (selected) accent.copy(alpha = 0.12f) else Color.Transparent)
            .clickable { onClick() }
            .padding(vertical = 12.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Icon(icon, null, tint = content)
        Spacer(Modifier.padding(2.dp))
        Text(label, style = MaterialTheme.typography.labelMedium, color = content)
    }
}

/** One accent swatch (themes §5): a 40dp circle filled with the accent's LIGHT-mode primary (even
 *  in dark mode), the German label beneath. Selected → 3dp same-color border + white checkmark. */
@Composable
private fun AccentSwatch(accent: AccentColor, selected: Boolean, onClick: () -> Unit) {
    val fill = Color(accent.lightPrimary)
    val labelColor = if (selected) MaterialTheme.colorScheme.primary
                     else MaterialTheme.colorScheme.onSurfaceVariant
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.clickable { onClick() }
    ) {
        Box(
            modifier = Modifier
                .size(40.dp)
                .clip(CircleShape)
                .background(fill)
                .border(
                    if (selected) BorderStroke(3.dp, fill) else BorderStroke(2.dp, Color.Transparent),
                    CircleShape
                ),
            contentAlignment = Alignment.Center
        ) {
            if (selected) {
                Icon(Icons.Filled.Check, null, tint = Color.White, modifier = Modifier.size(20.dp))
            }
        }
        Spacer(Modifier.padding(2.dp))
        Text(accent.label, style = MaterialTheme.typography.labelSmall, color = labelColor)
    }
}

/** Fetch/parse-failure state (error-handling): the friendly message + a retry button, distinct
 *  from a genuine "no data" empty state so the user knows a load failed and can try again. */
@Composable
private fun ErrorRetryState(message: String, enabled: Boolean, onRetry: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(message, color = MaterialTheme.colorScheme.error,
            style = MaterialTheme.typography.bodyMedium)
        Spacer(Modifier.padding(8.dp))
        Button(onClick = onRetry, enabled = enabled) { Text("Erneut versuchen") }
    }
}

/** Per-tab "not signed in" empty state (settings §3.7). */
@Composable
private fun NotSignedInState() {
    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text("Nicht angemeldet", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
        Text("Melde dich in den Einstellungen an.",
            color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(top = 4.dp))
    }
}

private fun logSubsystemLabel(s: LogSubsystem): String = when (s) {
    LogSubsystem.GEOFENCE -> "geofence"
    LogSubsystem.ORDER_SYNC -> "order-sync"
    LogSubsystem.DAILY_REMINDER -> "daily-reminder"
    LogSubsystem.MENU_CHECK -> "menu-check"
}

@Composable
private fun Placeholder(title: String) {
    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(title, style = MaterialTheme.typography.headlineSmall)
        Text("Kommt in einer späteren Iteration.", color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

private val DE_AT = Locale("de", "AT")

/** `YYYY-MM-DD` day key → short localized weekday + date for the navigator (menus §4.1),
 *  e.g. "Mo., 10. Feb."; falls back to the raw key on a parse error. */
private fun dayNavLabel(key: String): String = try {
    LocalDate.parse(key).format(DateTimeFormatter.ofPattern("EEE, d. MMM", DE_AT))
} catch (e: Exception) {
    key
}

/** Uppercase category heading (menus §5). Matches the enum display strings; UNKNOWN renders
 *  a literal "UNKNOWN" (SUPPE & SALAT's heading is suppressed by the caller). */
private fun categoryHeading(c: MenuCategory): String = when (c) {
    MenuCategory.MENU1 -> "MENÜ I"
    MenuCategory.MENU2 -> "MENÜ II"
    MenuCategory.MENU3 -> "MENÜ III"
    MenuCategory.SOUP_AND_SALAD -> "SUPPE & SALAT"
    MenuCategory.UNKNOWN -> "UNKNOWN"
}

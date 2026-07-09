package dev.radaiko.snackpilot.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import dev.radaiko.snackpilot.AppViewModel
import uniffi.snackpilot_core.MenuCategory
import uniffi.snackpilot_core.MenuItem
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale

/** Login gate → tabs. */
@Composable
fun RootScreen(vm: AppViewModel, autoDemo: Boolean = false) {
    LaunchedEffect(autoDemo) { if (autoDemo) vm.loadDemo() }
    if (vm.userInfo == null) LoginScreen(vm) else MainScaffold(vm)
}

@Composable
private fun LoginScreen(vm: AppViewModel) {
    var username by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }

    Surface(modifier = Modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(24.dp),
            verticalArrangement = Arrangement.Center
        ) {
            Text("SnackPilot", style = MaterialTheme.typography.displaySmall, fontWeight = FontWeight.Bold)
            Text("Kantine-Login", style = MaterialTheme.typography.titleMedium)
            Spacer(Modifier.padding(8.dp))

            OutlinedTextField(
                value = username,
                onValueChange = { username = it },
                label = { Text("Benutzername") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth()
            )
            Spacer(Modifier.padding(4.dp))
            OutlinedTextField(
                value = password,
                onValueChange = { password = it },
                label = { Text("Passwort") },
                singleLine = true,
                visualTransformation = PasswordVisualTransformation(),
                modifier = Modifier.fillMaxWidth()
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
                if (vm.busy) CircularProgressIndicator(modifier = Modifier.padding(2.dp)) else Text("Anmelden")
            }
            Spacer(Modifier.padding(4.dp))
            OutlinedButton(onClick = { vm.loadDemo() }, modifier = Modifier.fillMaxWidth()) {
                Text("Demo-Menüs anzeigen")
            }
            Text(
                "Offline-Vorschau mit Beispieldaten — keine Verbindung zum Server.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            Spacer(Modifier.padding(16.dp))
            Text(
                "Core ${vm.coreVersion}",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun MainScaffold(vm: AppViewModel) {
    var tab by remember { mutableIntStateOf(0) }
    Scaffold(
        bottomBar = {
            NavigationBar {
                NavigationBarItem(
                    selected = tab == 0, onClick = { tab = 0 },
                    icon = { Icon(Icons.Filled.Home, null) }, label = { Text("Menüs") }
                )
                NavigationBarItem(
                    selected = tab == 1, onClick = { tab = 1 },
                    icon = { Icon(Icons.AutoMirrored.Filled.List, null) }, label = { Text("Bestellungen") }
                )
                NavigationBarItem(
                    selected = tab == 2, onClick = { tab = 2 },
                    icon = { Icon(Icons.Filled.DateRange, null) }, label = { Text("Abrechnung") }
                )
                NavigationBarItem(
                    selected = tab == 3, onClick = { tab = 3 },
                    icon = { Icon(Icons.Filled.Settings, null) }, label = { Text("Einstellungen") }
                )
            }
        }
    ) { inner ->
        Column(modifier = Modifier.padding(inner)) {
            when (tab) {
                0 -> MenusScreen(vm)
                1 -> Placeholder("Bestellungen")
                2 -> Placeholder("Abrechnung")
                3 -> SettingsScreen(vm)
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MenusScreen(vm: AppViewModel) {
    Column {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text("Menüs", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
            Spacer(Modifier.weight(1f))
            if (vm.demoMode) {
                Text("DEMO", style = MaterialTheme.typography.labelSmall, fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.tertiary)
            }
        }
        val snapshot = vm.snapshot
        if (snapshot == null || snapshot.items.isEmpty()) {
            Text(
                "Keine Menüs für diesen Zeitraum.",
                modifier = Modifier.padding(16.dp),
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            return
        }
        LazyColumn(modifier = Modifier.fillMaxSize(), contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp)) {
            snapshot.availableDates.forEach { day ->
                item(key = "h-$day") {
                    Text(
                        dayLabel(day),
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.SemiBold,
                        modifier = Modifier.padding(vertical = 8.dp)
                    )
                }
                items(snapshot.items.filter { it.day == day }, key = { "${it.day}-${it.id}-${it.category}" }) { item ->
                    MenuRow(item)
                    HorizontalDivider()
                }
            }
        }
    }
}

@Composable
private fun MenuRow(item: MenuItem) {
    Row(modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp)) {
        Column(modifier = Modifier.weight(1f)) {
            val cat = categoryLabel(item.category)
            if (cat.isNotEmpty()) {
                Text(cat, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Text(item.title, style = MaterialTheme.typography.bodyLarge)
            if (item.subtitle.isNotEmpty()) {
                Text(item.subtitle, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            if (item.allergens.isNotEmpty()) {
                Text("Allergene: ${item.allergens.joinToString(", ")}",
                    style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        if (item.price.isNotEmpty()) {
            Text(item.price, style = MaterialTheme.typography.titleSmall)
        }
    }
}

@Composable
private fun SettingsScreen(vm: AppViewModel) {
    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Text("Einstellungen", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
        Spacer(Modifier.padding(8.dp))
        vm.userInfo?.let {
            Card(modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                Column(Modifier.padding(16.dp)) {
                    Text("Benutzer: ${it.username}")
                    if (vm.demoMode) Text("Modus: Demo")
                }
            }
        }
        Card(modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
            Text("Core-Version: ${vm.coreVersion}", modifier = Modifier.padding(16.dp))
        }
        Spacer(Modifier.padding(8.dp))
        TextButton(onClick = { vm.logout() }) {
            Text("Abmelden", color = MaterialTheme.colorScheme.error)
        }
    }
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

/** `YYYY-MM-DD` (the core's normalized day key) → localized weekday + date, else the raw key. */
private fun dayLabel(key: String): String = try {
    LocalDate.parse(key).format(DateTimeFormatter.ofPattern("EEEE, d. MMMM", Locale.GERMAN))
} catch (e: Exception) {
    key
}

private fun categoryLabel(c: MenuCategory): String = when (c) {
    MenuCategory.MENU1 -> "Menü I"
    MenuCategory.MENU2 -> "Menü II"
    MenuCategory.MENU3 -> "Menü III"
    MenuCategory.SOUP_AND_SALAD -> "Suppe & Salat"
    MenuCategory.UNKNOWN -> ""
}

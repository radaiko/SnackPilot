package dev.radaiko.snackpilot

import android.Manifest
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.radaiko.snackpilot.ui.RootScreen
import uniffi.snackpilot_core.NotificationCommand

class MainActivity : ComponentActivity() {
    private val notifications by lazy { NotificationService(this) }
    private val requestNotifPermission =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            requestNotifPermission.launch(Manifest.permission.POST_NOTIFICATIONS)
        } else {
            notifications // ensure channels exist on older versions
        }

        val autoDemo = BuildConfig.DEBUG && intent.getBooleanExtra("uiTestDemo", false)
        val autoOrder = BuildConfig.DEBUG && intent.getBooleanExtra("uiTestOrder", false)
        val initialTab = intent.getStringExtra("uiTestTab")
        setContent {
            MaterialTheme(colorScheme = darkColorScheme()) {
                RootScreen(viewModel(), autoDemo = autoDemo, initialTab = initialTab, autoOrder = autoOrder)
            }
        }

        // DEBUG headless hook: deliver a sample new-menu notification.
        if (BuildConfig.DEBUG && intent.getBooleanExtra("uiTestNotify", false)) {
            notifications.execute(
                NotificationCommand.FireNow(
                    id = "test-menu",
                    title = "Neues Menü verfügbar",
                    body = "Das Menü für nächste Woche ist online.",
                    channelId = NotificationService.CHANNEL_MENU_UPDATES,
                    screen = null
                )
            )
        }
    }
}

package dev.radaiko.snackpilot

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.radaiko.snackpilot.ui.RootScreen

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        // DEBUG-only headless hook: launch with `-e uiTestDemo true` to jump into offline
        // demo data. Never renders anything that touches the live server.
        val autoDemo = BuildConfig.DEBUG && intent.getBooleanExtra("uiTestDemo", false)
        val autoOrder = BuildConfig.DEBUG && intent.getBooleanExtra("uiTestOrder", false)
        val initialTab = intent.getStringExtra("uiTestTab")
        setContent {
            MaterialTheme(colorScheme = darkColorScheme()) {
                RootScreen(viewModel(), autoDemo = autoDemo, initialTab = initialTab, autoOrder = autoOrder)
            }
        }
    }
}

package dev.radaiko.snackpilot.ui

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

/// SnackPilot brand seed — a warm, appetizing amber (shared with the iOS accent).
private val Brand = Color(0xFFF57722)

private val LightColors = lightColorScheme(primary = Brand, tertiary = Color(0xFF4C662B))
private val DarkColors = darkColorScheme(primary = Brand, tertiary = Color(0xFFB1D18A))

/**
 * Material 3 theme with Material You dynamic color (Android 12+, wallpaper-based), falling back
 * to the SnackPilot brand palette on older devices. Follows the current Material 3 guidelines:
 * dynamic color, full light/dark support.
 */
@Composable
fun SnackPilotTheme(content: @Composable () -> Unit) {
    val dark = isSystemInDarkTheme()
    val dynamic = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
    val context = LocalContext.current
    val colorScheme = when {
        dynamic && dark -> dynamicDarkColorScheme(context)
        dynamic && !dark -> dynamicLightColorScheme(context)
        dark -> DarkColors
        else -> LightColors
    }
    MaterialTheme(colorScheme = colorScheme, content = content)
}

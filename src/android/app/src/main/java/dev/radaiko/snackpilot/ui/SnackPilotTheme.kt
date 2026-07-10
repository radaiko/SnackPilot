package dev.radaiko.snackpilot.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import dev.radaiko.snackpilot.AccentColor
import dev.radaiko.snackpilot.ThemePreference

// Tertiary seed kept from the original brand palette; drives the "DEMO" badge tint only (the accent
// never touches tertiary, matching the v1 palette-composition rule that leaves non-primary keys alone).
private val LightTertiary = Color(0xFF4C662B)
private val DarkTertiary = Color(0xFFB1D18A)

/**
 * Material 3 theme driven by the user's appearance settings (themes §1/§3/§4). The color scheme is
 * resolved from [preference] (SYSTEM → [isSystemInDarkTheme]) and its `primary` is set to the
 * selected [accent]'s primary for the resolved scheme (light hex in light mode, dark hex in dark).
 *
 * Material You dynamic color is intentionally NOT used: the fixed accent palette must win, so the
 * app looks identical regardless of the device wallpaper.
 */
@Composable
fun SnackPilotTheme(
    preference: ThemePreference,
    accent: AccentColor,
    content: @Composable () -> Unit
) {
    val dark = when (preference) {
        ThemePreference.SYSTEM -> isSystemInDarkTheme()
        ThemePreference.LIGHT -> false
        ThemePreference.DARK -> true
    }
    val primary = Color(accent.primary(dark))
    val colorScheme = if (dark) {
        darkColorScheme(primary = primary, tertiary = DarkTertiary)
    } else {
        lightColorScheme(primary = primary, tertiary = LightTertiary)
    }
    MaterialTheme(colorScheme = colorScheme, content = content)
}

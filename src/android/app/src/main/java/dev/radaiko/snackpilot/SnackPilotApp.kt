package dev.radaiko.snackpilot

import android.app.Application
import com.aptabase.Aptabase
import com.aptabase.InitOptions
import uniffi.snackpilot_core.AnalyticsSink

/**
 * Self-hosted Aptabase analytics (vault: "Snackpilot - Aptabase Self-Host").
 *
 * Anonymous by design — no user IDs, device tokens, or IPs are attached (GDPR Recital 26, so no
 * consent banner). The app key is NOT a secret: it ships in the APK and is extractable; the worst
 * case is event spoofing, never data leakage. Fire-and-forget — a send failure must never crash or
 * block the app (analytics.md).
 */
class SnackPilotApp : Application() {
    override fun onCreate() {
        super.onCreate()
        Aptabase.instance.initialize(
            this,
            "A-SH-1116712922",
            // Funnel base URL — the SDK appends /api/v0/events, so this must NOT include /api/v0.
            InitOptions(host = "https://hetzner-server-1.ibex-dory.ts.net"),
        )
        // Per-launch event → drives user/device/session counts (the core only emits on business
        // actions, so without this a passive install would be invisible in the dashboard).
        Aptabase.instance.trackEvent("app.launched")
    }
}

/**
 * Bridges the Rust core's `AnalyticsSink` to Aptabase. The core emits business events
 * (`order.submitted`, `order.cancelled`, `menu.newDetected`); this forwards each one.
 */
class AptabaseSink : AnalyticsSink {
    override fun track(event: String, props: Map<String, String>) {
        Aptabase.instance.trackEvent(event, props)
    }
}

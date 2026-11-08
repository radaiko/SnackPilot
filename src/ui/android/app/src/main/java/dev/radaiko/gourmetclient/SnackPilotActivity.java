package dev.radaiko.gourmetclient;

import android.app.NativeActivity;
import android.content.pm.ApplicationInfo;
import android.content.pm.ActivityInfo;
import android.content.pm.PackageManager;
import android.os.Bundle;
import android.util.Log;

import dev.radaiko.snackpilot.PlatformBridge;

/**
 * NativeActivity host for the Rust gpui-kit app (task 2.1 spike).
 *
 * Follows the gpui-mobile example's GpuiActivity (rev 0b882efd) minus the
 * AndroidX splash screen: NativeActivity's dlopen does not register the
 * library with the app classloader, so we load it explicitly via
 * System.loadLibrary using the android.app.lib_name manifest meta-data.
 *
 * Task 3.4 adds the platform-bridge hooks: registers with {@link PlatformBridge}
 * for permission prompts and forwards permission results back into Rust. The
 * intent extra "snackpilot.debug"="exercise" runs the bridge exercise
 * (credential round trip, fire/schedule/cancel notifications) on debuggable
 * builds only — no UI path ever sets that extra, and exported release builds
 * ignore it.
 */
public class SnackPilotActivity extends NativeActivity {

    private static volatile boolean sNativeLibLoaded = false;

    /** Drives the Rust debug exercise (android.rs); static so the symbol stays
     *  Java_..._SnackPilotActivity_nativeDebugExercise. */
    private static native void nativeDebugExercise();

    /** Geofence transitions into the Rust model (android.rs); static so the
     *  symbol stays Java_..._SnackPilotActivity_nativeOnGeofenceEvent. */
    private static native void nativeOnGeofenceEvent(boolean inside);

    /** Pump handed to {@link dev.radaiko.snackpilot.GeofenceBroadcastReceiver};
     *  set once the native library is loaded, so it is null (event dropped) in
     *  a process spawned only for the broadcast, where the symbols do not
     *  exist yet. */
    public interface GeofencePump {
        void onTransition(boolean inside);
    }

    private static volatile GeofencePump sGeofencePump = null;

    /** The pump, or null while the native library is not loaded in this process. */
    public static GeofencePump getGeofencePump() {
        return sGeofencePump;
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        PlatformBridge.INSTANCE.setActivity(this);
        if (!sNativeLibLoaded) {
            try {
                ActivityInfo ai = getPackageManager().getActivityInfo(
                        getComponentName(), PackageManager.GET_META_DATA);
                String libName = ai.metaData.getString("android.app.lib_name");
                if (libName != null) {
                    System.loadLibrary(libName);
                    // Cache the Kotlin classes Rust calls into; must run here on
                    // the Java main thread right after the library is loaded.
                    PlatformBridge.INSTANCE.nativeCacheClass();
                    sNativeLibLoaded = true;
                }
            } catch (PackageManager.NameNotFoundException e) {
                // Querying our own activity — cannot happen.
            } catch (UnsatisfiedLinkError e) {
                // Already loaded by NativeActivity — fine.
                sNativeLibLoaded = true;
            }
        }
        if (sNativeLibLoaded) {
            sGeofencePump = inside -> nativeOnGeofenceEvent(inside);
        }
        super.onCreate(savedInstanceState);
        String debug = getIntent() == null ? null : getIntent().getStringExtra("snackpilot.debug");
        // The activity is exported, so gate the exercise on FLAG_DEBUGGABLE:
        // a release build must never run it (it writes real credentials),
        // whatever extras an external caller puts on the intent.
        if ("exercise".equals(debug)
                && (getApplicationInfo().flags & ApplicationInfo.FLAG_DEBUGGABLE) != 0) {
            // Runs the whole exercise through the Rust Platform bridge (android.rs).
            Log.i("SnackPilotBridge", "debug exercise requested");
            nativeDebugExercise();
        }
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        PlatformBridge.INSTANCE.onPermissionsResult(requestCode, grantResults);
    }

    @Override
    protected void onDestroy() {
        if (PlatformBridge.INSTANCE.getActivity() == this) {
            PlatformBridge.INSTANCE.setActivity(null);
        }
        super.onDestroy();
    }
}

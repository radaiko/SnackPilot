package dev.radaiko.gourmetclient;

import android.app.NativeActivity;
import android.content.pm.ActivityInfo;
import android.content.pm.PackageManager;
import android.os.Bundle;

/**
 * NativeActivity host for the Rust gpui-kit app (task 2.1 spike).
 *
 * Follows the gpui-mobile example's GpuiActivity (rev 0b882efd) minus the
 * AndroidX splash screen: NativeActivity's dlopen does not register the
 * library with the app classloader, so we load it explicitly via
 * System.loadLibrary using the android.app.lib_name manifest meta-data.
 */
public class SnackPilotActivity extends NativeActivity {

    private static volatile boolean sNativeLibLoaded = false;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        if (!sNativeLibLoaded) {
            try {
                ActivityInfo ai = getPackageManager().getActivityInfo(
                        getComponentName(), PackageManager.GET_META_DATA);
                String libName = ai.metaData.getString("android.app.lib_name");
                if (libName != null) {
                    System.loadLibrary(libName);
                    sNativeLibLoaded = true;
                }
            } catch (PackageManager.NameNotFoundException e) {
                // Querying our own activity — cannot happen.
            } catch (UnsatisfiedLinkError e) {
                // Already loaded by NativeActivity — fine.
                sNativeLibLoaded = true;
            }
        }
        super.onCreate(savedInstanceState);
    }
}

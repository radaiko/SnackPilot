#import <UIKit/UIKit.h>

// GPUI iOS embedding API (gpui-mobile at rev 0b882efdac7f524e0bb0b1d4c886b2aa752f9f20).
void gpui_ios_set_embedded(void);
void *gpui_ios_get_window(void);
void *gpui_ios_view_controller(void *window);
void gpui_ios_layout_view(void *window);
void gpui_ios_request_frame(void *window);
void gpui_ios_did_become_active(void *app);
void gpui_ios_will_resign_active(void *app);
void gpui_ios_did_enter_background(void *app);
void gpui_ios_will_enter_foreground(void *app);

// Defined in the snackpilot-app Rust staticlib (src/ui/app/src/mobile.rs):
// installs the logger/panic hook and starts the GPUI run loop with the
// embedded asset catalog (replaces gpui_ios_register_app + gpui_ios_run_demo).
void gpui_ios_run(void);

// Defined in the snackpilot-app Rust staticlib (src/ui/app/src/lib.rs):
// current spike counter value, for the UI test's accessibility assertion.
unsigned long spike_count(void);

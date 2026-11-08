// Real-keychain round trip for the desktop platform (task 3.6).
//
// Manual proof that DesktopPlatform stores, reads and deletes the real
// credential slots through the OS keychain on a real machine (the Mac).
// Writes the dummy values `test-user` / `test-pass` to the
// `gourmet_username` / `gourmet_password` slots under service `app:no-auth`,
// reads them back, then deletes both and verifies they are gone. Prints only
// booleans — never the secret values.

use snackpilot_app::platform::desktop::DesktopPlatform;
use snackpilot_model::Platform;

fn main() {
    let p = DesktopPlatform::new();

    let save_user = p.save_credential("gourmet_username", "test-user");
    let save_pass = p.save_credential("gourmet_password", "test-pass");
    let match_user = p
        .get_credential("gourmet_username")
        .ok()
        .flatten()
        .as_deref()
        == Some("test-user");
    let match_pass = p
        .get_credential("gourmet_password")
        .ok()
        .flatten()
        .as_deref()
        == Some("test-pass");

    let delete_user = p.clear_credential("gourmet_username");
    let delete_pass = p.clear_credential("gourmet_password");
    let user_gone = p
        .get_credential("gourmet_username")
        .ok()
        .flatten()
        .is_none();
    let pass_gone = p
        .get_credential("gourmet_password")
        .ok()
        .flatten()
        .is_none();

    println!(
        "save_user={} save_pass={} match_user={} match_pass={} \
         delete_user={} delete_pass={} user_gone={} pass_gone={}",
        save_user.is_ok(),
        save_pass.is_ok(),
        match_user,
        match_pass,
        delete_user.is_ok(),
        delete_pass.is_ok(),
        user_gone,
        pass_gone,
    );
    // Error descriptions carry no secret values.
    if let Err(e) = &save_user {
        println!("save_user error: {e:?}");
    }
    if let Err(e) = &save_pass {
        println!("save_pass error: {e:?}");
    }
    assert!(
        save_user.is_ok()
            && save_pass.is_ok()
            && match_user
            && match_pass
            && delete_user.is_ok()
            && delete_pass.is_ok()
            && user_gone
            && pass_gone,
        "keychain round-trip failed"
    );
}

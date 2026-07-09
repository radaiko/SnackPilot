import SwiftUI

/// Top-level gate: the login screen until a Gourmet session (or demo mode) is active,
/// then the tabbed app.
struct RootView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Group {
            if model.userInfo == nil {
                LoginView()
            } else {
                MainTabView()
            }
        }
        .tint(.brand)
    }
}

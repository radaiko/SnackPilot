import SwiftUI

/// Kantine (Gourmet) credentials sub-screen, pushed from Settings → Konto (settings §4 / §3.5).
/// Reuses the former root login form. The demo credentials render canned data offline and are
/// never sent to the live server; any other credentials perform the real scraping login.
struct KantineLoginView: View {
    @EnvironmentObject var model: AppModel
    @State private var username = ""
    @State private var password = ""

    var body: some View {
        Form {
            if model.gourmetAuthenticated {
                Section("Sitzung") {
                    LabeledContent("Angemeldet als", value: model.userInfo?.username ?? username)
                    if model.demoMode {
                        LabeledContent("Modus", value: "Demo")
                    }
                    Button("Abmelden", role: .destructive) {
                        Task { await model.gourmetLogout() }
                    }
                }
            } else {
                Section("Zugangsdaten") {
                    TextField("Benutzername", text: $username)
                        .textContentType(.username)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    SecureField("Passwort", text: $password)
                        .textContentType(.password)
                }

                if let error = model.errorText {
                    Section {
                        Text(error)
                            .font(.footnote)
                            .foregroundStyle(.red)
                    }
                }

                Section {
                    Button {
                        let u = username, p = password
                        Task { await model.login(user: u, pass: p) }
                    } label: {
                        HStack {
                            Spacer()
                            if model.busy {
                                ProgressView()
                            } else {
                                Text("Speichern").bold()
                            }
                            Spacer()
                        }
                    }
                    .primaryAction()
                    .disabled(model.busy || username.isEmpty || password.isEmpty)
                }

                Section {
                    Button("Demo-Menüs anzeigen") { model.loadDemo() }
                } footer: {
                    Text("Offline-Vorschau mit Beispieldaten — keine Verbindung zum Server.")
                }
            }
        }
        .navigationTitle("Kantine-Zugangsdaten")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear {
            if let creds = KeychainStore.savedGourmet() {
                username = creds.username
                password = creds.password
            }
        }
    }
}

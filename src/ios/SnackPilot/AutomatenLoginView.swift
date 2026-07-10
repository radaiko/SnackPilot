import SwiftUI

/// Automaten (Ventopay) credentials sub-screen, pushed from Settings → Konto (settings §5 /
/// 04-ui-ux §3.6). Saves credentials then logs in against the core; on success shows the active
/// session with an Abmelden control. The Ventopay store carries no user info, so no username is
/// shown for the session (settings §5).
struct AutomatenLoginView: View {
    @EnvironmentObject var model: AppModel
    @State private var username = ""
    @State private var password = ""

    /// The screen subtitle (04-ui-ux §3.6), carried as the first section's header.
    private var subtitle: some View {
        Text("Für Automaten und Kassenabrechnungen").textCase(nil)
    }

    var body: some View {
        Form {
            if model.ventopayAuthenticated {
                Section {
                    Text("Automaten-Sitzung aktiv")
                    Button("Abmelden", role: .destructive) {
                        Task { await model.ventopayLogout() }
                    }
                } header: {
                    subtitle
                }
            } else {
                Section {
                    TextField("Benutzername", text: $username)
                        .textContentType(.username)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    SecureField("Passwort", text: $password)
                        .textContentType(.password)
                } header: {
                    subtitle
                }

                if let error = model.ventopayError {
                    Section {
                        Text(error)
                            .font(.footnote)
                            .foregroundStyle(.red)
                    }
                }

                Section {
                    Button {
                        let u = username, p = password
                        Task { await model.ventopayLogin(user: u, pass: p) }
                    } label: {
                        HStack {
                            Spacer()
                            if model.ventopayBusy {
                                ProgressView()
                            } else {
                                Text("Speichern").bold()
                            }
                            Spacer()
                        }
                    }
                    .primaryAction()
                    .disabled(model.ventopayBusy || username.isEmpty || password.isEmpty)
                }
            }
        }
        .navigationTitle("Automaten-Zugangsdaten")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear {
            if let creds = KeychainStore.savedVentopay() {
                username = creds.username
                password = creds.password
            }
        }
    }
}

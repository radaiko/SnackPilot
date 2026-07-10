import SwiftUI

/// Automaten (Ventopay) credentials sub-screen, pushed from Settings → Konto (settings §5 /
/// 04-ui-ux §3.6). Saves credentials then logs in against the core; on success shows the active
/// session with an Abmelden control. The Ventopay store carries no user info, so no username is
/// shown for the session (settings §5).
struct AutomatenLoginView: View {
    @EnvironmentObject var model: AppModel
    @State private var username = ""
    @State private var password = ""

    var body: some View {
        Form {
            if model.ventopayAuthenticated {
                // Mirror the Kantine "Sitzung" section (settings §4/§5). Ventopay carries no user
                // info, so the status line shows "Aktiv" instead of a username.
                Section("Sitzung") {
                    LabeledContent("Status", value: "Aktiv")
                    if model.demoMode {
                        LabeledContent("Modus", value: "Demo")
                    }
                    Button("Abmelden", role: .destructive) {
                        Task { await model.ventopayLogout() }
                    }
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
                    Text("Zugangsdaten")
                } footer: {
                    Text("Für Automaten und Kassenabrechnungen")
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

import SwiftUI

/// Gourmet (Kantine) login. The demo credentials render canned data offline and are never
/// sent to the live server; any other credentials perform the real scraping login.
struct LoginView: View {
    @EnvironmentObject var model: AppModel
    @State private var username = ""
    @State private var password = ""

    var body: some View {
        NavigationStack {
            Form {
                Section("Kantine-Login") {
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
                                Text("Anmelden").bold()
                            }
                            Spacer()
                        }
                    }
                    .disabled(model.busy || username.isEmpty || password.isEmpty)
                }

                Section {
                    Button("Demo-Menüs anzeigen") { model.loadDemo() }
                } footer: {
                    Text("Offline-Vorschau mit Beispieldaten — keine Verbindung zum Server.")
                }
            }
            .navigationTitle("SnackPilot")
            .safeAreaInset(edge: .bottom) {
                Text("Core \(model.coreVersion)")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .padding(.bottom, 8)
            }
        }
    }
}

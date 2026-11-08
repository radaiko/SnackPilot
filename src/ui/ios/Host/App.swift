import UIKit

// SnackPilot spike host (task 2.2), following the gpui-mobile example's
// example/ios/App.swift at rev 0b882efdac7f524e0bb0b1d4c886b2aa752f9f20.
// The bridge supports exactly ONE GPUI view for the app lifetime.

@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?

    func application(_ application: UIApplication,
                     didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        // Register the platform shim table before any Rust code runs (task 3.5).
        PlatformBridge.bootstrap()
        // Evidence run: `xcrun simctl launch ... -DebugPlatformExercise YES`
        // exercises credential, notification and schedule flows end to end;
        // each step logs to stderr tagged "[3.5]".
        if ProcessInfo.processInfo.arguments.contains("-DebugPlatformExercise") {
            snackpilot_ios_run_debug_exercise()
        }
        let window = UIWindow(frame: UIScreen.main.bounds)
        window.rootViewController = SpikeContainerController()
        window.makeKeyAndVisible()
        self.window = window
        return true
    }

    func applicationDidBecomeActive(_ application: UIApplication) {
        gpui_ios_did_become_active(nil)
    }

    func applicationWillResignActive(_ application: UIApplication) {
        gpui_ios_will_resign_active(nil)
    }

    func applicationDidEnterBackground(_ application: UIApplication) {
        gpui_ios_did_enter_background(nil)
    }

    func applicationWillEnterForeground(_ application: UIApplication) {
        gpui_ios_will_enter_foreground(nil)
    }
}

/// UIKit owns this view's frame; GPUI owns only the content rendered inside it.
final class GPUIView: UIView {
    private let gpuiWindow: UnsafeMutableRawPointer
    let contentController: UIViewController

    override init(frame: CGRect) {
        gpui_ios_set_embedded()
        gpui_ios_run()
        guard let window = gpui_ios_get_window(),
              let controller = gpui_ios_view_controller(window) else {
            fatalError("Could not create the GPUI spike view")
        }
        gpuiWindow = window
        contentController = Unmanaged<UIViewController>.fromOpaque(controller).takeUnretainedValue()
        super.init(frame: frame)
        clipsToBounds = true
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) is unsupported") }

    func attach(to parent: UIViewController) {
        parent.addChild(contentController)
        addSubview(contentController.view)
        contentController.didMove(toParent: parent)
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        guard bounds.width > 0, bounds.height > 0,
              contentController.view.frame != bounds else { return }
        // Publish the new geometry and its rendered content together. Otherwise
        // Core Animation can stretch the previous drawable until the next tick.
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        contentController.view.frame = bounds
        gpui_ios_layout_view(gpuiWindow)
        drawFrame()
        CATransaction.commit()
    }

    func drawFrame() { gpui_ios_request_frame(gpuiWindow) }
}

final class SpikeContainerController: UIViewController {
    private var gpui: GPUIView!
    private var displayLink: CADisplayLink?
    private var lastCount: Int = -1

    override func viewDidLoad() {
        super.viewDidLoad()
        overrideUserInterfaceStyle = .light
        view.backgroundColor = .systemBackground

        gpui = GPUIView(frame: .zero)
        gpui.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(gpui)
        gpui.attach(to: self)

        NSLayoutConstraint.activate([
            gpui.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor),
            gpui.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            gpui.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            gpui.bottomAnchor.constraint(equalTo: view.keyboardLayoutGuide.topAnchor),
        ])
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        displayLink = CADisplayLink(target: self, selector: #selector(renderFrame))
        displayLink?.add(to: .main, forMode: .common)
    }

    override func viewWillDisappear(_ animated: Bool) {
        displayLink?.invalidate()
        displayLink = nil
        super.viewWillDisappear(animated)
    }

    /// Drives GPUI frames and mirrors the Rust counter into an accessibility
    /// label, so the XCUITest can assert the tap-to-increment round trip
    /// (GPUI renders to Metal and exposes no accessibility tree of its own).
    @objc private func renderFrame() {
        gpui.drawFrame()
        let count = Int(spike_count())
        if count != lastCount {
            lastCount = count
            let label = "counter \(count)"
            gpui.contentController.view.isAccessibilityElement = true
            gpui.contentController.view.accessibilityLabel = label
        }
    }
}

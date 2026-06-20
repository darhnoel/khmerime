import UIKit

class SceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?
    private var nav: UINavigationController?

    func scene(
        _ scene: UIScene,
        willConnectTo session: UISceneSession,
        options connectionOptions: UIScene.ConnectionOptions
    ) {
        guard let windowScene = scene as? UIWindowScene else { return }
        let window = UIWindow(windowScene: windowScene)
        let nav = UINavigationController(rootViewController: makeRootViewController())
        nav.isNavigationBarHidden = true
        window.rootViewController = nav
        window.makeKeyAndVisible()
        self.window = window
        self.nav = nav
    }

    // Intro-flow state machine (docs/mobile-app-intro-flow.md):
    //   !hasSeenWelcome   -> Brand Landing -> Setup Guide -> Dashboard
    //   !keyboardEnabled  -> Setup Guide -> Dashboard
    //   else              -> Dashboard
    private func makeRootViewController() -> UIViewController {
        if !UserDefaults.standard.bool(forKey: WelcomeViewController.hasSeenWelcomeKey) {
            let welcome = WelcomeViewController()
            welcome.onGetStarted = { [weak self] in self?.advanceFromWelcome() }
            return welcome
        }
        return KeyboardStatus.isEnabled ? DashboardTabController() : makeSetupGuide()
    }

    // Get Started skips the Setup Guide when the keyboard is already enabled, so
    // it doesn't flash past on the way to the Dashboard.
    private func advanceFromWelcome() {
        if KeyboardStatus.isEnabled { showDashboard() } else { pushSetupGuide() }
    }

    private func pushSetupGuide() {
        nav?.pushViewController(makeSetupGuide(), animated: true)
    }

    private func makeSetupGuide() -> SetupGuideViewController {
        let guide = SetupGuideViewController()
        guide.onKeyboardEnabled = { [weak self] in self?.showDashboard() }
        return guide
    }

    private func showDashboard() {
        nav?.setViewControllers([DashboardTabController()], animated: true)
    }
}

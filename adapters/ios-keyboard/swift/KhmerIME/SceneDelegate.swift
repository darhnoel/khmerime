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
    //   !hasSeenWelcome  -> Brand Landing -> Setup Guide
    //   else (Iter 3)    -> Setup Guide if the keyboard isn't enabled, else Dashboard
    // The Dashboard (Iteration 3) doesn't exist yet, so post-enable is stubbed.
    private func makeRootViewController() -> UIViewController {
        let welcome = WelcomeViewController()
        welcome.onGetStarted = { [weak self] in self?.pushSetupGuide() }
        return welcome
    }

    private func pushSetupGuide() {
        let guide = SetupGuideViewController()
        guide.onKeyboardEnabled = { [weak self] in self?.handleKeyboardEnabled() }
        nav?.pushViewController(guide, animated: true)
    }

    private func handleKeyboardEnabled() {
        // TODO(Iteration 3): replace the stack root with DashboardTabController.
    }
}

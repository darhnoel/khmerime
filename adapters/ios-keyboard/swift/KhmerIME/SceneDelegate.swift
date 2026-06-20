import UIKit

class SceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?

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
    }

    // Intro-flow state machine (docs/mobile-app-intro-flow.md):
    //   !hasSeenWelcome  -> Brand Landing
    //   else (Iter 2-3)  -> Setup Guide if the keyboard isn't enabled, else Dashboard
    // Only the Brand Landing exists today; the later branches are stubbed to it.
    private func makeRootViewController() -> UIViewController {
        let welcome = WelcomeViewController()
        welcome.onGetStarted = { [weak self] in
            // TODO(Iteration 2): push SetupGuideViewController here, then route to
            // DashboardTabController once UITextInputMode reports the keyboard enabled.
            self?.advanceFromWelcome()
        }
        return welcome
    }

    private func advanceFromWelcome() {
        // Placeholder until the Setup Guide exists (Iteration 2).
    }
}

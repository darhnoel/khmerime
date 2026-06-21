import UIKit

// DashboardTabController — Iteration 3 of the intro flow (ADR-0011). The home
// shown once the keyboard is enabled: Settings, Tips, and Support tabs, styled
// in Silk Veil (ink bar, amber selection).
final class DashboardTabController: UITabBarController {

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = Brand.ink
        viewControllers = [
            tab(SettingsViewController(), title: "Settings", symbol: "gearshape"),
            tab(TipsViewController(), title: "Tips", symbol: "lightbulb"),
            tab(SupportViewController(), title: "Support", symbol: "heart"),
        ]
        styleTabBar()
    }

    private func tab(_ vc: UIViewController, title: String, symbol: String) -> UIViewController {
        vc.tabBarItem = UITabBarItem(title: title, image: UIImage(systemName: symbol), selectedImage: nil)
        return vc
    }

    private func styleTabBar() {
        let appearance = UITabBarAppearance()
        appearance.configureWithOpaqueBackground()
        appearance.backgroundColor = Brand.ink
        tabBar.standardAppearance = appearance
        tabBar.scrollEdgeAppearance = appearance
        tabBar.tintColor = Brand.amber
        tabBar.unselectedItemTintColor = Brand.ivoryDim
    }
}

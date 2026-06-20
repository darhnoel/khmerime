import UIKit

// SetupGuideViewController — Iteration 2 of the intro flow (ADR-0011,
// docs/mobile-app-intro-flow.md). A 3-step paged guide to enabling the keyboard
// in iOS Settings, with an Open Settings shortcut. While visible it polls
// UITextInputMode; the moment KhmerIME is enabled it persists `keyboard_enabled`
// and calls `onKeyboardEnabled` (routes to the Dashboard in Iteration 3).
final class SetupGuideViewController: UIViewController {

    static let keyboardEnabledKey = "keyboard_enabled"

    /// Called once the keyboard is detected as enabled (flag already persisted).
    var onKeyboardEnabled: (() -> Void)?

    private let steps: [(number: Int, instruction: String)] = [
        (1, "ចូលទៅកាន់ **Settings → General → Keyboard → Keyboards**"),
        (2, "ចុច **Add New Keyboard…** ហើយជ្រើស **KhmerIME**"),
        (3, "ចុច **KhmerIME** បញ្ចូលក្នុងបញ្ជី រួចបើក **Allow Full Access**"),
    ]

    private let pageVC = UIPageViewController(transitionStyle: .scroll, navigationOrientation: .horizontal)
    private let pageControl = UIPageControl()
    private var stepControllers: [UIViewController] = []
    private var pollTimer: Timer?

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = Brand.ink
        stepControllers = steps.map { SetupStepViewController(number: $0.number, instruction: $0.instruction) }
        buildLayout()
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        navigationController?.setNavigationBarHidden(true, animated: animated)
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        // The user leaves to Settings and returns; poll until KhmerIME turns on.
        if KeyboardStatus.isEnabled { finishEnabled(); return }
        pollTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            guard let self, KeyboardStatus.isEnabled else { return }
            self.finishEnabled()
        }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        pollTimer?.invalidate()
        pollTimer = nil
    }

    private func buildLayout() {
        let back = UIButton(type: .system)
        back.setTitle("‹ Back", for: .normal)
        back.setTitleColor(Brand.ivoryDim, for: .normal)
        back.titleLabel?.font = .systemFont(ofSize: 16, weight: .regular)
        back.translatesAutoresizingMaskIntoConstraints = false
        back.addTarget(self, action: #selector(backTapped), for: .touchUpInside)
        view.addSubview(back)

        let title = UILabel()
        title.text = "Enable KhmerIME"
        title.font = .systemFont(ofSize: 22, weight: .bold)
        title.textColor = Brand.ivory
        title.textAlignment = .center
        title.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(title)

        if let first = stepControllers.first {
            pageVC.setViewControllers([first], direction: .forward, animated: false)
        }
        pageVC.dataSource = self
        pageVC.delegate = self
        addChild(pageVC)
        pageVC.view.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(pageVC.view)
        pageVC.didMove(toParent: self)

        pageControl.numberOfPages = stepControllers.count
        pageControl.currentPageIndicatorTintColor = Brand.amber
        pageControl.pageIndicatorTintColor = UIColor.white.withAlphaComponent(0.22)
        pageControl.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(pageControl)

        let openSettings = UIButton(type: .system)
        openSettings.setTitle("Open Settings", for: .normal)
        openSettings.setTitleColor(Brand.ink, for: .normal)
        openSettings.titleLabel?.font = .systemFont(ofSize: 18, weight: .semibold)
        openSettings.backgroundColor = Brand.amber
        openSettings.layer.cornerRadius = 26
        openSettings.layer.cornerCurve = .continuous
        openSettings.translatesAutoresizingMaskIntoConstraints = false
        openSettings.addTarget(self, action: #selector(openSettingsTapped), for: .touchUpInside)
        view.addSubview(openSettings)

        let safe = view.safeAreaLayoutGuide
        NSLayoutConstraint.activate([
            back.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 20),
            back.topAnchor.constraint(equalTo: safe.topAnchor, constant: 8),

            title.topAnchor.constraint(equalTo: back.bottomAnchor, constant: 16),
            title.centerXAnchor.constraint(equalTo: view.centerXAnchor),

            pageVC.view.topAnchor.constraint(equalTo: title.bottomAnchor, constant: 8),
            pageVC.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            pageVC.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            pageVC.view.bottomAnchor.constraint(equalTo: pageControl.topAnchor, constant: -8),

            pageControl.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            pageControl.bottomAnchor.constraint(equalTo: openSettings.topAnchor, constant: -20),

            openSettings.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 32),
            openSettings.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -32),
            openSettings.bottomAnchor.constraint(equalTo: safe.bottomAnchor, constant: -24),
            openSettings.heightAnchor.constraint(equalToConstant: 52),
        ])
    }

    @objc private func openSettingsTapped() {
        guard let url = URL(string: UIApplication.openSettingsURLString) else { return }
        UIApplication.shared.open(url)
    }

    @objc private func backTapped() {
        navigationController?.popViewController(animated: true)
    }

    private func finishEnabled() {
        pollTimer?.invalidate()
        pollTimer = nil
        UserDefaults.standard.set(true, forKey: Self.keyboardEnabledKey)
        onKeyboardEnabled?()
    }

    private func index(of vc: UIViewController) -> Int? { stepControllers.firstIndex(of: vc) }
}

extension SetupGuideViewController: UIPageViewControllerDataSource, UIPageViewControllerDelegate {
    func pageViewController(_ pvc: UIPageViewController, viewControllerBefore vc: UIViewController) -> UIViewController? {
        guard let idx = index(of: vc), idx > 0 else { return nil }
        return stepControllers[idx - 1]
    }

    func pageViewController(_ pvc: UIPageViewController, viewControllerAfter vc: UIViewController) -> UIViewController? {
        guard let idx = index(of: vc), idx < stepControllers.count - 1 else { return nil }
        return stepControllers[idx + 1]
    }

    func pageViewController(
        _ pvc: UIPageViewController,
        didFinishAnimating finished: Bool,
        previousViewControllers: [UIViewController],
        transitionCompleted completed: Bool
    ) {
        if let current = pvc.viewControllers?.first, let idx = index(of: current) {
            pageControl.currentPage = idx
        }
    }
}

// One step of the setup guide: an amber numbered badge over the instruction.
private final class SetupStepViewController: UIViewController {
    private let number: Int
    private let instruction: String

    init(number: Int, instruction: String) {
        self.number = number
        self.instruction = instruction
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    override func viewDidLoad() {
        super.viewDidLoad()
        let badge = makeBadge(number)
        let label = UILabel()
        label.attributedText = boldedInstruction(instruction)
        label.numberOfLines = 0
        label.textAlignment = .center
        label.translatesAutoresizingMaskIntoConstraints = false

        let stack = UIStackView(arrangedSubviews: [badge, label])
        stack.axis = .vertical
        stack.alignment = .center
        stack.spacing = 24
        stack.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(stack)

        NSLayoutConstraint.activate([
            stack.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            stack.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 40),
            stack.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -40),
            badge.widthAnchor.constraint(equalToConstant: 56),
            badge.heightAnchor.constraint(equalToConstant: 56),
        ])
    }

    private func makeBadge(_ n: Int) -> UIView {
        let badge = UIView()
        badge.backgroundColor = Brand.amber
        badge.layer.cornerRadius = 28
        let label = UILabel()
        label.text = "\(n)"
        label.font = .systemFont(ofSize: 24, weight: .bold)
        label.textColor = Brand.ink
        label.translatesAutoresizingMaskIntoConstraints = false
        badge.addSubview(label)
        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: badge.centerXAnchor),
            label.centerYAnchor.constraint(equalTo: badge.centerYAnchor),
        ])
        return badge
    }

    // Renders **bold** segments of the instruction; everything else is regular.
    private func boldedInstruction(_ text: String) -> NSAttributedString {
        let result = NSMutableAttributedString()
        let regular = UIFont.systemFont(ofSize: 19, weight: .regular)
        let bold = UIFont.systemFont(ofSize: 19, weight: .bold)
        for (i, part) in text.components(separatedBy: "**").enumerated() {
            result.append(NSAttributedString(string: part, attributes: [
                .font: i.isMultiple(of: 2) ? regular : bold,
                .foregroundColor: Brand.ivory,
            ]))
        }
        return result
    }
}

import UIKit

// TipsViewController — Iteration 4 content, built alongside the Dashboard. Four
// swipeable tips that teach the core KhmerIME gestures, Silk Veil styled.
final class TipsViewController: UIViewController {

    private struct Tip {
        let symbol: String
        let headline: String
        let body: String
    }

    private let tips: [Tip] = [
        Tip(symbol: "keyboard",
            headline: "Type roman, get Khmer",
            body: "Type somreach and KhmerIME suggests សម្រាក. Tap a candidate or press return to commit."),
        Tip(symbol: "text.append",
            headline: "Long words, split up",
            body: "For long compositions KhmerIME shows segmented previews you can step through with ‹ and ›."),
        Tip(symbol: "character",
            headline: "Names and loanwords",
            body: "Press ✦ for CharPick. Type a roman letter to see matching Khmer characters, then tap one to insert it."),
        Tip(symbol: "globe",
            headline: "English when you need it",
            body: "Press EN to pass keys straight through to the app. Press EN again to return to Khmer."),
    ]

    private let pageVC = UIPageViewController(transitionStyle: .scroll, navigationOrientation: .horizontal)
    private let pageControl = UIPageControl()
    private var pages: [UIViewController] = []

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = Brand.ink
        pages = tips.map { TipPageViewController(symbol: $0.symbol, headline: $0.headline, body: $0.body) }
        buildLayout()
    }

    private func buildLayout() {
        if let first = pages.first {
            pageVC.setViewControllers([first], direction: .forward, animated: false)
        }
        pageVC.dataSource = self
        pageVC.delegate = self
        addChild(pageVC)
        pageVC.view.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(pageVC.view)
        pageVC.didMove(toParent: self)

        pageControl.numberOfPages = pages.count
        pageControl.currentPageIndicatorTintColor = Brand.amber
        pageControl.pageIndicatorTintColor = UIColor.white.withAlphaComponent(0.22)
        pageControl.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(pageControl)

        let safe = view.safeAreaLayoutGuide
        NSLayoutConstraint.activate([
            pageVC.view.topAnchor.constraint(equalTo: safe.topAnchor),
            pageVC.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            pageVC.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            pageVC.view.bottomAnchor.constraint(equalTo: pageControl.topAnchor, constant: -8),

            pageControl.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            pageControl.bottomAnchor.constraint(equalTo: safe.bottomAnchor, constant: -16),
        ])
    }

    private func index(of vc: UIViewController) -> Int? { pages.firstIndex(of: vc) }
}

extension TipsViewController: UIPageViewControllerDataSource, UIPageViewControllerDelegate {
    func pageViewController(_ pvc: UIPageViewController, viewControllerBefore vc: UIViewController) -> UIViewController? {
        guard let idx = index(of: vc), idx > 0 else { return nil }
        return pages[idx - 1]
    }

    func pageViewController(_ pvc: UIPageViewController, viewControllerAfter vc: UIViewController) -> UIViewController? {
        guard let idx = index(of: vc), idx < pages.count - 1 else { return nil }
        return pages[idx + 1]
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

private final class TipPageViewController: UIViewController {
    private let symbol: String
    private let headline: String
    private let bodyText: String

    init(symbol: String, headline: String, body: String) {
        self.symbol = symbol
        self.headline = headline
        self.bodyText = body
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    override func viewDidLoad() {
        super.viewDidLoad()
        let icon = UIImageView(image: UIImage(systemName: symbol))
        icon.tintColor = Brand.amber
        icon.contentMode = .scaleAspectFit
        icon.preferredSymbolConfiguration = UIImage.SymbolConfiguration(pointSize: 52, weight: .regular)
        icon.translatesAutoresizingMaskIntoConstraints = false

        let head = UILabel()
        head.text = headline
        head.font = .systemFont(ofSize: 24, weight: .bold)
        head.textColor = Brand.ivory
        head.textAlignment = .center
        head.numberOfLines = 0

        let body = UILabel()
        body.text = bodyText
        body.font = .systemFont(ofSize: 17, weight: .regular)
        body.textColor = Brand.ivoryDim
        body.textAlignment = .center
        body.numberOfLines = 0

        let stack = UIStackView(arrangedSubviews: [icon, head, body])
        stack.axis = .vertical
        stack.alignment = .center
        stack.spacing = 18
        stack.setCustomSpacing(26, after: icon)
        stack.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(stack)

        NSLayoutConstraint.activate([
            stack.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            stack.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 40),
            stack.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -40),
            icon.heightAnchor.constraint(equalToConstant: 64),
        ])
    }
}

import UIKit

// CharacterTableViewController — Dashboard reference tab. A romanization lookup for
// every Khmer consonant and vowel, transcribed from data/khmer_character_table.md.
// One swipeable page per section (consonants / dependent vowels / independent vowels),
// each an internally-scrolling table, with a UIPageControl — same slider style as Tips.
final class CharacterTableViewController: UIViewController {

    private struct Row { let khmer: String; let roman: String }
    private struct Section { let title: String; let rows: [Row] }

    // Character data is the reference itself, not UI copy — kept inline (not localized).
    private let sections: [Section] = [
        Section(title: String(localized: "chars.section.consonants"), rows: [
            Row(khmer: "ក ខ គ ឃ ង", roman: "k kh g gh ng"),
            Row(khmer: "ច ឆ ជ ឈ ញ", roman: "ch chh j jh nh"),
            Row(khmer: "ដ ឋ ឌ ឍ ណ", roman: "d th dd ddh n"),
            Row(khmer: "ត ថ ទ ធ ន", roman: "t th tt tth n"),
            Row(khmer: "ប ផ ព ភ ម", roman: "b bh p ph m"),
            Row(khmer: "យ រ ល វ", roman: "y r l v, w"),
            Row(khmer: "ស ហ ឡ អ", roman: "s h l a, e, i, o, u"),
        ]),
        Section(title: String(localized: "chars.section.dependentVowels"), rows: [
            Row(khmer: "កា", roman: "a, ar, ea"),
            Row(khmer: "កិ", roman: "e, i"),
            Row(khmer: "កី", roman: "ei, ey"),
            Row(khmer: "កឹ", roman: "e, eu, ue, ir"),
            Row(khmer: "កឺ", roman: "e, eu, er"),
            Row(khmer: "កុ", roman: "o, u"),
            Row(khmer: "កូ", roman: "o, u, ou"),
            Row(khmer: "កួ", roman: "uo, ou"),
            Row(khmer: "កើ", roman: "er"),
            Row(khmer: "កឿ", roman: "oeu"),
            Row(khmer: "កៀ", roman: "ie"),
            Row(khmer: "កេ", roman: "e"),
            Row(khmer: "កែ", roman: "e, ae"),
            Row(khmer: "កៃ", roman: "ai, ay, ei, ey"),
            Row(khmer: "កោ", roman: "ao, ou"),
            Row(khmer: "កៅ", roman: "av, au, ov"),
            Row(khmer: "កុំ", roman: "um, om"),
            Row(khmer: "កំ", roman: "om, um"),
            Row(khmer: "កាំ", roman: "am, an-, ean, oam"),
            Row(khmer: "កះ", roman: "ah, eah, eh"),
            Row(khmer: "កុះ", roman: "oh, uh, os, us"),
            Row(khmer: "កេះ", roman: "eh, ih, es, is"),
            Row(khmer: "កោះ", roman: "oh, os, uoh, uos, ouh, ous"),
        ]),
        Section(title: String(localized: "chars.section.independentVowels"), rows: [
            Row(khmer: "ឥ", roman: "ei, i, e, eu"),
            Row(khmer: "ឦ", roman: "ei, i"),
            Row(khmer: "ឧ", roman: "u"),
            Row(khmer: "ឩ", roman: "u"),
            Row(khmer: "ឪ", roman: "ov"),
            Row(khmer: "ឫ", roman: "ru, reu"),
            Row(khmer: "ឬ", roman: "ru, reu"),
            Row(khmer: "ឭ", roman: "leu"),
            Row(khmer: "ឮ", roman: "leu"),
            Row(khmer: "ឯ", roman: "ae, e"),
            Row(khmer: "ឰ", roman: "ai"),
            Row(khmer: "ឱ", roman: "ao"),
            Row(khmer: "ឲ", roman: "ao"),
            Row(khmer: "ឳ", roman: "av, aov"),
        ]),
    ]

    private let pageVC = UIPageViewController(transitionStyle: .scroll, navigationOrientation: .horizontal)
    private let pageControl = UIPageControl()
    private var pages: [UIViewController] = []

    override func viewDidLoad() {
        super.viewDidLoad()
        title = String(localized: "chars.tab")
        view.backgroundColor = Brand.ink
        pages = sections.map { section in
            SectionPageViewController(
                title: section.title,
                rows: section.rows.map { ($0.khmer, $0.roman) })
        }
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

extension CharacterTableViewController: UIPageViewControllerDataSource, UIPageViewControllerDelegate {
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

// One section's rows as an internally-scrolling table with an amber title header.
private final class SectionPageViewController: UIViewController, UITableViewDataSource, UITableViewDelegate {
    private let sectionTitle: String
    private let rows: [(khmer: String, roman: String)]
    private let tableView = UITableView(frame: .zero, style: .plain)

    init(title: String, rows: [(khmer: String, roman: String)]) {
        self.sectionTitle = title
        self.rows = rows
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = Brand.ink
        tableView.backgroundColor = Brand.ink
        tableView.separatorColor = UIColor.white.withAlphaComponent(0.08)
        tableView.showsVerticalScrollIndicator = false
        tableView.rowHeight = UITableView.automaticDimension
        tableView.estimatedRowHeight = 52
        tableView.register(CharacterCell.self, forCellReuseIdentifier: "cell")
        tableView.dataSource = self
        tableView.delegate = self
        tableView.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(tableView)
        NSLayoutConstraint.activate([
            tableView.topAnchor.constraint(equalTo: view.topAnchor),
            tableView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            tableView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            tableView.bottomAnchor.constraint(equalTo: view.bottomAnchor),
        ])
    }

    func numberOfSections(in tableView: UITableView) -> Int { 1 }

    func tableView(_ tableView: UITableView, numberOfRowsInSection section: Int) -> Int { rows.count }

    func tableView(_ tableView: UITableView, titleForHeaderInSection section: Int) -> String? { sectionTitle }

    func tableView(_ tableView: UITableView, cellForRowAt indexPath: IndexPath) -> UITableViewCell {
        let cell = tableView.dequeueReusableCell(withIdentifier: "cell", for: indexPath) as! CharacterCell
        let row = rows[indexPath.row]
        cell.configure(khmer: row.khmer, roman: row.roman)
        return cell
    }

    func tableView(_ tableView: UITableView, willDisplayHeaderView view: UIView, forSection section: Int) {
        (view as? UITableViewHeaderFooterView)?.textLabel?.textColor = Brand.amber
    }
}

// One reference row: Khmer character(s) on the left, roman input on the right.
private final class CharacterCell: UITableViewCell {
    private let khmerLabel = UILabel()
    private let romanLabel = UILabel()

    override init(style: UITableViewCell.CellStyle, reuseIdentifier: String?) {
        super.init(style: style, reuseIdentifier: reuseIdentifier)
        backgroundColor = UIColor.white.withAlphaComponent(0.05)
        selectionStyle = .none

        khmerLabel.font = .systemFont(ofSize: 22, weight: .medium)
        khmerLabel.textColor = Brand.ivory
        khmerLabel.numberOfLines = 0
        khmerLabel.setContentHuggingPriority(.defaultLow, for: .horizontal)

        romanLabel.font = .monospacedSystemFont(ofSize: 15, weight: .regular)
        romanLabel.textColor = Brand.ivoryDim
        romanLabel.numberOfLines = 0
        romanLabel.textAlignment = .right
        romanLabel.setContentHuggingPriority(.defaultHigh, for: .horizontal)
        romanLabel.setContentCompressionResistancePriority(.required, for: .horizontal)

        let stack = UIStackView(arrangedSubviews: [khmerLabel, romanLabel])
        stack.axis = .horizontal
        stack.alignment = .center
        stack.spacing = 16
        stack.translatesAutoresizingMaskIntoConstraints = false
        contentView.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: contentView.topAnchor, constant: 10),
            stack.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -10),
            stack.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 16),
            stack.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -16),
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    func configure(khmer: String, roman: String) {
        khmerLabel.text = khmer
        romanLabel.text = roman
    }
}

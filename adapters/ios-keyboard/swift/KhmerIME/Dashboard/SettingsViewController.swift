import UIKit

// SettingsViewController — Iteration 3 (ADR-0011). The Standard/Smart toggle persists a shared
// preference the keyboard extension reads (see SmartModePreference); it's inert without a registered
// provider, so in the OSS build it has no visible effect. Custom Words and Clear Learned History
// still show "Coming soon" (iOS has no learned-history persistence yet).
final class SettingsViewController: UITableViewController {

    private struct Item {
        let title: String
        let detail: String
        let isMuted: Bool
        // A switch row (Standard/Smart toggle) instead of a detail-text row.
        let isSwitch: Bool

        init(title: String, detail: String, isMuted: Bool, isSwitch: Bool = false) {
            self.title = title
            self.detail = detail
            self.isMuted = isMuted
            self.isSwitch = isSwitch
        }
    }
    private struct Section {
        let header: String
        let items: [Item]
    }

    private let smartMode = SmartModePreference()

    private lazy var sections: [Section] = [
        Section(header: String(localized: "settings.section.prediction"),
                items: [
                    Item(title: String(localized: "settings.item.smartMode"), detail: "", isMuted: false, isSwitch: true),
                    Item(title: String(localized: "settings.item.nextWord"), detail: String(localized: "settings.comingSoon"), isMuted: true),
                ]),
        Section(header: String(localized: "settings.section.words"),
                items: [Item(title: String(localized: "settings.item.customWords"), detail: String(localized: "settings.comingSoon"), isMuted: true)]),
        Section(header: String(localized: "settings.section.data"),
                items: [Item(title: String(localized: "settings.item.clearHistory"), detail: String(localized: "settings.comingSoon"), isMuted: true)]),
        Section(header: String(localized: "settings.section.about"),
                items: [Item(title: String(localized: "settings.item.version"), detail: SettingsViewController.versionString, isMuted: false)]),
    ]

    init() { super.init(style: .insetGrouped) }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    override func viewDidLoad() {
        super.viewDidLoad()
        title = String(localized: "settings.title")
        view.backgroundColor = Brand.ink
        tableView.backgroundColor = Brand.ink
        tableView.separatorColor = UIColor.white.withAlphaComponent(0.08)
    }

    override func numberOfSections(in tableView: UITableView) -> Int { sections.count }

    override func tableView(_ tableView: UITableView, numberOfRowsInSection section: Int) -> Int {
        sections[section].items.count
    }

    override func tableView(_ tableView: UITableView, titleForHeaderInSection section: Int) -> String? {
        sections[section].header
    }

    override func tableView(_ tableView: UITableView, cellForRowAt indexPath: IndexPath) -> UITableViewCell {
        let item = sections[indexPath.section].items[indexPath.row]
        let cell = UITableViewCell(style: .value1, reuseIdentifier: nil)
        cell.backgroundColor = UIColor.white.withAlphaComponent(0.05)
        cell.textLabel?.text = item.title
        cell.textLabel?.textColor = item.isMuted ? Brand.ivoryDim : Brand.ivory
        cell.detailTextLabel?.text = item.detail
        cell.detailTextLabel?.textColor = Brand.ivoryDim
        cell.selectionStyle = .none
        if item.isSwitch {
            let toggle = UISwitch()
            toggle.isOn = smartMode.isEnabled
            toggle.addTarget(self, action: #selector(smartModeToggled(_:)), for: .valueChanged)
            cell.accessoryView = toggle
        }
        return cell
    }

    @objc private func smartModeToggled(_ sender: UISwitch) {
        smartMode.setEnabled(sender.isOn)
    }

    private static var versionString: String {
        let info = Bundle.main.infoDictionary
        let version = info?["CFBundleShortVersionString"] as? String ?? "1.0"
        let build = info?["CFBundleVersion"] as? String ?? "1"
        return "\(version) (\(build))"
    }
}

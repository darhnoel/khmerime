import UIKit

// SettingsViewController — Iteration 3 (ADR-0011). Only About is functional
// today: iOS has no learned-history persistence or next-word prediction yet,
// and the host app doesn't link the engine, so the toggle, Custom Words, and
// Clear Learned History show as "Coming soon" until that bridge exists.
final class SettingsViewController: UITableViewController {

    private struct Item {
        let title: String
        let detail: String
        let isMuted: Bool
    }
    private struct Section {
        let header: String
        let items: [Item]
    }

    private let sections: [Section] = [
        Section(header: "Prediction", items: [Item(title: "Next Word Prediction", detail: "Coming soon", isMuted: true)]),
        Section(header: "Words", items: [Item(title: "Custom Words", detail: "Coming soon", isMuted: true)]),
        Section(header: "Data", items: [Item(title: "Clear Learned History", detail: "Coming soon", isMuted: true)]),
        Section(header: "About", items: [Item(title: "Version", detail: SettingsViewController.versionString, isMuted: false)]),
    ]

    init() { super.init(style: .insetGrouped) }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    override func viewDidLoad() {
        super.viewDidLoad()
        title = "Settings"
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
        return cell
    }

    private static var versionString: String {
        let info = Bundle.main.infoDictionary
        let version = info?["CFBundleShortVersionString"] as? String ?? "1.0"
        let build = info?["CFBundleVersion"] as? String ?? "1"
        return "\(version) (\(build))"
    }
}

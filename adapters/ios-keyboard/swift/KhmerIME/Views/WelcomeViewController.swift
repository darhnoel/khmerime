import UIKit

// WelcomeViewController — Iteration 1 "Brand Landing" of the mobile app intro
// flow (ADR-0011, docs/mobile-app-intro-flow.md). A one-time Silk Veil welcome
// shown until the user taps Get Started, which persists `has_seen_welcome`.
// Onward navigation to the Setup Guide arrives in Iteration 2 via `onGetStarted`.
final class WelcomeViewController: UIViewController {

    static let hasSeenWelcomeKey = "has_seen_welcome"

    /// Invoked after Get Started persists the flag. The owner (SceneDelegate)
    /// decides where to go next.
    var onGetStarted: (() -> Void)?

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = Brand.ink
        buildLayout()
    }

    private func buildLayout() {
        let logo = makePlaceholderLogo()
        let wordmark = makeLabel("KhmerIME", size: 34, weight: .heavy, color: Brand.ivory)
        let romanKhmer = makeLabel("roman → ខ្មែរ", size: 20, weight: .semibold, color: Brand.amber)
        let tagline = makeLabel("Type Khmer using the Latin alphabet", size: 16, weight: .regular, color: Brand.ivoryDim)
        tagline.numberOfLines = 0

        let getStarted = makePrimaryButton("Get Started")
        getStarted.addTarget(self, action: #selector(getStartedTapped), for: .touchUpInside)

        let alreadyEnabled = makeLabel("Already enabled? Open keyboard →", size: 14, weight: .regular, color: Brand.ivoryDim)

        let stack = UIStackView(arrangedSubviews: [logo, wordmark, romanKhmer, tagline, getStarted, alreadyEnabled])
        stack.axis = .vertical
        stack.alignment = .center
        stack.spacing = 14
        stack.translatesAutoresizingMaskIntoConstraints = false
        stack.setCustomSpacing(28, after: logo)
        stack.setCustomSpacing(30, after: tagline)
        view.addSubview(stack)

        NSLayoutConstraint.activate([
            stack.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            stack.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            stack.leadingAnchor.constraint(greaterThanOrEqualTo: view.leadingAnchor, constant: 32),
            stack.trailingAnchor.constraint(lessThanOrEqualTo: view.trailingAnchor, constant: -32),
            logo.widthAnchor.constraint(equalToConstant: 132),
            logo.heightAnchor.constraint(equalToConstant: 132),
            getStarted.widthAnchor.constraint(equalToConstant: 220),
            getStarted.heightAnchor.constraint(equalToConstant: 52),
        ])
    }

    @objc private func getStartedTapped() {
        UserDefaults.standard.set(true, forKey: Self.hasSeenWelcomeKey)
        onGetStarted?()
    }

    // MARK: - Builders

    // PLACEHOLDER logo — a glass-card "ខ" approximating the CSS logo in
    // site/download/styles.css. Replace with the exported khmerime-logo PNG
    // (Iteration 1 pre-work) once the asset is available.
    private func makePlaceholderLogo() -> UIView {
        let card = UIView()
        card.backgroundColor = UIColor.white.withAlphaComponent(0.06)
        card.layer.cornerRadius = 30
        card.layer.cornerCurve = .continuous
        card.layer.borderWidth = 1
        card.layer.borderColor = UIColor.white.withAlphaComponent(0.18).cgColor

        let glyph = makeLabel("ខ", size: 66, weight: .semibold, color: Brand.ivory)
        glyph.translatesAutoresizingMaskIntoConstraints = false
        card.addSubview(glyph)
        NSLayoutConstraint.activate([
            glyph.centerXAnchor.constraint(equalTo: card.centerXAnchor),
            glyph.centerYAnchor.constraint(equalTo: card.centerYAnchor),
        ])
        return card
    }

    private func makeLabel(_ text: String, size: CGFloat, weight: UIFont.Weight, color: UIColor) -> UILabel {
        let label = UILabel()
        label.text = text
        label.font = .systemFont(ofSize: size, weight: weight)
        label.textColor = color
        label.textAlignment = .center
        return label
    }

    private func makePrimaryButton(_ title: String) -> UIButton {
        let button = UIButton(type: .system)
        button.setTitle(title, for: .normal)
        button.setTitleColor(Brand.ink, for: .normal)
        button.titleLabel?.font = .systemFont(ofSize: 18, weight: .semibold)
        button.backgroundColor = Brand.amber
        button.layer.cornerRadius = 26
        button.layer.cornerCurve = .continuous
        return button
    }
}

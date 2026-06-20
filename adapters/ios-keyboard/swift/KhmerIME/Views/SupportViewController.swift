import UIKit

// SupportViewController — Iteration 5 content, built alongside the Dashboard. A
// single glass card inviting support, with the ABA QR. The QR is a placeholder
// box until site/download/assets/my-aba-cropped.png is bundled as an asset.
final class SupportViewController: UIViewController {

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = Brand.ink
        buildLayout()
    }

    private func buildLayout() {
        let card = UIView()
        card.backgroundColor = UIColor.white.withAlphaComponent(0.06)
        card.layer.cornerRadius = 24
        card.layer.cornerCurve = .continuous
        card.layer.borderWidth = 1
        card.layer.borderColor = UIColor.white.withAlphaComponent(0.18).cgColor
        card.translatesAutoresizingMaskIntoConstraints = false

        let heading = makeLabel("Support KhmerIME", size: 22, weight: .bold, color: Brand.ivory)
        let body = makeLabel(
            "KhmerIME is an open-source project. Your support keeps development, testing, packaging, and future improvements going.",
            size: 16, weight: .regular, color: Brand.ivoryDim
        )

        let qr = makeQRPlaceholder()
        let caption = makeLabel("ABA Mobile", size: 14, weight: .semibold, color: Brand.amber)

        let stack = UIStackView(arrangedSubviews: [heading, body, qr, caption])
        stack.axis = .vertical
        stack.alignment = .center
        stack.spacing = 16
        stack.setCustomSpacing(22, after: body)
        stack.translatesAutoresizingMaskIntoConstraints = false
        card.addSubview(stack)
        view.addSubview(card)

        let safe = view.safeAreaLayoutGuide
        NSLayoutConstraint.activate([
            card.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            card.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 24),
            card.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -24),
            card.topAnchor.constraint(greaterThanOrEqualTo: safe.topAnchor, constant: 24),

            stack.topAnchor.constraint(equalTo: card.topAnchor, constant: 28),
            stack.bottomAnchor.constraint(equalTo: card.bottomAnchor, constant: -28),
            stack.leadingAnchor.constraint(equalTo: card.leadingAnchor, constant: 24),
            stack.trailingAnchor.constraint(equalTo: card.trailingAnchor, constant: -24),

            qr.widthAnchor.constraint(equalToConstant: 160),
            qr.heightAnchor.constraint(equalToConstant: 160),
        ])
    }

    // PLACEHOLDER — replace with a UIImageView of the bundled ABA QR asset.
    private func makeQRPlaceholder() -> UIView {
        let box = UIView()
        box.backgroundColor = UIColor.white.withAlphaComponent(0.04)
        box.layer.cornerRadius = 12
        box.layer.borderWidth = 1
        box.layer.borderColor = Brand.amber.withAlphaComponent(0.5).cgColor
        let label = makeLabel("ABA QR", size: 14, weight: .regular, color: Brand.ivoryDim)
        label.translatesAutoresizingMaskIntoConstraints = false
        box.addSubview(label)
        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: box.centerXAnchor),
            label.centerYAnchor.constraint(equalTo: box.centerYAnchor),
        ])
        return box
    }

    private func makeLabel(_ text: String, size: CGFloat, weight: UIFont.Weight, color: UIColor) -> UILabel {
        let label = UILabel()
        label.text = text
        label.font = .systemFont(ofSize: size, weight: weight)
        label.textColor = color
        label.textAlignment = .center
        label.numberOfLines = 0
        return label
    }
}

import UIKit

// SupportViewController — Iteration 5 content, built alongside the Dashboard. A
// single glass card inviting support, with the ABA QR bundled through the iOS
// asset-generation step.
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

        let qr = makeQR()
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

    private func makeQR() -> UIView {
        let imageView = UIImageView(image: UIImage(named: "ABAQR"))
        imageView.contentMode = .scaleAspectFit
        imageView.layer.cornerRadius = 12
        imageView.clipsToBounds = true
        return imageView
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

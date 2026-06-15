import UIKit

final class GlassKeyView: UIView {

    let label: String
    var onTap: (() -> Void)?
    var isActive: Bool = false { didSet { setNeedsDisplay() } }

    private lazy var pressAnimator = GlassKeyPressAnimator(onUpdate: { [weak self] squish in
        let scale = 1 - squish * 0.08
        self?.transform = CGAffineTransform(scaleX: scale, y: scale)
    })

    init(label: String, isActive: Bool = false) {
        self.label = label
        self.isActive = isActive
        super.init(frame: .zero)
        isOpaque = false
    }

    required init?(coder: NSCoder) { fatalError("use init(label:)") }

    override func draw(_ rect: CGRect) {
        let isDark = traitCollection.userInterfaceStyle == .dark
        let bg = isActive
            ? GlassColorSpec.toggleActiveBackground(isDark: isDark)
            : GlassColorSpec.backgroundColor(isDark: isDark)
        let border = GlassColorSpec.borderColor(isDark: isDark)
        let textColor: UIColor = isActive
            ? GlassColorSpec.toggleActiveTextColor()
            : (isDark ? .white : .black)

        let cornerRadius = rect.height * 0.22
        let path = UIBezierPath(roundedRect: rect, cornerRadius: cornerRadius)
        bg.setFill()
        path.fill()
        border.setStroke()
        path.lineWidth = 1
        path.stroke()

        let fontSize: CGFloat = label.count > 1 ? 13 : 17
        let attrs: [NSAttributedString.Key: Any] = [
            .font: UIFont.systemFont(ofSize: fontSize, weight: .medium),
            .foregroundColor: textColor,
        ]
        let size = (label as NSString).size(withAttributes: attrs)
        let origin = CGPoint(x: (rect.width - size.width) / 2, y: (rect.height - size.height) / 2)
        (label as NSString).draw(at: origin, withAttributes: attrs)
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesBegan(touches, with: event)
        pressAnimator.press()
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesEnded(touches, with: event)
        pressAnimator.release()
        if let touch = touches.first, bounds.contains(touch.location(in: self)) {
            onTap?()
        }
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesCancelled(touches, with: event)
        pressAnimator.release()
    }
}

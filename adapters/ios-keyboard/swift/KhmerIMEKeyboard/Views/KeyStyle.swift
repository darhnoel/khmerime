import UIKit

class GlassKeyButton: UIButton {
    // Max depth of the press-squish: the button shrinks to (1 - pressScaleDepth)
    // of its size at full press. Exposed so point(inside:) can compensate for the
    // hit-test dead ring the scale would otherwise create (see point(inside:with:)).
    static let pressScaleDepth: CGFloat = 0.08

    var isGlassActive = false {
        didSet { updateGlassAppearance() }
    }

    // Fires on every touchesBegan, bypassing UIControl's single-touch tracking.
    // Use this instead of addTarget(for: .touchDown) for keys that must register
    // every rapid tap even when two touches physically overlap.
    var onPress: (() -> Void)?
    var previewLabel: String?
    var onPreviewChanged: ((GlassKeyButton, String?) -> Void)?

    private var isPressed = false
    private var isPreviewVisible = false
    private var pressAnimator: GlassKeyPressAnimator?
    private var inputFeedbackForTesting: (() -> Void)?

    private lazy var blurView: UIVisualEffectView = {
        let v = UIVisualEffectView(effect: UIBlurEffect(style: .systemUltraThinMaterial))
        v.isUserInteractionEnabled = false
        insertSubview(v, at: 0)
        return v
    }()

    override init(frame: CGRect) {
        super.init(frame: frame)
        isMultipleTouchEnabled = true
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        isMultipleTouchEnabled = true
    }

    // Inject a synchronous runner for testing before any touch event fires.
    func configureForTesting(runner: @escaping AnimatorRunner) {
        pressAnimator = GlassKeyPressAnimator(
            onUpdate: { [weak self] squish in self?.applySquish(squish) },
            runner: runner
        )
    }

    func configureInputFeedbackForTesting(_ performFeedback: @escaping () -> Void) {
        inputFeedbackForTesting = performFeedback
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        performKeyPressFeedback()
        showPreviewIfNeeded()
        onPress?()
        super.touchesBegan(touches, with: event)
        ensurePressAnimator().press()
        isPressed = true
        updateGlassAppearance()
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesEnded(touches, with: event)
        ensurePressAnimator().release()
        isPressed = false
        updateGlassAppearance()
        hidePreviewIfNeeded()
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesCancelled(touches, with: event)
        ensurePressAnimator().release()
        isPressed = false
        updateGlassAppearance()
        hidePreviewIfNeeded()
    }

    // The press animation scales the button down to (1 - pressScaleDepth). UIKit
    // hit-tests through that scaled geometry, shrinking the touchable area and
    // leaving a dead ring around the edge where rapid re-taps are dropped (every
    // other fast tap misses). Expand the hit region to cover the full un-squished
    // frame so a touch landing anywhere over the key's layout area still registers,
    // even mid-squish. The margin is the exact inverse of the max scale, so at rest
    // the overlap into the 6pt inter-key gap stays under ~2pt — well clear of
    // neighbours.
    override func point(inside point: CGPoint, with event: UIEvent?) -> Bool {
        let minScale = 1 - Self.pressScaleDepth
        let marginFraction = 0.5 / minScale - 0.5
        let expanded = bounds.insetBy(
            dx: -bounds.width * marginFraction,
            dy: -bounds.height * marginFraction
        )
        return expanded.contains(point)
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        updateBlurViewLayout()
        updateGlassAppearance()
    }

    override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        updateGlassAppearance()
    }

    private func ensurePressAnimator() -> GlassKeyPressAnimator {
        if let existing = pressAnimator { return existing }
        let animator = GlassKeyPressAnimator { [weak self] squish in
            self?.applySquish(squish)
        }
        pressAnimator = animator
        return animator
    }

    private func performKeyPressFeedback() {
        if let inputFeedbackForTesting {
            inputFeedbackForTesting()
            return
        }
        UIDevice.current.playInputClick()
    }

    private func applySquish(_ squish: CGFloat) {
        let scale = 1 - squish * Self.pressScaleDepth
        transform = CGAffineTransform(scaleX: scale, y: scale)
    }

    private func showPreviewIfNeeded() {
        guard let previewLabel else { return }
        isPreviewVisible = true
        onPreviewChanged?(self, previewLabel)
    }

    private func hidePreviewIfNeeded() {
        guard isPreviewVisible else { return }
        isPreviewVisible = false
        onPreviewChanged?(self, nil)
    }

    private func updateBlurViewLayout() {
        let radius = GlassColorSpec.keyCornerRadius(height: bounds.height)
        blurView.frame = bounds
        blurView.layer.cornerRadius = radius
        blurView.clipsToBounds = true
    }

    private func updateGlassAppearance() {
        KeyStyle.updateGlassAppearance(self, isActive: isGlassActive, isPressed: isPressed)
    }
}

enum KeyStyle {

    static func applyLetter(_ btn: UIButton, isIPad: Bool = false) {
        applyGlass(btn, isIPad: isIPad)
        btn.titleLabel?.font = .systemFont(ofSize: isIPad ? 20 : 17)
        btn.setTitleColor(.label, for: .normal)
    }

    static func applySymbol(_ btn: UIButton, isIPad: Bool = false) {
        applyGlass(btn, isIPad: isIPad)
        btn.titleLabel?.font = .systemFont(ofSize: isIPad ? 20 : 17)
        btn.setTitleColor(.label, for: .normal)
    }

    static func applySpecial(_ btn: UIButton, isIPad: Bool = false, isActive: Bool = false) {
        if let glassButton = btn as? GlassKeyButton {
            glassButton.isGlassActive = isActive
        }
        applyGlass(btn, isIPad: isIPad)
        btn.titleLabel?.font = .systemFont(ofSize: isIPad ? 17 : 15, weight: .medium)
    }

    private static func applyGlass(_ btn: UIButton, isIPad _: Bool) {
        updateGlassAppearance(btn, isActive: (btn as? GlassKeyButton)?.isGlassActive ?? false)
        btn.layer.borderWidth = 1
        btn.layer.shadowOpacity = 0
        btn.layer.masksToBounds = false
    }

    fileprivate static func updateGlassAppearance(_ btn: UIButton, isActive: Bool, isPressed: Bool = false) {
        let isDark = btn.traitCollection.userInterfaceStyle == .dark
        // Pressed always wins while held, regardless of active/toggle state.
        // Active state uses flat opaque fill so EN/✦ buttons stand out clearly.
        // Inactive state is transparent — the blurView behind provides glass depth.
        btn.backgroundColor = isPressed
            ? GlassColorSpec.pressedBackground(isDark: isDark)
            : isActive
                ? GlassColorSpec.toggleActiveBackground(isDark: isDark)
                : .clear
        btn.setTitleColor(isActive ? GlassColorSpec.toggleActiveTextColor() : .label, for: .normal)
        btn.layer.cornerRadius = GlassColorSpec.keyCornerRadius(height: btn.bounds.height)
        btn.layer.borderColor = GlassColorSpec.borderColor(isDark: isDark).cgColor
    }
}

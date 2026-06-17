import UIKit

final class GlobeKeyButton: GlassKeyButton {
    static let longPressDuration: TimeInterval = 0.5

    var onShortTap: (() -> Void)?
    var onLongPress: ((_ button: GlobeKeyButton, _ event: UIEvent) -> Void)?

    private var longPressTimer: Timer?
    private var didFireLongPress = false
    private var longPressEvent: UIEvent?

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesBegan(touches, with: event)
        didFireLongPress = false
        longPressEvent = event

        longPressTimer?.invalidate()
        longPressTimer = Timer.scheduledTimer(
            withTimeInterval: Self.longPressDuration,
            repeats: false
        ) { [weak self] _ in
            self?.handleLongPress()
        }
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesEnded(touches, with: event)
        longPressTimer?.invalidate()
        longPressTimer = nil
        longPressEvent = nil
        if !didFireLongPress { onShortTap?() }
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesCancelled(touches, with: event)
        longPressTimer?.invalidate()
        longPressTimer = nil
        longPressEvent = nil
    }

    private func handleLongPress() {
        didFireLongPress = true
        guard let event = longPressEvent else { return }
        onLongPress?(self, event)
    }
}

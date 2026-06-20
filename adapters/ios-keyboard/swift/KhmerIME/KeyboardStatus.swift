import UIKit

// Whether KhmerIME is currently enabled in iOS Settings → Keyboards. The
// keyboard's bundle id is only exposed via KVC ("identifier") on UITextInputMode.
enum KeyboardStatus {
    static let bundleID = "com.khmerime.KhmerIME.Keyboard"

    static var isEnabled: Bool {
        UITextInputMode.activeInputModes.contains {
            ($0.value(forKey: "identifier") as? String) == bundleID
        }
    }
}

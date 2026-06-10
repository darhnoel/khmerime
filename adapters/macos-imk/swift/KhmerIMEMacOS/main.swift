import Cocoa
import InputMethodKit

// Create the IMKServer using the connection name declared in Info.plist.
// This registers the input method with the system and begins listening for
// input events routed by the OS to KhmerInputController instances.
let bundleId = Bundle.main.bundleIdentifier!
let connectionName = "\(bundleId)_Connection"
guard IMKServer(name: connectionName, bundleIdentifier: bundleId) != nil else {
    fputs("KhmerIME: failed to create IMKServer — bundle id: \(bundleId)\n", stderr)
    exit(1)
}

NSApplication.shared.run()

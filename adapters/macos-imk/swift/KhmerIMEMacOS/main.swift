import Cocoa
import InputMethodKit

// Create the IMKServer using the connection name declared in Info.plist.
// This registers the input method with the system and begins listening for
// input events routed by the OS to KhmerInputController instances.
let bundleId = Bundle.main.bundleIdentifier!
let connectionName = "\(bundleId)_Connection"
// Top-level let keeps the server alive for the process lifetime — IMK drops
// the mach connection if this is deallocated.
guard let server = IMKServer(name: connectionName, bundleIdentifier: bundleId) else {
    fputs("KhmerIME: failed to create IMKServer — bundle id: \(bundleId)\n", stderr)
    exit(1)
}
_ = server

NSApplication.shared.run()

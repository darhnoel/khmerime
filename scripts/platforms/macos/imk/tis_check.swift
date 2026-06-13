#!/usr/bin/env swift
// Ground-truth check that the input method is registered with Text Input
// Sources. System Settings uses the same API, so if the IME is missing here
// it will not appear in Keyboard → Input Sources. Lists every Khmer-related
// input source; duplicates usually mean two installed copies (user-level
// and /Library) are both registered.

import Carbon
import Foundation

struct StderrStream: TextOutputStream {
    mutating func write(_ string: String) { FileHandle.standardError.write(Data(string.utf8)) }
}
var stderrStream = StderrStream()

let list = TISCreateInputSourceList(nil, true).takeRetainedValue() as! [TISInputSource]
var found = false
for src in list {
    let id = Unmanaged<CFString>.fromOpaque(
        TISGetInputSourceProperty(src, kTISPropertyInputSourceID)!
    ).takeUnretainedValue() as String
    guard id.lowercased().contains("khmer") else { continue }
    let name = Unmanaged<CFString>.fromOpaque(
        TISGetInputSourceProperty(src, kTISPropertyLocalizedName)!
    ).takeUnretainedValue() as String
    print("\(id) | \(name)")
    if id.contains("com.khmerime.inputmethod") { found = true }
}
if !found {
    print("error: com.khmerime.inputmethod.* not visible to TIS", to: &stderrStream)
    exit(1)
}

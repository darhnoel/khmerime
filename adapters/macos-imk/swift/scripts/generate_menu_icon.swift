#!/usr/bin/env swift
// Generates KhmerIMEMacOS/menu_icon.tiff — the input-menu icon for the IME.
//
// Usage:
//   swift adapters/macos-imk/swift/scripts/generate_menu_icon.swift [output.tiff]
//
// Output is a multi-representation TIFF (16×16 @1x + 32×32 @2x, black glyph on
// transparent background), matching the format Apple uses for its own input
// methods. Edit the constants below and re-run to change the icon.

import AppKit

// ខ (KHA) — first letter of ខ្មែរ "Khmer". Deliberately NOT ក, which is the
// menu label of Apple's built-in Khmer keyboard layout (com.apple.keylayout.Khmer).
let glyph = "ខ"
let fontName = "Khmer Sangam MN" // ships with macOS; UI sans for Khmer
let pointSize: CGFloat = 16      // logical menu-bar icon size
let fillRatio: CGFloat = 0.85    // fraction of the canvas the glyph occupies

func renderRep(scale: Int) -> NSBitmapImageRep {
    let pixels = Int(pointSize) * scale
    guard let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0
    ) else { fatalError("could not create \(pixels)px bitmap rep") }

    guard let ctx = NSGraphicsContext(bitmapImageRep: rep) else {
        fatalError("could not create graphics context")
    }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = ctx
    let cg = ctx.cgContext

    guard let font = NSFont(name: fontName, size: 100) else {
        fatalError("font \"\(fontName)\" not found")
    }
    let attributed = NSAttributedString(
        string: glyph,
        attributes: [.font: font, .foregroundColor: NSColor.black]
    )
    let line = CTLineCreateWithAttributedString(attributed)
    // Tight ink bounds (not typographic metrics) so the glyph is centered
    // and scaled by what is actually drawn.
    let ink = CTLineGetImageBounds(line, cg)

    let canvas = CGFloat(pixels)
    let s = canvas * fillRatio / max(ink.width, ink.height)
    cg.translateBy(x: canvas / 2, y: canvas / 2)
    cg.scaleBy(x: s, y: s)
    cg.translateBy(x: -ink.midX, y: -ink.midY)
    cg.textPosition = .zero
    CTLineDraw(line, cg)

    NSGraphicsContext.restoreGraphicsState()
    // Logical size in points; gives the @2x rep its 144 dpi tag.
    rep.size = NSSize(width: pointSize, height: pointSize)
    return rep
}

let scriptDir = URL(fileURLWithPath: CommandLine.arguments[0])
    .deletingLastPathComponent()
let defaultOut = scriptDir
    .appendingPathComponent("../KhmerIMEMacOS/menu_icon.tiff")
    .standardizedFileURL
let outURL = CommandLine.arguments.count > 1
    ? URL(fileURLWithPath: CommandLine.arguments[1])
    : defaultOut

let reps = [renderRep(scale: 1), renderRep(scale: 2)]
guard let tiff = NSBitmapImageRep.representationOfImageReps(
    in: reps, using: .tiff, properties: [:]
) else { fatalError("could not encode TIFF") }

try tiff.write(to: outURL)
print("wrote \(outURL.path) (\(reps.map { "\($0.pixelsWide)px" }.joined(separator: " + ")))")

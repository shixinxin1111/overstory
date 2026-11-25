import AppKit

// Export high-resolution UI symbols for vector tracing, never app branding.
let symbols = [
    "todos": "checklist", "bookmarks": "globe", "notes": "note.text",
    "plus": "plus", "search": "magnifyingglass", "settings": "gearshape",
    "expand": "macwindow", "panel": "pip",
    "collapse": "chevron.down", "back": "chevron.left", "close": "xmark",
    "check": "checkmark", "trash": "trash", "edit": "square.and.pencil",
    "external": "arrow.up.right", "chevron": "chevron.right", "folder": "folder",
    "download": "square.and.arrow.down", "upload": "square.and.arrow.up",
    "more": "ellipsis", "spark": "sparkle", "retry": "arrow.clockwise",
    "favorite": "star", "favoriteFill": "star.fill"
]
let output = URL(fileURLWithPath: CommandLine.arguments.last!, isDirectory: true)
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
for (key, name) in symbols.sorted(by: { $0.key < $1.key }) {
    guard let original = NSImage(systemSymbolName: name, accessibilityDescription: nil),
          let symbol = original.withSymbolConfiguration(
            NSImage.SymbolConfiguration(pointSize: 320, weight: .regular)
                .applying(.init(paletteColors: [.black]))
          ) else { fatalError("Unavailable SF Symbol: \(name)") }
    let bitmap = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: 384, pixelsHigh: 384,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
        isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0
    )!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    NSColor.white.setFill()
    NSRect(x: 0, y: 0, width: 384, height: 384).fill()
    symbol.isTemplate = false
    let scale = min(320 / symbol.size.width, 320 / symbol.size.height)
    let size = NSSize(width: symbol.size.width * scale, height: symbol.size.height * scale)
    symbol.draw(in: NSRect(x: (384 - size.width) / 2, y: (384 - size.height) / 2,
                          width: size.width, height: size.height))
    NSGraphicsContext.restoreGraphicsState()
    try bitmap.representation(using: .png, properties: [:])!
        .write(to: output.appendingPathComponent("\(key).png"))
    print("\(key): \(name) → \(output.appendingPathComponent("\(key).png").path)")
}

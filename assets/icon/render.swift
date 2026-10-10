import AppKit

// usage: swift render.swift <in.svg> <out-dir>
// Writes the macOS bundle PNGs and icon.ico for the Windows .exe.
let args = CommandLine.arguments
let svg = NSImage(contentsOf: URL(fileURLWithPath: args[1]))!
let outDir = URL(fileURLWithPath: args[2])

/// Renders the SVG as a `px` square PNG. `scale` > 1 crops the margin around the tile.
func render(_ px: Int, scale: Double = 1) -> Data {
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: px, pixelsHigh: px,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = NSSize(width: px, height: px)
    NSGraphicsContext.saveGraphicsState()
    let ctx = NSGraphicsContext(bitmapImageRep: rep)!
    ctx.imageInterpolation = .high
    NSGraphicsContext.current = ctx
    let side = Double(px) * scale
    let origin = (Double(px) - side) / 2
    svg.draw(in: NSRect(x: origin, y: origin, width: side, height: side))
    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])!
}

// macOS keeps Apple's grid: an 824 tile inside 1024, margin included.
let bundlePngs: [(String, Int)] = [
    ("16x16.png", 16), ("16x16@2x.png", 32),
    ("32x32.png", 32), ("32x32@2x.png", 64),
    ("128x128.png", 128), ("128x128@2x.png", 256),
    ("256x256.png", 256), ("256x256@2x.png", 512),
    ("512x512.png", 512), ("512x512@2x.png", 1024),
]
for (name, px) in bundlePngs {
    try! render(px).write(to: outDir.appendingPathComponent(name))
}

// Windows icons fill more of their square: the tile covers 94% of each size.
// ICO: 6-byte header, one 16-byte entry per image, then the PNG data.
let icoSizes = [16, 24, 32, 48, 64, 128, 256]
let icoScale = 0.94 * 1024 / 824
let images = icoSizes.map { render($0, scale: icoScale) }
var ico = Data()
func u16(_ v: Int) { ico.append(contentsOf: [UInt8(v & 0xFF), UInt8(v >> 8 & 0xFF)]) }
func u32(_ v: Int) { u16(v & 0xFFFF); u16(v >> 16) }
u16(0); u16(1); u16(icoSizes.count)
var offset = 6 + 16 * icoSizes.count
for (px, png) in zip(icoSizes, images) {
    let side = UInt8(px == 256 ? 0 : px)
    ico.append(contentsOf: [side, side, 0, 0])
    u16(1); u16(32); u32(png.count); u32(offset)
    offset += png.count
}
images.forEach { ico.append($0) }
try! ico.write(to: outDir.appendingPathComponent("icon.ico"))

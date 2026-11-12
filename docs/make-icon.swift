import AppKit
import Foundation

let size = 1024.0
let img = NSImage(size: NSSize(width: size, height: size))
img.lockFocus()
guard let ctx = NSGraphicsContext.current?.cgContext else { exit(1) }

// Squircle body, inset to macOS icon proportions.
let inset = 82.0
let body = CGRect(x: inset, y: inset, width: size - 2 * inset, height: size - 2 * inset)
let rounded = CGPath(roundedRect: body, cornerWidth: 190, cornerHeight: 190, transform: nil)

ctx.saveGState()
ctx.addPath(rounded)
ctx.clip()
let colors = [
    NSColor(srgbRed: 0.11, green: 0.14, blue: 0.21, alpha: 1).cgColor,
    NSColor(srgbRed: 0.04, green: 0.06, blue: 0.10, alpha: 1).cgColor,
] as CFArray
let gradient = CGGradient(
    colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colors, locations: [0, 1])!
ctx.drawLinearGradient(
    gradient, start: CGPoint(x: 0, y: size), end: CGPoint(x: 0, y: 0), options: [])
ctx.restoreGState()

// Hairline rim.
ctx.addPath(rounded)
ctx.setStrokeColor(NSColor(white: 1, alpha: 0.09).cgColor)
ctx.setLineWidth(7)
ctx.strokePath()

// Three stacked bars: a stack of services, narrowing upward.
let barColor = NSColor(srgbRed: 0.31, green: 0.64, blue: 1.0, alpha: 1)
let barHeight = 100.0
let widths = [560.0, 455.0, 350.0]
let alphas = [1.0, 0.82, 0.62]
var y = 250.0
for index in 0..<widths.count {
    let width = widths[index]
    let bar = CGRect(x: (size - width) / 2, y: y, width: width, height: barHeight)
    let path = CGPath(roundedRect: bar, cornerWidth: 36, cornerHeight: 36, transform: nil)
    ctx.addPath(path)
    ctx.setFillColor(barColor.withAlphaComponent(alphas[index]).cgColor)
    ctx.fillPath()
    y += barHeight + 46
}

img.unlockFocus()

let rep = NSBitmapImageRep(data: img.tiffRepresentation!)!
let png = rep.representation(using: .png, properties: [:])!
try png.write(to: URL(fileURLWithPath: "/tmp/stackdeck-icon.png"))
print("written")

"""Generate Overstory's original application icon; requires Pillow."""
from pathlib import Path
from math import hypot
from PIL import Image, ImageDraw

out = Path(__file__).resolve().parents[1] / "src-tauri" / "icons"
out.mkdir(parents=True, exist_ok=True)
size = 1024
image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
pixels = image.load()
stops = [(0, (255, 255, 255)), (.08, (220, 240, 255)), (.25, (129, 168, 207)),
         (.55, (49, 90, 130)), (.8, (20, 49, 76)), (1, (93, 158, 216))]
for y in range(80, 944):
    for x in range(80, 944):
        radius = hypot(x - 512, y - 512) / 426
        if radius > 1:
            continue
        light = min(1., hypot(x - 350, y - 225) / 740)
        for (a, ca), (b, cb) in zip(stops, stops[1:]):
            if a <= light <= b:
                t = (light - a) / (b - a)
                rgb = tuple(round(u + (v - u) * t) for u, v in zip(ca, cb))
                break
        edge = max(0, (radius - .97) / .03)
        rgb = tuple(round(c * (1 - edge * .4) + 245 * edge * .4) for c in rgb)
        pixels[x, y] = (*rgb, 255)
draw = ImageDraw.Draw(image)
draw.polygon([(512, 340), (558, 466), (684, 512), (558, 558),
              (512, 684), (466, 558), (340, 512), (466, 466)], fill=(246, 251, 255, 255))
image.save(out / "icon.png")
for n in [32, 128]:
    image.resize((n, n), Image.Resampling.LANCZOS).save(out / f"{n}x{n}.png")
image.resize((256, 256), Image.Resampling.LANCZOS).save(out / "128x128@2x.png")
image.save(out / "icon.icns", sizes=[(16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512), (1024, 1024)])
print(out / "icon.icns")

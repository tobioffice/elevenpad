#!/usr/bin/env python3
"""Generate the Notepad app icon used by the desktop entry and AppImage."""
from PIL import Image, ImageDraw

S = 512
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# blue rounded square
d.rounded_rectangle((40, 40, S - 40, S - 40), radius=96, fill=(0x00, 0x78, 0xD4, 255))

# white page
page = (120, 96, S - 120, S - 72)
d.rounded_rectangle(page, radius=18, fill=(255, 255, 255, 255))

# text lines
for i in range(5):
    y = page[1] + 52 + i * 46
    left = page[0] + 28
    right = page[2] - 28 - (60 if i % 2 else 0)
    d.rounded_rectangle((left, y, right, y + 14), radius=7, fill=(0x00, 0x78, 0xD4, 255))

# spiral binding
for i in range(5):
    y = page[1] + 34 + i * 66
    d.ellipse((page[0] - 16, y - 10, page[0] + 16, y + 10), fill=(0xE8, 0xE8, 0xE8, 255))
    d.ellipse((page[0] - 8, y - 6, page[0] + 8, y + 6), fill=(0x00, 0x5A, 0xA0, 255))

img.save("assets/icon.png")
img.resize((256, 256), Image.LANCZOS).save("assets/icon-256.png")
print("wrote assets/icon.png")

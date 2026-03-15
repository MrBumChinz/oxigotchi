#!/usr/bin/env python3
"""Oxigotchi splash screen — renders a face PNG centered on the waveshare 2.13" V4 e-ink display.
Matches pwnagotchi's own rendering: white canvas (255), black art (0), no inversion needed."""

import sys
import os

def main():
    if len(sys.argv) < 2:
        print("Usage: oxigotchi-splash.py <png_path>")
        sys.exit(1)

    png_path = sys.argv[1]

    if not os.path.exists(png_path):
        print(f"File not found: {png_path}")
        sys.exit(1)

    try:
        from PIL import Image
        from pwnagotchi.ui.hw.libs.waveshare.epaper.v2in13_V4 import epd2in13_V4

        epd = epd2in13_V4.EPD()
        epd.init()

        # Load face
        face = Image.open(png_path).convert('L')
        fw, fh = face.size

        # Scale to fill display
        scale = min(250 / fw, 122 / fh)
        new_w = int(fw * scale)
        new_h = int(fh * scale)
        face = face.resize((new_w, new_h), Image.LANCZOS)

        # White canvas (255) — matches pwnagotchi's own Image.new('1', ..., 255)
        canvas = Image.new('L', (250, 122), 255)

        # Center
        ox = (250 - new_w) // 2
        oy = (122 - new_h) // 2
        canvas.paste(face, (ox, oy))

        # Rotate 180 (display mounted upside down)
        canvas = canvas.transpose(Image.ROTATE_180)

        # Convert to 1-bit — same as pwnagotchi does
        canvas = canvas.point(lambda p: 255 if p > 128 else 0, '1')

        # Send to display
        buf = epd.getbuffer(canvas)
        try:
            epd.displayPartBaseImage(buf)
        except Exception:
            epd.displayPartial(buf)

    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)
    finally:
        try:
            epd2in13_V4.epdconfig.module_exit(cleanup=True)
        except Exception:
            pass

if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Oxigotchi splash screen — renders a face PNG centered on the waveshare 2.13" V4 e-ink display.
Matches pwnagotchi's own rendering: white canvas (255), black art (0), no inversion needed.

Uses a full display refresh (epd.display) so the image is written to both the
current and previous-image RAM banks.  This means the bull face survives partial
refresh cycles that pwnagotchi performs later during its own display init.
A sentinel file (/tmp/.oxigotchi-splash-done) is written after the image has
been pushed so downstream services can gate on it."""

import sys
import os
import time

SENTINEL = "/tmp/.oxigotchi-splash-done"

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

        # Full display refresh — writes image to both RAM banks so it persists
        # through partial refreshes and survives pwnagotchi's Clear() + base image init
        buf = epd.getbuffer(canvas)
        epd.display(buf)

        # Write sentinel so downstream services know splash is rendered
        with open(SENTINEL, 'w') as f:
            f.write(str(time.time()))

    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)
    finally:
        try:
            # Release SPI/GPIO so pwnagotchi can claim them
            epd2in13_V4.epdconfig.module_exit(cleanup=True)
        except Exception:
            pass

if __name__ == '__main__':
    main()

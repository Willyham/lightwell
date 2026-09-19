#!/usr/bin/env python3
"""Generate public synthetic inputs only; never read a photographer's originals."""
import argparse
import hashlib
import json
import struct
from pathlib import Path

from PIL import Image, ImageCms, ImageDraw, __version__

ROOT = Path(__file__).resolve().parents[1]
COLORS = [(220, 35, 45), (35, 190, 65), (40, 70, 220), (235, 195, 30)]


def pattern(width, height):
    image = Image.new('RGB', (width, height))
    draw = ImageDraw.Draw(image)
    for box, color in zip([(0, 0, width//2-1, height//2-1), (width//2, 0, width-1, height//2-1), (0, height//2, width//2-1, height-1), (width//2, height//2, width-1, height-1)], COLORS):
        draw.rectangle(box, fill=color)
    for i, label in enumerate(['TOP LEFT / RED', 'TOP RIGHT / GREEN', 'BOTTOM LEFT / BLUE', 'BOTTOM RIGHT / GOLD']):
        draw.text((10 + (i % 2) * (width//2), 10 + (i//2)*(height//2)), label, fill='white', font_size=16)
    draw.line((width//2, 30, width//2, height-30), fill='white', width=3)
    draw.polygon([(width//2, 30), (width//2-15, 55), (width//2+15, 55)], fill='white')
    for x in range(20, min(width-20, 150), 4):
        draw.line((x, height//2-20, x, height//2+20), fill='black', width=1)
    return image


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / 'fixtures/s0')
    parser.add_argument('--large', action='store_true', help='Also generate 24/60 MP in ignored fixtures/generated')
    args = parser.parse_args()
    if __version__ != '12.2.0':
        raise SystemExit('Use tools/fixture-requirements.txt (Pillow 12.2.0)')
    args.output.mkdir(parents=True, exist_ok=True)
    entries = []

    def save(name, image, orientation=1, profile=None, outcome='supported'):
        exif = Image.Exif()
        exif[274] = orientation
        options = {'quality': 95, 'subsampling': 0, 'exif': exif}
        if profile is not None:
            options['icc_profile'] = profile
        path = args.output / name
        image.save(path, 'JPEG', **options)
        w, h = image.size
        entries.append({'file': name, 'width': w, 'height': h, 'orientation': orientation, 'display_width': h if orientation >= 5 else w, 'display_height': w if orientation >= 5 else h, 'mode': image.mode, 'profile': 'untagged-assume-sRGB' if profile is None else 'embedded', 'expected': outcome})

    landscape = pattern(480, 320)
    for orientation in range(1, 9):
        save(f'orientation-{orientation}.jpg', landscape, orientation)
    save('portrait.jpg', pattern(320, 480))
    save('greyscale.jpg', landscape.convert('L'))
    # Normalize the ICC creation time so generation is deterministic.
    icc = bytearray(ImageCms.ImageCmsProfile(ImageCms.createProfile('sRGB')).tobytes())
    icc[24:36] = struct.pack('>6H', 2026, 1, 1, 0, 0, 0)
    save('srgb.jpg', landscape, profile=bytes(icc))
    save('cmyk.jpg', landscape.convert('CMYK'), outcome='unsupported-color')
    save('invalid-profile.jpg', landscape, profile=b'not-an-ICC-profile', outcome='unsupported-profile')
    raw = (args.output / 'orientation-1.jpg').read_bytes()
    # Oversized JPEG SOF dimensions, without allocating an oversized raster.
    oversized = bytearray(raw)
    sof = oversized.index(b'\xff\xc0')
    oversized[sof+5:sof+9] = struct.pack('>HH', 65535, 65535)
    for name, data, expected in [('invalid.jpg', b'not a JPEG\n', 'invalid-input'), ('truncated.jpg', raw[:len(raw)//3], 'invalid-input'), ('oversized.jpg', oversized, 'resource-limit')]:
        (args.output / name).write_bytes(data)
        entries.append({'file': name, 'expected': expected})
    for entry in entries:
        entry['sha256'] = hashlib.sha256((args.output / entry['file']).read_bytes()).hexdigest()
    (args.output / 'manifest.json').write_text(json.dumps({'generator': 'Pillow 12.2.0', 'entries': entries}, indent=2)+'\n', encoding="utf-8")
    if args.large:
        output = ROOT / 'fixtures/generated'
        output.mkdir(parents=True, exist_ok=True)
        large = []
        for w, h in [(6000, 4000), (10000, 6000)]:
            path = output / f'{w*h//1000000}mp.jpg'
            pattern(w, h).save(path, quality=95, subsampling=0)
            large.append({'file': path.name, 'width': w, 'height': h, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
        (output / 'manifest.json').write_text(json.dumps(large, indent=2)+'\n', encoding="utf-8")
    print(f'Generated {len(entries)} fixtures in {args.output}')


if __name__ == '__main__':
    main()

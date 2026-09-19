#!/usr/bin/env python3
"""Check corpus hashes, header data, independent corner expectations and regeneration."""
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageCms, ImageOps, features

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'fixtures/s0'
# TL, TR, BL, BR after EXIF transform, specified independently of ImageOps.
EXPECTED = {1: 'RGBY', 2: 'GRYB', 3: 'YBGR', 4: 'BYRG', 5: 'RBGY', 6: 'BRYG', 7: 'YGBR', 8: 'GYRB'}
COLORS = {'R': (220, 35, 45), 'G': (35, 190, 65), 'B': (40, 70, 220), 'Y': (235, 195, 30)}


def main():
    manifest = json.loads((FIXTURES / 'manifest.json').read_text())
    for item in manifest['entries']:
        path = FIXTURES / item['file']
        assert hashlib.sha256(path.read_bytes()).hexdigest() == item['sha256'], path
        if item['expected'] != 'supported':
            continue
        with Image.open(path) as image:
            assert image.size == (item['width'], item['height']), path
            assert image.getexif().get(274, 1) == item['orientation'], path
            assert image.mode == item['mode'], path
            display = ImageOps.exif_transpose(image)
            assert display.size == (item['display_width'], item['display_height']), path
            if path.name.startswith('orientation-'):
                w, h = display.size
                positions = [(w//4, h//4), (3*w//4, h//4), (w//4, 3*h//4), (3*w//4, 3*h//4)]
                for point, color in zip(positions, EXPECTED[item['orientation']]):
                    assert max(abs(a-b) for a, b in zip(display.getpixel(point), COLORS[color])) <= 5, (path, point)
            if path.name == 'srgb.jpg':
                import io
                profile = ImageCms.ImageCmsProfile(io.BytesIO(image.info['icc_profile']))
                assert 'sRGB' in ImageCms.getProfileDescription(profile)
    # Full decoding must reject both invalid and truncated streams.
    for name in ['invalid.jpg', 'truncated.jpg']:
        try:
            with Image.open(FIXTURES / name) as image:
                image.load()
        except OSError:
            pass
        else:
            raise AssertionError(f'{name} decoded unexpectedly')
    # Read SOF directly: never ask the decoder to allocate 4.3 billion pixels.
    raw = (FIXTURES / 'oversized.jpg').read_bytes()
    offset = raw.index(b'\xff\xc0')
    assert raw[offset+5:offset+9] == b'\xff\xff\xff\xff'
    with tempfile.TemporaryDirectory(prefix='lightwell-fixture-check-') as tmp:
        subprocess.run([sys.executable, str(ROOT / 'tools/generate_fixtures.py'), '--output', tmp], check=True)
        for file in FIXTURES.iterdir():
            assert file.read_bytes() == (Path(tmp) / file.name).read_bytes(), f'Nonreproducible: {file.name}'
    print(f'PASS {len(manifest["entries"])} fixtures; all orientations, error inputs and byte reproduction; JPEG {features.version_codec("jpg")}')


if __name__ == '__main__':
    main()

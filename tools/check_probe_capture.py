#!/usr/bin/env python3
"""Verify the orientation-6 trial screenshot; never treat PNG existence as a pass."""
import argparse
import json
from pathlib import Path
from PIL import Image


def check(path, orientation=6, source_aspect=None):
    image = Image.open(path).convert('RGB')
    # Match saturated interiors, excluding UI/text; permutation is independent of renderer state.
    base=[(220,35,45),(35,190,65),(40,70,220),(235,195,30)]
    orders=[[0,1,2,3],[1,0,3,2],[3,2,1,0],[2,3,0,1],[0,2,1,3],[2,0,3,1],[3,1,2,0],[1,3,0,2]]
    colors=[base[i] for i in orders[orientation-1]]
    aspect=source_aspect if source_aspect is not None else (2/3 if orientation>=5 else 3/2)
    xs, ys = [], []
    for y in range(0, image.height, 4):
        for x in range(0, image.width, 4):
            pixel = image.getpixel((x,y))
            if any(max(abs(a-b) for a,b in zip(pixel,color)) <= 8 for color in colors):
                xs.append(x); ys.append(y)
    assert xs, 'No fixture pixels: blank or wrong render'
    left, right, top, bottom = min(xs), max(xs)+4, min(ys), max(ys)+4
    width, height = right-left, bottom-top
    assert width > image.width * 0.2 and height > image.height * 0.5, 'Fixture too small/missing'
    assert abs(width/height - aspect) < 0.015, 'Incorrect Fit aspect ratio'
    assert abs((left+right)/2 - image.width/2) <= 5, 'Image not centered'
    actual = []
    for (fx,fy), expected in zip([(0.25,0.25),(0.75,0.25),(0.25,0.75),(0.75,0.75)], colors):
        pixel = image.getpixel((round(left+fx*width),round(top+fy*height)))
        assert max(abs(a-b) for a,b in zip(pixel,expected)) <= 8, f'Wrong orientation/color: {pixel}, expected {expected}'
        actual.append(pixel)
    return {'status':'passed', 'physical_size':image.size, 'image_bounds':[left,top,right,bottom], 'corner_rgb':actual, 'tolerance_per_channel':8, 'scope':f'orientation-{orientation} Fit geometry and sRGB interiors; not monitor calibration or native dialog verification'}


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture',type=Path)
    args=parser.parse_args()
    result=check(args.capture)
    args.capture.with_suffix('.pixels.json').write_text(json.dumps(result,indent=2)+'\n', encoding="utf-8")
    print(json.dumps(result))

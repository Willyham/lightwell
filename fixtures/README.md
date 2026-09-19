# S0 synthetic fixture manifest

These images are generated entirely by [generate_fixtures.py](../tools/generate_fixtures.py); no private photos, camera metadata, downloaded art or bundled font assets are used. Text is drawn using Pillow's built-in default font. The source generator and generated images inherit the eventual project license; no third-party photo permission is required. GPL-3.0-or-later is now selected; repository license application remains TASK-005.

Install optional generation tooling in a local environment (network needed on first install):

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r tools/fixture-requirements.txt
.venv/bin/python tools/generate_fixtures.py --large
.venv/bin/python tools/check_fixtures.py
```

On Windows use `.venv\Scripts\python` instead of `.venv/bin/python`. Routine repository checks do not require Pillow. Pillow is pinned to 12.2.0; JPEG bytes may vary with its linked codec build, so the checker reports that build and checks reproduction against the checked-in hashes. This is not a promise of byte-identical encoding across arbitrary libjpeg builds.

## Inputs and expected results

The checked-in [JSON manifest](s0/manifest.json) provides SHA-256, encoded dimensions, mode, EXIF orientation, expected displayed dimensions and outcome per file. Each landscape has distinct red/green/blue/gold quadrants, corner labels, an upward arrow and fine vertical detail. Original raster corner order is red, green, blue, gold (TL, TR, BL, BR); fixtures deliberately use an asymmetric landscape.

| Input | Expected behavior |
| --- | --- |
| `orientation-1.jpg` through `orientation-8.jpg` | 480×320 encoded, untagged assumed sRGB. Orientations 1–4 display 480×320; 5–8 display 320×480. Explicit independent corner expectations in checker |
| `portrait.jpg` | 320×480, EXIF 1, untagged sRGB |
| `greyscale.jpg` | 480×320 luminance; neutral output after RGB expansion |
| `srgb.jpg` | Same landscape, valid embedded synthetic sRGB ICC profile with normalized creation timestamp |
| `cmyk.jpg` | Valid CMYK JPEG; expected explicit unsupported-color error for proposed S0 subset |
| `invalid-profile.jpg` | RGB JPEG with invalid ICC payload; expected explicit unsupported-profile error |
| `invalid.jpg`, `truncated.jpg` | Invalid header / incomplete entropy stream; report failure and retain last successful image |
| `oversized.jpg` | Header declares 65535×65535; reject before raster allocation. Do not decode with unlimited allocation |
| Generated `24mp.jpg`, `60mp.jpg` | 6000×4000 and 10000×6000, untagged sRGB, generated on demand in ignored `fixtures/generated/` |

`--output` permits regeneration into a temporary directory without touching originals. File-not-found uses an intentionally absent path; unreadable-file tests need a platform user/ACL that cannot read it (chmod alone is unreliable as root). Repeated open and cancellation reuse orientation-1, orientation-6 and the generated workloads. Hash sources before and after application runs to verify preservation. No fixtures currently prove renderer correctness or an application resource limit; those checks belong to TASK-036/050.

## Later inputs and baseline requirements

M1 needs geometry goldens for centered proportional Option scaling, straightening and rotated crops; export profile/metadata/GPS/thumbnail fixtures; and moved, changed, duplicate and inaccessible originals for Locate. Broader ICC inputs (Adobe RGB and Display P3) need licensed/generated profiles before professional color claims. Nikon Z6 and Fujifilm X100VI RAW recording modes require owner-supplied samples stored outside tracked source, with local provenance and permissions. None blocks the S0 fixture corpus.

Before measuring, record CPU/GPU, RAM, OS, display resolution/scale/profile, storage medium, compiler/lockfile/build profile, fixture hash, renderer/backend, and cold/warm method. Available host inventory: M4 Pro, 14 CPU/20 GPU cores, 48 GB, macOS 26.5.2; display/storage conditions remain unmeasured. Synthetic 100k/1m metadata rows are later database workloads, not proof of real-image browsing performance.

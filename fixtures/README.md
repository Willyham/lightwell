# S0 synthetic fixture manifest

The sixteen checked-in JPEGs are synthetic golden inputs, covered by the repository GPL-3.0-or-later license. Their manifest records expected behavior and SHA-256 hashes.

All maintained fixture tooling is now [Rust xtask](../xtask/src/fixtures.rs):

```sh
cargo xtask fixtures
cargo xtask generate-fixtures
```

`fixtures` verifies golden hashes, encoded/oriented dimensions, orientation, grayscale/profile presence, supported/error outcomes and independent corner expectations. It also checks repeatability of the Rust generator. The content checks are not independent ICC colorimetry.

`generate-fixtures` creates 24/60 MP workloads in a new ignored `fixtures/generated/` directory. Use `--output NEW_DIRECTORY` for another destination. It refuses an existing directory. Rust `image` 0.25.9 encoding and `pattern-v1` produce quadrant colors, an arrow and detail marks without fonts or external tools. The generated manifest records their hashes; comparisons must use identical inputs.

## Inputs and expected results

The checked-in [JSON manifest](s0/manifest.json) provides SHA-256, encoded dimensions, mode, EXIF orientation, expected displayed dimensions and outcome per file. Each checked-in landscape has distinct red/green/blue/gold quadrants, corner labels, an upward arrow and fine vertical detail. Original raster corner order is red, green, blue, gold (TL, TR, BL, BR); fixtures deliberately use an asymmetric landscape.

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

`generate-fixtures --output` permits new large workloads in a fresh directory without touching originals. File-not-found uses an intentionally absent path; unreadable-file tests need a platform user/ACL that cannot read it (chmod alone is unreliable as root). Repeated open and cancellation reuse orientation-1, orientation-6 and the generated workloads. Hash sources before and after application runs to verify preservation. Fixture content checks complement the renderer evidence from TASK-036/050.

## Later inputs and baseline requirements

M1 needs geometry goldens for centered proportional Option scaling, straightening and rotated crops; export profile/metadata/GPS/thumbnail fixtures; and moved, changed, duplicate and inaccessible originals for Locate. Broader ICC inputs (Adobe RGB and Display P3) need licensed/generated profiles before professional color claims. Nikon Z6 and Fujifilm X100VI RAW recording modes require owner-supplied samples stored outside tracked source, with local provenance and permissions. None blocks the S0 fixture corpus.

Before measuring, record CPU/GPU, RAM, OS, display resolution/scale/profile, storage medium, compiler/lockfile/build profile, fixture hash, renderer/backend, and cold/warm method. Available host inventory: M4 Pro, 14 CPU/20 GPU cores, 48 GB, macOS 26.5.2; display/storage conditions remain unmeasured. Synthetic 100k/1m metadata rows are later database workloads, not proof of real-image browsing performance.

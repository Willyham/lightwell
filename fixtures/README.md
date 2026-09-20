# Synthetic fixtures

The sixteen checked-in JPEGs under `s0/` are synthetic golden inputs covered by the repository license. `s0/manifest.json` records SHA-256, encoded dimensions, mode, EXIF orientation, expected displayed dimensions and expected outcome per file.

```sh
cargo xtask fixtures
cargo xtask generate-fixtures [--output NEW_DIRECTORY]
```

`fixtures` verifies golden hashes, dimensions, orientation, greyscale and profile presence, supported or error outcomes and independent corner expectations, and checks that the Rust generator is repeatable. `generate-fixtures` writes 24 MP (6000×4000) and 60 MP (10000×6000) untagged sRGB workloads into a new ignored `fixtures/generated/` directory and refuses an existing one. The content checks are not independent ICC colorimetry.

Each landscape fixture has red, green, blue and gold quadrants (top-left, top-right, bottom-left, bottom-right), corner labels, an upward arrow and fine vertical detail, so orientation and reflection errors are visible.

| Input | Expected behavior |
| --- | --- |
| `orientation-1.jpg` to `orientation-8.jpg` | 480×320 encoded, untagged sRGB. Orientations 1 to 4 display 480×320; 5 to 8 display 320×480 |
| `portrait.jpg` | 320×480, EXIF 1, untagged sRGB |
| `greyscale.jpg` | 480×320 luminance; neutral output after RGB expansion |
| `srgb.jpg` | Landscape with a valid embedded synthetic sRGB ICC profile |
| `cmyk.jpg` | Valid CMYK JPEG; explicit unsupported-color error |
| `invalid-profile.jpg` | RGB JPEG with an invalid ICC payload; explicit unsupported-profile error |
| `invalid.jpg`, `truncated.jpg` | Invalid header and incomplete entropy stream; report failure and keep the last successful image |
| `oversized.jpg` | Header declares 65535×65535; rejected before raster allocation |

`history/m2-journey.json` is a catalog journey recorded with the pre-module editor against `s0/orientation-6.jpg`: full history entries, navigation state, versions, request hashes and sampled pixels per entry. The core continuity test rebuilds that catalog around a local copy of the fixture and requires the module-based editor to reproduce it exactly. Regenerate it only from a pre-module build; it is evidence, not a template.

Private originals for local diagnostics go in ignored `fixtures/jpg/` or `private/` and are never committed. Crop and straightening geometry goldens (rotated-box mapping, output rounding, coverage and fitting, at off-center, near-boundary, portrait, landscape and every ±45° reference angle) live as code tests in `crates/lightwell-core/src/modules/crop/geometry.rs`; no new binary fixtures were added for M4. Later work still needs export profile and metadata fixtures, moved and changed originals for Locate, Adobe RGB and Display P3 inputs before any wide-gamut claim, and owner-supplied Nikon Z6 and Fujifilm X100VI samples stored outside tracked source.

use crate::{
    Error, ErrorKind, Recipe, SnapshotId, SourceImage,
    modules::{ExactGeometry, ModuleRegistry, Processing, Stage},
};
use rayon::prelude::*;
use std::{collections::HashSet, sync::Arc};

const PARALLEL_RENDER_PIXELS: u64 = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<[u8]>,
    pub source_fingerprint: String,
    pub snapshot_id: SnapshotId,
}

impl Raster {
    fn expected_len(width: u32, height: u32) -> Result<usize, Error> {
        let pixels = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| Error::new(ErrorKind::ResourceLimit, "image dimensions overflow"))?;
        if pixels > 512 * 1024 * 1024 {
            return Err(Error::new(
                ErrorKind::ResourceLimit,
                "evaluated image exceeds 512 MiB",
            ));
        }
        usize::try_from(pixels).map_err(|_| {
            Error::new(
                ErrorKind::ResourceLimit,
                "image allocation is not addressable",
            )
        })
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = ((u64::from(y) * u64::from(self.width) + u64::from(x)) * 4) as usize;
        self.rgba
            .get(offset..offset + 4)
            .map(|p| [p[0], p[1], p[2], p[3]])
    }
}

/// Composition and mapping of exact geometry belong to the host; modules only declare one step.
impl ExactGeometry {
    pub(crate) fn identity(width: u32, height: u32) -> Self {
        Self {
            a: 1,
            b: 0,
            c: 0,
            d: 1,
            tx: 0,
            ty: 0,
            output_width: width,
            output_height: height,
        }
    }

    /// Compose `self` followed by `next`.
    pub(crate) fn then(self, next: Self) -> Self {
        Self {
            a: next.a * self.a + next.b * self.c,
            b: next.a * self.b + next.b * self.d,
            c: next.c * self.a + next.d * self.c,
            d: next.c * self.b + next.d * self.d,
            tx: next.a * self.tx + next.b * self.ty + next.tx,
            ty: next.c * self.tx + next.d * self.ty + next.ty,
            output_width: next.output_width,
            output_height: next.output_height,
        }
    }

    fn map(self, x: u32, y: u32) -> (u32, u32) {
        let out_x = self.a * i64::from(x) + self.b * i64::from(y) + self.tx;
        let out_y = self.c * i64::from(x) + self.d * i64::from(y) + self.ty;
        debug_assert!(out_x >= 0 && out_x < i64::from(self.output_width));
        debug_assert!(out_y >= 0 && out_y < i64::from(self.output_height));
        (out_x as u32, out_y as u32)
    }

    fn unmap(self, x: u32, y: u32) -> (u32, u32) {
        let translated_x = i64::from(x) - self.tx;
        let translated_y = i64::from(y) - self.ty;
        let input_x = self.a * translated_x + self.c * translated_y;
        let input_y = self.b * translated_x + self.d * translated_y;
        debug_assert!(input_x >= 0 && input_y >= 0);
        (input_x as u32, input_y as u32)
    }

    fn is_identity(self, input_width: u32, input_height: u32) -> bool {
        self.output_width == input_width
            && self.output_height == input_height
            && (self.a, self.b, self.c, self.d, self.tx, self.ty) == (1, 0, 0, 1, 0, 0)
    }
}

fn copy_transformed(source: &SourceImage, geometry: ExactGeometry) -> Result<Vec<u8>, Error> {
    let width = geometry.output_width;
    let height = geometry.output_height;
    let mut output = vec![0; Raster::expected_len(width, height)?];
    let row_bytes = usize::try_from(u64::from(width) * 4)
        .map_err(|_| Error::new(ErrorKind::ResourceLimit, "image row is not addressable"))?;
    let copy_row = |out_y: usize, row: &mut [u8]| {
        for out_x in 0..width {
            let (input_x, input_y) = geometry.unmap(out_x, out_y as u32);
            let from =
                ((u64::from(input_y) * u64::from(source.width) + u64::from(input_x)) * 4) as usize;
            let to = out_x as usize * 4;
            row[to..to + 4].copy_from_slice(&source.rgba[from..from + 4]);
        }
    };
    if u64::from(width) * u64::from(height) >= PARALLEL_RENDER_PIXELS {
        output
            .par_chunks_exact_mut(row_bytes)
            .enumerate()
            .for_each(|(out_y, row)| copy_row(out_y, row));
    } else {
        output
            .chunks_exact_mut(row_bytes)
            .enumerate()
            .for_each(|(out_y, row)| copy_row(out_y, row));
    }
    Ok(output)
}

/// One recipe compiled by the registry: the ordered processing primitives and the composed
/// geometry that produces the output stage.
pub(crate) struct Compiled {
    pub(crate) operations: Vec<Processing>,
    pub(crate) geometry: ExactGeometry,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) has_pixels: bool,
}

/// Where one final-stage pixel comes from: the source pixel it copies and the replacement that wins there.
struct Resolved {
    rgb: Option<[u8; 3]>,
    source_x: u32,
    source_y: u32,
}

impl Compiled {
    fn resolve(&self, x: u32, y: u32) -> Option<Resolved> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let (source_x, source_y) = self.geometry.unmap(x, y);
        let mut suffix = ExactGeometry::identity(self.width, self.height);
        for operation in self.operations.iter().rev() {
            match operation {
                Processing::ExactGeometry(step) => suffix = step.then(suffix),
                Processing::PointReplace {
                    x: pixel_x,
                    y: pixel_y,
                    rgb,
                } if suffix.map(*pixel_x, *pixel_y) == (x, y) => {
                    return Some(Resolved {
                        rgb: Some(*rgb),
                        source_x,
                        source_y,
                    });
                }
                Processing::PointReplace { .. } => {}
            }
        }
        Some(Resolved {
            rgb: None,
            source_x,
            source_y,
        })
    }
}

fn check_source(source: &SourceImage) -> Result<(), Error> {
    if source.rgba.len() != Raster::expected_len(source.width, source.height)? {
        return Err(Error::new(
            ErrorKind::Validation,
            "source pixel buffer has the wrong length",
        ));
    }
    Ok(())
}

fn source_pixel(source: &SourceImage, x: u32, y: u32) -> [u8; 4] {
    let offset = ((u64::from(y) * u64::from(source.width) + u64::from(x)) * 4) as usize;
    let pixel = &source.rgba[offset..offset + 4];
    [pixel[0], pixel[1], pixel[2], pixel[3]]
}

/// One evaluated pixel of a recipe's output stage, with that stage's dimensions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sample {
    pub width: u32,
    pub height: u32,
    /// `None` when the coordinate lies outside the output stage.
    pub rgba: Option<[u8; 4]>,
}

/// One compiled recipe bound to its source: the stage it produces and O(layers) point queries
/// that never allocate a frame. Compiling once serves any number of sampled pixels.
pub(crate) struct Evaluation<'a> {
    source: &'a SourceImage,
    compiled: Compiled,
}

impl<'a> Evaluation<'a> {
    pub(crate) fn new(
        registry: &ModuleRegistry,
        source: &'a SourceImage,
        recipe: &Recipe,
    ) -> Result<Self, Error> {
        check_source(source)?;
        Ok(Self {
            source,
            compiled: registry.compile(source.width, source.height, recipe)?,
        })
    }

    pub(crate) fn stage(&self) -> Stage {
        Stage {
            width: self.compiled.width,
            height: self.compiled.height,
        }
    }

    /// `None` when the coordinate lies outside the output stage.
    pub(crate) fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        self.compiled.resolve(x, y).map(|resolved| {
            let mut rgba = source_pixel(self.source, resolved.source_x, resolved.source_y);
            if let Some(rgb) = resolved.rgb {
                rgba[..3].copy_from_slice(&rgb);
            }
            rgba
        })
    }
}

/// Evaluate one output pixel without rasterizing; cost is linear in the layer count.
pub fn sample(
    registry: &ModuleRegistry,
    source: &SourceImage,
    recipe: &Recipe,
    x: u32,
    y: u32,
) -> Result<Sample, Error> {
    let evaluation = Evaluation::new(registry, source, recipe)?;
    let stage = evaluation.stage();
    Ok(Sample {
        width: stage.width,
        height: stage.height,
        rgba: evaluation.pixel(x, y),
    })
}

pub fn render(
    registry: &ModuleRegistry,
    source: &SourceImage,
    snapshot_id: SnapshotId,
    recipe: &Recipe,
) -> Result<Raster, Error> {
    check_source(source)?;
    let Compiled {
        operations,
        geometry,
        width,
        height,
        has_pixels,
    } = registry.compile(source.width, source.height, recipe)?;
    let identity = geometry.is_identity(source.width, source.height);
    let mut output = if identity && !has_pixels {
        None
    } else if identity {
        Some(source.rgba.as_ref().to_vec())
    } else {
        Some(copy_transformed(source, geometry)?)
    };

    if has_pixels {
        let pixels = output.as_mut().expect("pixel recipes have writable output");
        let mut suffix = ExactGeometry::identity(width, height);
        let mut replaced = HashSet::new();
        for operation in operations.iter().rev() {
            match operation {
                Processing::ExactGeometry(step) => suffix = step.then(suffix),
                Processing::PointReplace {
                    x: pixel_x,
                    y: pixel_y,
                    rgb,
                } => {
                    let (x, y) = suffix.map(*pixel_x, *pixel_y);
                    if replaced.insert((x, y)) {
                        let offset =
                            ((u64::from(y) * u64::from(width) + u64::from(x)) * 4) as usize;
                        pixels[offset..offset + 3].copy_from_slice(rgb);
                    }
                }
            }
        }
    }

    Ok(Raster {
        width,
        height,
        rgba: output.map_or_else(|| source.rgba.clone(), |pixels| pixels.into()),
        source_fingerprint: source.fingerprint.clone(),
        snapshot_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AssetId, Layer, PIXEL_EFFECT, PixelReplace, Recipe, Snapshot, TRANSFORM_EFFECT, Transform,
    };

    fn registry() -> ModuleRegistry {
        ModuleRegistry::builtin()
    }
    fn source(width: u32, height: u32) -> SourceImage {
        let mut rgba = Vec::new();
        for i in 0..width * height {
            rgba.extend([i as u8, (i + 20) as u8, (i + 40) as u8, 255]);
        }
        SourceImage {
            width,
            height,
            rgba: rgba.into(),
            fingerprint: "sha256:test".into(),
            orientation: 1,
        }
    }
    fn red(raster: &Raster) -> Vec<u8> {
        raster.rgba.chunks_exact(4).map(|p| p[0]).collect()
    }
    fn rendered(source: &SourceImage, layers: Vec<Layer>) -> Raster {
        render(
            &registry(),
            source,
            SnapshotId::new(),
            &Recipe { format: 1, layers },
        )
        .unwrap()
    }

    fn reference(source: &SourceImage, layers: &[Layer]) -> (u32, u32, Vec<u8>) {
        let mut width = source.width;
        let mut height = source.height;
        let mut rgba = source.rgba.as_ref().to_vec();
        for layer in layers {
            match layer.effect_id.as_str() {
                PIXEL_EFFECT => {
                    let pixel: PixelReplace =
                        serde_json::from_value(layer.payload.clone()).unwrap();
                    let offset = ((pixel.y * width + pixel.x) * 4) as usize;
                    rgba[offset..offset + 3].copy_from_slice(&pixel.rgb);
                }
                TRANSFORM_EFFECT => {
                    let transform: Transform =
                        serde_json::from_value(layer.payload.clone()).unwrap();
                    let (next_width, next_height) = match transform {
                        Transform::RotateLeft | Transform::RotateRight => (height, width),
                        Transform::MirrorHorizontal | Transform::FlipVertical => (width, height),
                    };
                    let mut next = vec![0; (next_width * next_height * 4) as usize];
                    for y in 0..height {
                        for x in 0..width {
                            let (next_x, next_y) = match transform {
                                Transform::RotateRight => (height - 1 - y, x),
                                Transform::RotateLeft => (y, width - 1 - x),
                                Transform::MirrorHorizontal => (width - 1 - x, y),
                                Transform::FlipVertical => (x, height - 1 - y),
                            };
                            let from = ((y * width + x) * 4) as usize;
                            let to = ((next_y * next_width + next_x) * 4) as usize;
                            next[to..to + 4].copy_from_slice(&rgba[from..from + 4]);
                        }
                    }
                    width = next_width;
                    height = next_height;
                    rgba = next;
                }
                other => panic!("unexpected test effect {other}"),
            }
        }
        (width, height, rgba)
    }

    #[test]
    fn pixel_layers_are_ordered_exact_and_source_is_immutable() {
        let source = source(3, 2);
        let original = source.clone();
        let a = Layer::pixel(1, 0, [200, 201, 202]);
        let first = rendered(&source, vec![a.clone()]);
        let second = rendered(&source, vec![a, Layer::pixel(1, 0, [9, 8, 7])]);
        assert_eq!(first.pixel(1, 0), Some([200, 201, 202, 255]));
        assert_eq!(second.pixel(1, 0), Some([9, 8, 7, 255]));
        assert_eq!(first.pixel(0, 0), Some([0, 20, 40, 255]));
        assert_eq!(source, original);
        assert!(rendered(&source, vec![]).pixel(1, 0) != second.pixel(1, 0));
    }

    #[test]
    fn exact_transform_coordinate_tables_for_asymmetric_input() {
        let source = source(3, 2);
        assert_eq!(
            red(&rendered(
                &source,
                vec![Layer::transform(Transform::RotateRight)]
            )),
            vec![3, 0, 4, 1, 5, 2]
        );
        assert_eq!(
            red(&rendered(
                &source,
                vec![Layer::transform(Transform::RotateLeft)]
            )),
            vec![2, 5, 1, 4, 0, 3]
        );
        assert_eq!(
            red(&rendered(
                &source,
                vec![Layer::transform(Transform::MirrorHorizontal)]
            )),
            vec![2, 1, 0, 5, 4, 3]
        );
        assert_eq!(
            red(&rendered(
                &source,
                vec![Layer::transform(Transform::FlipVertical)]
            )),
            vec![3, 4, 5, 0, 1, 2]
        );
    }

    #[test]
    fn transform_identities_and_operation_order_hold() {
        let source = source(5, 3);
        for (transform, count) in [
            (Transform::RotateRight, 4),
            (Transform::RotateLeft, 4),
            (Transform::MirrorHorizontal, 2),
            (Transform::FlipVertical, 2),
        ] {
            let layers = (0..count).map(|_| Layer::transform(transform)).collect();
            assert_eq!(rendered(&source, layers).rgba, source.rgba);
        }
        let before = rendered(
            &source,
            vec![
                Layer::pixel(0, 0, [250, 0, 0]),
                Layer::transform(Transform::RotateRight),
            ],
        );
        assert_eq!(before.pixel(2, 0), Some([250, 0, 0, 255]));
        let after = rendered(
            &source,
            vec![
                Layer::transform(Transform::RotateRight),
                Layer::pixel(0, 0, [250, 0, 0]),
            ],
        );
        assert_eq!(after.pixel(0, 0), Some([250, 0, 0, 255]));
        assert_ne!(before.rgba, after.rgba);
    }

    #[test]
    fn compiled_recipes_match_stepwise_evaluation_for_interleaved_operations() {
        let source = source(5, 3);
        let transforms = [
            Transform::RotateLeft,
            Transform::RotateRight,
            Transform::MirrorHorizontal,
            Transform::FlipVertical,
        ];
        for first in transforms {
            for second in transforms {
                for third in transforms {
                    let layers = vec![
                        Layer::pixel(1, 1, [201, 1, 2]),
                        Layer::transform(first),
                        Layer::pixel(0, 0, [3, 202, 4]),
                        Layer::transform(second),
                        Layer::pixel(1, 1, [5, 6, 203]),
                        Layer::transform(third),
                        Layer::pixel(0, 0, [204, 8, 9]),
                    ];
                    let expected = reference(&source, &layers);
                    let actual = rendered(&source, layers);
                    assert_eq!((actual.width, actual.height), (expected.0, expected.1));
                    assert_eq!(actual.rgba.as_ref(), expected.2);
                }
            }
        }
    }

    #[test]
    fn samples_match_rendered_pixels_and_keep_source_alpha() {
        let registry = registry();
        let mut source = source(5, 3);
        let rgba: Vec<u8> = source
            .rgba
            .iter()
            .enumerate()
            .map(|(i, v)| if i % 4 == 3 { (i / 4) as u8 + 100 } else { *v })
            .collect();
        source.rgba = rgba.into();
        let recipe = Recipe {
            format: 1,
            layers: vec![
                Layer::pixel(1, 1, [201, 1, 2]),
                Layer::transform(Transform::RotateLeft),
                Layer::pixel(0, 0, [3, 202, 4]),
                Layer::transform(Transform::MirrorHorizontal),
                Layer::pixel(0, 0, [204, 8, 9]),
                Layer::pixel(0, 0, [205, 10, 11]),
            ],
        };
        let raster = render(&registry, &source, SnapshotId::new(), &recipe).unwrap();
        for y in 0..raster.height {
            for x in 0..raster.width {
                let sampled = sample(&registry, &source, &recipe, x, y).unwrap();
                assert_eq!(
                    (sampled.width, sampled.height),
                    (raster.width, raster.height)
                );
                assert_eq!(sampled.rgba, raster.pixel(x, y), "({x}, {y})");
            }
        }
        assert_eq!(
            sample(&registry, &source, &recipe, raster.width, 0)
                .unwrap()
                .rgba,
            None
        );
        assert_eq!(
            sample(&registry, &source, &recipe, 0, raster.height)
                .unwrap()
                .rgba,
            None
        );
        let invalid = Recipe {
            format: 1,
            layers: vec![Layer::pixel(9, 9, [0, 0, 0])],
        };
        assert!(sample(&registry, &source, &invalid, 0, 0).is_err());
    }

    #[test]
    fn invalid_coordinates_and_buffers_fail_without_panicking() {
        let registry = registry();
        let source = source(3, 2);
        let snapshot = Snapshot::original(AssetId::new());
        assert!(
            render(
                &registry,
                &source,
                snapshot.id.clone(),
                &Recipe {
                    format: 1,
                    layers: vec![Layer::pixel(3, 0, [0, 0, 0])]
                }
            )
            .is_err()
        );
        let malformed = SourceImage {
            rgba: vec![0].into(),
            ..source
        };
        assert!(render(&registry, &malformed, snapshot.id, &Recipe::default()).is_err());
    }
}

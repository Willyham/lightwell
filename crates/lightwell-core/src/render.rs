use crate::{
    EFFECT_FORMAT, Error, ErrorKind, PIXEL_EFFECT, PixelReplace, RECIPE_FORMAT, Recipe, SnapshotId,
    SourceImage, TRANSFORM_EFFECT, Transform,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Geometry {
    input_width: u32,
    input_height: u32,
    output_width: u32,
    output_height: u32,
    a: i64,
    b: i64,
    c: i64,
    d: i64,
    tx: i64,
    ty: i64,
}

impl Geometry {
    fn identity(width: u32, height: u32) -> Self {
        Self {
            input_width: width,
            input_height: height,
            output_width: width,
            output_height: height,
            a: 1,
            b: 0,
            c: 0,
            d: 1,
            tx: 0,
            ty: 0,
        }
    }

    fn transform(transform: Transform, width: u32, height: u32) -> Self {
        match transform {
            Transform::RotateRight => Self {
                input_width: width,
                input_height: height,
                output_width: height,
                output_height: width,
                a: 0,
                b: -1,
                c: 1,
                d: 0,
                tx: i64::from(height) - 1,
                ty: 0,
            },
            Transform::RotateLeft => Self {
                input_width: width,
                input_height: height,
                output_width: height,
                output_height: width,
                a: 0,
                b: 1,
                c: -1,
                d: 0,
                tx: 0,
                ty: i64::from(width) - 1,
            },
            Transform::MirrorHorizontal => Self {
                input_width: width,
                input_height: height,
                output_width: width,
                output_height: height,
                a: -1,
                b: 0,
                c: 0,
                d: 1,
                tx: i64::from(width) - 1,
                ty: 0,
            },
            Transform::FlipVertical => Self {
                input_width: width,
                input_height: height,
                output_width: width,
                output_height: height,
                a: 1,
                b: 0,
                c: 0,
                d: -1,
                tx: 0,
                ty: i64::from(height) - 1,
            },
        }
    }

    /// Compose `self` followed by `next`.
    fn then(self, next: Self) -> Self {
        debug_assert_eq!(self.output_width, next.input_width);
        debug_assert_eq!(self.output_height, next.input_height);
        Self {
            input_width: self.input_width,
            input_height: self.input_height,
            output_width: next.output_width,
            output_height: next.output_height,
            a: next.a * self.a + next.b * self.c,
            b: next.a * self.b + next.b * self.d,
            c: next.c * self.a + next.d * self.c,
            d: next.c * self.b + next.d * self.d,
            tx: next.a * self.tx + next.b * self.ty + next.tx,
            ty: next.c * self.tx + next.d * self.ty + next.ty,
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
        debug_assert!(input_x >= 0 && input_x < i64::from(self.input_width));
        debug_assert!(input_y >= 0 && input_y < i64::from(self.input_height));
        (input_x as u32, input_y as u32)
    }

    fn is_identity(self) -> bool {
        self.input_width == self.output_width
            && self.input_height == self.output_height
            && (self.a, self.b, self.c, self.d, self.tx, self.ty) == (1, 0, 0, 1, 0, 0)
    }
}

#[derive(Clone, Debug)]
enum Operation {
    Pixel(PixelReplace),
    Transform {
        transform: Transform,
        input_width: u32,
        input_height: u32,
    },
}

fn copy_transformed(source: &SourceImage, geometry: Geometry) -> Result<Vec<u8>, Error> {
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

pub fn render(
    source: &SourceImage,
    snapshot_id: SnapshotId,
    recipe: &Recipe,
) -> Result<Raster, Error> {
    if source.rgba.len() != Raster::expected_len(source.width, source.height)? {
        return Err(Error::new(
            ErrorKind::Validation,
            "source pixel buffer has the wrong length",
        ));
    }
    if recipe.format != RECIPE_FORMAT {
        return Err(Error::new(
            ErrorKind::Incompatible,
            format!("unsupported recipe format {}", recipe.format),
        ));
    }

    let mut layer_ids = HashSet::with_capacity(recipe.layers.len());
    let mut operations = Vec::with_capacity(recipe.layers.len());
    let mut geometry = Geometry::identity(source.width, source.height);
    let mut width = source.width;
    let mut height = source.height;
    for layer in &recipe.layers {
        if !layer_ids.insert(&layer.id) {
            return Err(Error::new(
                ErrorKind::Validation,
                "duplicate layer identity",
            ));
        }
        if layer.effect_format != EFFECT_FORMAT {
            return Err(Error::new(
                ErrorKind::Incompatible,
                "unsupported layer format",
            ));
        }
        match layer.effect_id.as_str() {
            PIXEL_EFFECT => {
                let pixel: PixelReplace =
                    serde_json::from_value(layer.payload.clone()).map_err(|e| {
                        Error::new(ErrorKind::Validation, format!("invalid pixel payload: {e}"))
                    })?;
                if pixel.x >= width || pixel.y >= height {
                    return Err(Error::new(
                        ErrorKind::Validation,
                        format!(
                            "pixel ({}, {}) is outside {}x{} input stage",
                            pixel.x, pixel.y, width, height
                        ),
                    ));
                }
                operations.push(Operation::Pixel(pixel));
            }
            TRANSFORM_EFFECT => {
                let transform: Transform =
                    serde_json::from_value(layer.payload.clone()).map_err(|e| {
                        Error::new(
                            ErrorKind::Validation,
                            format!("invalid transform payload: {e}"),
                        )
                    })?;
                let step = Geometry::transform(transform, width, height);
                operations.push(Operation::Transform {
                    transform,
                    input_width: width,
                    input_height: height,
                });
                geometry = geometry.then(step);
                width = step.output_width;
                height = step.output_height;
            }
            other => {
                return Err(Error::new(
                    ErrorKind::Incompatible,
                    format!("unavailable effect {other}"),
                ));
            }
        }
    }

    let has_pixels = operations
        .iter()
        .any(|operation| matches!(operation, Operation::Pixel(_)));
    let mut output = if geometry.is_identity() && !has_pixels {
        None
    } else if geometry.is_identity() {
        Some(source.rgba.as_ref().to_vec())
    } else {
        Some(copy_transformed(source, geometry)?)
    };

    if has_pixels {
        let pixels = output.as_mut().expect("pixel recipes have writable output");
        let mut suffix = Geometry::identity(width, height);
        let mut replaced = HashSet::new();
        for operation in operations.iter().rev() {
            match operation {
                Operation::Transform {
                    transform,
                    input_width,
                    input_height,
                } => {
                    suffix =
                        Geometry::transform(*transform, *input_width, *input_height).then(suffix);
                }
                Operation::Pixel(pixel) => {
                    let (x, y) = suffix.map(pixel.x, pixel.y);
                    if replaced.insert((x, y)) {
                        let offset =
                            ((u64::from(y) * u64::from(width) + u64::from(x)) * 4) as usize;
                        pixels[offset..offset + 3].copy_from_slice(&pixel.rgb);
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
    use crate::{AssetId, Layer, Recipe, Snapshot};

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
        render(source, SnapshotId::new(), &Recipe { format: 1, layers }).unwrap()
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
    fn invalid_coordinates_and_buffers_fail_without_panicking() {
        let source = source(3, 2);
        let snapshot = Snapshot::original(AssetId::new());
        assert!(
            render(
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
        assert!(render(&malformed, snapshot.id, &Recipe::default()).is_err());
    }
}

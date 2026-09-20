use crate::{
    EFFECT_FORMAT, Error, ErrorKind, PIXEL_EFFECT, PixelReplace, Recipe, SnapshotId, SourceImage,
    TRANSFORM_EFFECT, Transform,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
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

    fn from_source(source: &SourceImage, snapshot_id: SnapshotId) -> Result<Self, Error> {
        if source.rgba.len() != Self::expected_len(source.width, source.height)? {
            return Err(Error::new(
                ErrorKind::Validation,
                "source pixel buffer has the wrong length",
            ));
        }
        Ok(Self {
            width: source.width,
            height: source.height,
            rgba: source.rgba.clone(),
            source_fingerprint: source.fingerprint.clone(),
            snapshot_id,
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

pub fn render(
    source: &SourceImage,
    snapshot_id: SnapshotId,
    recipe: &Recipe,
) -> Result<Raster, Error> {
    recipe.validate()?;
    let mut raster = Raster::from_source(source, snapshot_id)?;
    for layer in &recipe.layers {
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
                replace(&mut raster, pixel)?;
            }
            TRANSFORM_EFFECT => {
                let transform: Transform =
                    serde_json::from_value(layer.payload.clone()).map_err(|e| {
                        Error::new(
                            ErrorKind::Validation,
                            format!("invalid transform payload: {e}"),
                        )
                    })?;
                raster = transformed(raster, transform)?;
            }
            other => {
                return Err(Error::new(
                    ErrorKind::Incompatible,
                    format!("unavailable effect {other}"),
                ));
            }
        }
    }
    Ok(raster)
}

fn replace(raster: &mut Raster, pixel: PixelReplace) -> Result<(), Error> {
    if pixel.x >= raster.width || pixel.y >= raster.height {
        return Err(Error::new(
            ErrorKind::Validation,
            format!(
                "pixel ({}, {}) is outside {}x{} input stage",
                pixel.x, pixel.y, raster.width, raster.height
            ),
        ));
    }
    let offset = ((u64::from(pixel.y) * u64::from(raster.width) + u64::from(pixel.x)) * 4) as usize;
    raster.rgba[offset..offset + 3].copy_from_slice(&pixel.rgb);
    Ok(())
}

fn transformed(mut input: Raster, transform: Transform) -> Result<Raster, Error> {
    let (out_width, out_height) = match transform {
        Transform::RotateLeft | Transform::RotateRight => (input.height, input.width),
        Transform::MirrorHorizontal | Transform::FlipVertical => (input.width, input.height),
    };
    let mut output = vec![0; Raster::expected_len(out_width, out_height)?];
    for y in 0..input.height {
        for x in 0..input.width {
            let (out_x, out_y) = match transform {
                Transform::RotateRight => (input.height - 1 - y, x),
                Transform::RotateLeft => (y, input.width - 1 - x),
                Transform::MirrorHorizontal => (input.width - 1 - x, y),
                Transform::FlipVertical => (x, input.height - 1 - y),
            };
            let from = ((u64::from(y) * u64::from(input.width) + u64::from(x)) * 4) as usize;
            let to = ((u64::from(out_y) * u64::from(out_width) + u64::from(out_x)) * 4) as usize;
            output[to..to + 4].copy_from_slice(&input.rgba[from..from + 4]);
        }
    }
    input.width = out_width;
    input.height = out_height;
    input.rgba = output;
    Ok(input)
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
            rgba,
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
            rgba: vec![0],
            ..source
        };
        assert!(render(&malformed, snapshot.id, &Recipe::default()).is_err());
    }
}

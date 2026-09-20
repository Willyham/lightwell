//! The closed host set of processing primitives a module may compile its payloads into.
//! Composition, mapping and rasterizing stay in the host; a module only describes its step.

/// One image stage: the dimensions a layer's payload addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stage {
    pub width: u32,
    pub height: u32,
}

/// An exact integer coordinate mapping with the stage it produces. Several of these compose into
/// one mapping, so a stack of transforms still rasterizes in a single pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactGeometry {
    pub a: i64,
    pub b: i64,
    pub c: i64,
    pub d: i64,
    pub tx: i64,
    pub ty: i64,
    pub output_width: u32,
    pub output_height: u32,
}

/// An interpolating stage boundary: an affine map from output pixel centers back to input
/// coordinates, with the stage it produces.
///
/// Output pixel `(x, y)` has its center at `(x' , y') = (x + 0.5, y + 0.5)` in output continuous
/// coordinates, and `inverse` maps that center to continuous input coordinates
/// `u = m0·x' + m1·y' + m2` and `v = m3·x' + m4·y' + m5` for `inverse = [m0, m1, m2, m3, m4, m5]`.
/// `(u, v)` is a pixel-center coordinate of the input stage, so the input raster is read at index
/// coordinates `(u - 0.5, v - 0.5)`, bilinearly in linear light with indices clamped to its edge.
///
/// The exact layers before a resample rasterize into one bounded intermediate frame, the resample
/// writes the next frame and exact layers after it compose as before.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resample {
    pub inverse: [f64; 6],
    pub output_width: u32,
    pub output_height: u32,
}

/// What the host does with one compiled layer. `Eq` is not derivable because a resample carries
/// f64 coefficients.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Processing {
    ExactGeometry(ExactGeometry),
    /// One input-stage pixel, applied through the geometry that follows it.
    PointReplace {
        x: u32,
        y: u32,
        rgb: [u8; 3],
    },
    Resample(Resample),
}

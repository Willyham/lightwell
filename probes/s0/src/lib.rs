//! Experimental shared image path; not the maintained application service.
mod profile;
use image::{ImageDecoder, ImageReader, Limits};
use std::{
    fs::File,
    io::{Cursor, Read},
    path::Path,
    time::Instant,
};

pub struct Photo {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub decode_ms: f64,
}

// Walk JPEG header segments without decoding or allocating from declared dimensions.
fn header(bytes: &[u8]) -> Result<(u32, u32, u8), String> {
    if !bytes.starts_with(&[0xff, 0xd8]) || !bytes.ends_with(&[0xff, 0xd9]) {
        return Err("invalid-input: missing JPEG SOI/EOI".into());
    }
    let mut i = 2;
    while i + 4 <= bytes.len() {
        if bytes[i] != 0xff {
            return Err("invalid-input: JPEG marker".into());
        }
        while i < bytes.len() && bytes[i] == 0xff {
            i += 1;
        }
        let marker = *bytes.get(i).ok_or("invalid-input: marker")?;
        i += 1;
        if marker == 0xda || marker == 0xd9 {
            break;
        }
        let size = bytes.get(i..i + 2).ok_or("invalid-input: segment length")?;
        let size = u16::from_be_bytes([size[0], size[1]]) as usize;
        if size < 2 || i + size > bytes.len() {
            return Err("invalid-input: segment bounds".into());
        }
        if [0xc0, 0xc1, 0xc2].contains(&marker) {
            if size < 8 || bytes[i + 2] != 8 {
                return Err("unsupported-color: JPEG precision".into());
            }
            let h = u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]) as u32;
            let w = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            return Ok((w, h, bytes[i + 7]));
        }
        i += size;
    }
    Err("unsupported-input: JPEG frame type".into())
}

pub fn open(path: &Path) -> Result<Photo, String> {
    let start = Instant::now();
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| format!("read-error: {}", e.kind()))?
        .take(128 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("read-error: {}", e.kind()))?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("resource-limit: encoded bytes".into());
    }
    let (w, h, components) = header(&bytes)?;
    if w == 0 || h == 0 || w > 16384 || h > 16384 || u64::from(w) * u64::from(h) > 64_000_000 {
        return Err("resource-limit: dimensions".into());
    }
    if ![1, 3].contains(&components) {
        return Err("unsupported-color: only RGB/greyscale".into());
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Jpeg);
    let mut limits = Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| "invalid-input: decoder header")?;
    if let Some(profile) = decoder
        .icc_profile()
        .map_err(|_| "unsupported-profile: unreadable ICC")?
    {
        profile::check(&profile, components)?;
    }
    let orientation = decoder
        .orientation()
        .map_err(|_| "invalid-input: orientation")?;
    let mut decoded =
        image::DynamicImage::from_decoder(decoder).map_err(|_| "invalid-input: decode")?;
    decoded.apply_orientation(orientation);
    let rgba = decoded.into_rgba8();
    Ok(Photo {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
        decode_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/s0")
            .join(name)
    }
    #[test]
    fn orientations_and_preservation() {
        let permutations = [
            [0, 1, 2, 3],
            [1, 0, 3, 2],
            [3, 2, 1, 0],
            [2, 3, 0, 1],
            [0, 2, 1, 3],
            [2, 0, 3, 1],
            [3, 1, 2, 0],
            [1, 3, 0, 2],
        ];
        let colors: [[u8; 3]; 4] = [[220, 35, 45], [35, 190, 65], [40, 70, 220], [235, 195, 30]];
        for orientation in 1..=8 {
            let path = fixture(&format!("orientation-{orientation}.jpg"));
            let original = std::fs::read(&path).unwrap();
            let photo = open(&path).unwrap();
            assert_eq!(
                (photo.width, photo.height),
                if orientation >= 5 {
                    (320, 480)
                } else {
                    (480, 320)
                }
            );
            for (index, (x, y)) in [(1, 1), (3, 1), (1, 3), (3, 3)].iter().enumerate() {
                let offset =
                    (((photo.height * y / 4) * photo.width + photo.width * x / 4) * 4) as usize;
                for (actual, expected) in photo.rgba[offset..offset + 3]
                    .iter()
                    .zip(colors[permutations[orientation - 1][index]])
                {
                    assert!(actual.abs_diff(expected) <= 5);
                }
            }
            assert_eq!(std::fs::read(path).unwrap(), original);
        }
    }
    #[test]
    fn input_contract() {
        for name in ["srgb.jpg", "portrait.jpg", "greyscale.jpg"] {
            assert!(open(&fixture(name)).is_ok(), "{name}");
        }
        for (name, code) in [
            ("invalid.jpg", "invalid-input"),
            ("truncated.jpg", "invalid-input"),
            ("oversized.jpg", "resource-limit"),
            ("cmyk.jpg", "unsupported-color"),
            ("invalid-profile.jpg", "unsupported-profile"),
            ("missing.jpg", "read-error"),
        ] {
            assert!(
                open(&fixture(name)).err().unwrap().starts_with(code),
                "{name}"
            );
        }
    }
}

/// One active decode, one replaceable pending path, one bounded result.
/// Polling is needed only while loading; dropping the viewer never joins a decode on the UI thread.
#[derive(Default)]
pub struct Loader {
    generation: u64,
    active: Option<(u64, std::sync::mpsc::Receiver<Result<Photo, String>>)>,
    pending: Option<std::path::PathBuf>,
}
impl Loader {
    pub fn request(&mut self, path: std::path::PathBuf) {
        self.generation += 1;
        if self.active.is_some() {
            self.pending = Some(path);
        } else {
            self.start(path);
        }
    }
    fn start(&mut self, path: std::path::PathBuf) {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let _ = sender.send(open(&path));
        });
        self.active = Some((self.generation, receiver));
    }
    pub fn loading(&self) -> bool {
        self.active.is_some()
    }
    pub fn poll(&mut self) -> Option<Result<Photo, String>> {
        let (generation, receiver) = self.active.as_ref()?;
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(_) => Err("internal: decode worker disconnected".into()),
        };
        let current = *generation == self.generation;
        self.active = None;
        if let Some(path) = self.pending.take() {
            self.start(path);
        }
        if current { Some(result) } else { None }
    }
}

#[cfg(test)]
mod loader_tests {
    use super::*;
    #[test]
    fn newest_request_wins_with_bounded_pending_work() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/s0");
        let mut loader = Loader::default();
        loader.request(base.join("orientation-1.jpg"));
        loader.request(base.join("invalid.jpg"));
        loader.request(base.join("orientation-6.jpg"));
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(result) = loader.poll() {
                let photo =
                    result.expect("latest valid request supersedes pending invalid request");
                assert_eq!((photo.width, photo.height), (320, 480));
                assert!(!loader.loading());
                break;
            }
            assert!(Instant::now() < deadline, "bounded loader did not complete");
            std::thread::yield_now();
        }
    }
}

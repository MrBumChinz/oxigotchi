//! Face pack system — runtime-loadable PNG face bitmaps.
//!
//! Users drop 120x66 PNG files into /home/pi/face_packs/<name>/ and the
//! daemon converts them to 1-bit .raw bitmaps in a background tick.
//! Missing faces fall back to the built-in bitmaps compiled into the binary.

use crate::personality::Face;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Root directory for user face packs. User-writable via SCP.
pub const FACE_PACK_DIR: &str = "/home/pi/face_packs";

/// Cache subdirectory — managed by daemon, users don't touch.
pub const FACE_PACK_CACHE: &str = "/home/pi/face_packs/.cache";

/// Expected size of a face bitmap: 120x66 / 8 bits = 990 bytes.
pub const RAW_FACE_SIZE: usize = 990;

/// Maximum number of packs to discover (sanity cap).
pub const MAX_PACKS: usize = 32;

/// Mapping of lowercase filename (without extension) to Face enum variant.
/// Must be kept in sync with the Face enum in personality/mod.rs.
pub const FACE_NAMES: &[(&str, Face)] = &[
    ("angry", Face::Angry),
    ("ao_crashed", Face::AoCrashed),
    ("awake", Face::Awake),
    ("battery_critical", Face::BatteryCritical),
    ("battery_low", Face::BatteryLow),
    ("bored", Face::Bored),
    ("broken", Face::Broken),
    ("cool", Face::Cool),
    ("debug", Face::Debug),
    ("demotivated", Face::Demotivated),
    ("excited", Face::Excited),
    ("friend", Face::Friend),
    ("fw_crash", Face::FwCrash),
    ("grateful", Face::Grateful),
    ("grazing", Face::Grazing),
    ("happy", Face::Happy),
    ("intense", Face::Intense),
    ("lonely", Face::Lonely),
    ("motivated", Face::Motivated),
    ("raging", Face::Raging),
    ("sad", Face::Sad),
    ("shutdown", Face::Shutdown),
    ("sleep", Face::Sleep),
    ("smart", Face::Smart),
    ("upload", Face::Upload),
    ("wifi_down", Face::WifiDown),
];

/// Errors from the face pack subsystem.
#[derive(Debug)]
pub enum FacePackError {
    /// I/O error reading or writing files.
    Io(std::io::Error),
    /// PNG decode error.
    Decode(String),
    /// PNG has wrong dimensions (must be exactly 120x66).
    WrongSize { width: u32, height: u32 },
    /// Unsupported color type (palette, 16-bit-per-channel).
    UnsupportedFormat(String),
    /// Raw file is not exactly 990 bytes.
    BadRawSize(usize),
    /// Pack name contains invalid characters or is reserved.
    InvalidPackName(String),
    /// Unknown face filename.
    UnknownFaceName(String),
    /// No such pack directory.
    PackNotFound(String),
}

impl std::fmt::Display for FacePackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FacePackError::Io(e) => write!(f, "I/O error: {e}"),
            FacePackError::Decode(e) => write!(f, "PNG decode: {e}"),
            FacePackError::WrongSize { width, height } => {
                write!(f, "PNG is {width}x{height}, expected 120x66")
            }
            FacePackError::UnsupportedFormat(s) => write!(f, "unsupported format: {s}"),
            FacePackError::BadRawSize(n) => write!(f, "raw file is {n} bytes, expected 990"),
            FacePackError::InvalidPackName(n) => write!(f, "invalid pack name: {n}"),
            FacePackError::UnknownFaceName(n) => write!(f, "unknown face name: {n}"),
            FacePackError::PackNotFound(n) => write!(f, "pack not found: {n}"),
        }
    }
}

impl From<std::io::Error> for FacePackError {
    fn from(e: std::io::Error) -> Self {
        FacePackError::Io(e)
    }
}

/// Parse a filename stem (without extension) to a Face variant.
/// Case-insensitive. Returns None if the name doesn't match any known face.
pub fn face_name_from_filename(stem: &str) -> Option<Face> {
    let lower = stem.to_lowercase();
    FACE_NAMES
        .iter()
        .find(|(name, _)| *name == lower.as_str())
        .map(|(_, face)| *face)
}

/// A loaded face pack. The map holds decoded 990-byte bitmaps in RAM.
#[derive(Debug, Clone, Default)]
pub struct FacePack {
    pub name: String,
    pub(crate) map: HashMap<Face, Vec<u8>>,
}

impl FacePack {
    /// Empty pack — all faces fall through to built-ins. Used as the "default".
    pub fn empty() -> Self {
        Self {
            name: "default".to_string(),
            map: HashMap::new(),
        }
    }

    /// Number of face bitmaps in this pack (not counting fallbacks).
    pub fn face_count(&self) -> usize {
        self.map.len()
    }
}

/// Decode a PNG file at `path` and convert to a 120x66 1-bit packed bitmap.
///
/// Returns Err if the PNG is not exactly 120x66, has an unsupported format,
/// or cannot be decoded.
///
/// Conversion:
/// - L8 → use luma value directly
/// - LA8 → use luma, ignore alpha
/// - RGB8 → convert to luma via 0.299*R + 0.587*G + 0.114*B
/// - RGBA8 → convert to luma, ignore alpha
/// - Other formats → rejected
///
/// Threshold: pixel < 128 → black (bit=1), else white (bit=0).
pub fn png_to_raw(path: &Path) -> Result<Vec<u8>, FacePackError> {
    let file = std::fs::File::open(path)?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder
        .read_info()
        .map_err(|e| FacePackError::Decode(e.to_string()))?;

    let info = reader.info();
    if info.width != 120 || info.height != 66 {
        return Err(FacePackError::WrongSize {
            width: info.width,
            height: info.height,
        });
    }

    let color_type = info.color_type;
    let bit_depth = info.bit_depth;

    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut buf)
        .map_err(|e| FacePackError::Decode(e.to_string()))?;
    let data = &buf[..frame.buffer_size()];

    let mut luma = vec![0u8; 120 * 66];

    match (color_type, bit_depth) {
        (png::ColorType::Grayscale, png::BitDepth::Eight) => {
            luma.copy_from_slice(&data[..120 * 66]);
        }
        (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => {
            for i in 0..(120 * 66) {
                luma[i] = data[i * 2];
            }
        }
        (png::ColorType::Rgb, png::BitDepth::Eight) => {
            for i in 0..(120 * 66) {
                let r = data[i * 3] as u32;
                let g = data[i * 3 + 1] as u32;
                let b = data[i * 3 + 2] as u32;
                luma[i] = ((299 * r + 587 * g + 114 * b) / 1000) as u8;
            }
        }
        (png::ColorType::Rgba, png::BitDepth::Eight) => {
            for i in 0..(120 * 66) {
                let r = data[i * 4] as u32;
                let g = data[i * 4 + 1] as u32;
                let b = data[i * 4 + 2] as u32;
                luma[i] = ((299 * r + 587 * g + 114 * b) / 1000) as u8;
            }
        }
        _ => {
            return Err(FacePackError::UnsupportedFormat(format!(
                "{:?} @ {:?}",
                color_type, bit_depth
            )));
        }
    }

    // Pack to 1-bit MSB-first, row-major. 120 cols / 8 = 15 bytes per row.
    let mut raw = vec![0u8; RAW_FACE_SIZE];
    for y in 0..66 {
        for x in 0..120 {
            let pixel = luma[y * 120 + x];
            if pixel < 128 {
                let byte_idx = y * 15 + x / 8;
                let bit_idx = 7 - (x % 8);
                raw[byte_idx] |= 1 << bit_idx;
            }
        }
    }

    Ok(raw)
}

/// Atomically write `bytes` to `path`.
///
/// Sequence:
/// 1. Write to `<path>.tmp` (truncating any existing temp)
/// 2. Sync to disk (important on SD cards)
/// 3. Rename to final path
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), FacePackError> {
    use std::io::Write;

    let tmp_path: PathBuf = {
        let mut p = path.to_path_buf();
        let mut name = p
            .file_name()
            .map(|s| s.to_os_string())
            .unwrap_or_default();
        name.push(".tmp");
        p.set_file_name(name);
        p
    };

    {
        let mut f = std::fs::File::create(&tmp_path)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }

    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Validate a pack name: 1-32 chars, ASCII alphanumeric or `_` `-` only,
/// not reserved (`default`, anything starting with `.`).
pub fn validate_pack_name(name: &str) -> Result<(), FacePackError> {
    if name.is_empty() || name.len() > 32 {
        return Err(FacePackError::InvalidPackName(name.to_string()));
    }
    if name == "default" {
        return Err(FacePackError::InvalidPackName(format!("{name} (reserved)")));
    }
    if name.starts_with('.') {
        return Err(FacePackError::InvalidPackName(format!("{name} (hidden)")));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(FacePackError::InvalidPackName(name.to_string()));
    }
    Ok(())
}

/// Discover all valid pack directories under `root`.
/// Returns pack names (not full paths). Caps at MAX_PACKS.
pub fn discover_packs(root: &Path) -> Result<Vec<String>, FacePackError> {
    let mut packs = Vec::new();
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(packs),
        Err(e) => return Err(FacePackError::Io(e)),
    };

    for entry in entries.flatten() {
        if packs.len() >= MAX_PACKS {
            log::warn!("face_pack: hit MAX_PACKS limit ({MAX_PACKS}), skipping rest");
            break;
        }
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !file_type.is_dir() {
            continue;
        }
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };
        if validate_pack_name(&name).is_err() {
            continue;
        }
        packs.push(name);
    }
    Ok(packs)
}

/// Find one PNG in `pack_dir` that needs conversion (no cached .raw or stale mtime).
/// Returns (png_path, raw_path) for the first stale file found, or None.
/// Unknown face filenames are silently skipped (logged).
pub fn find_stale_png(
    pack_dir: &Path,
    cache_dir: &Path,
) -> Result<Option<(PathBuf, PathBuf)>, FacePackError> {
    let entries = match std::fs::read_dir(pack_dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(FacePackError::Io(e)),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let extension = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
        if extension.as_deref() != Some("png") {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };
        if face_name_from_filename(stem).is_none() {
            log::warn!(
                "face_pack: unknown face name '{}' in pack — valid names: {}",
                stem,
                FACE_NAMES.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
            );
            continue;
        }

        let raw_path = cache_dir.join(format!("{}.raw", stem.to_lowercase()));
        let png_mtime = entry.metadata()?.modified()?;
        let raw_mtime = std::fs::metadata(&raw_path).and_then(|m| m.modified()).ok();

        let is_stale = match raw_mtime {
            None => true,
            Some(raw_time) => png_mtime > raw_time,
        };

        if is_stale {
            return Ok(Some((path, raw_path)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_face_names_covers_all_face_variants() {
        for face in Face::all() {
            assert!(
                FACE_NAMES.iter().any(|(_, f)| *f == *face),
                "Face::{:?} has no FACE_NAMES entry",
                face
            );
        }
    }

    #[test]
    fn test_face_names_length_matches_enum() {
        assert_eq!(FACE_NAMES.len(), Face::all().len());
    }

    #[test]
    fn test_face_names_no_duplicates() {
        let mut names: Vec<&str> = FACE_NAMES.iter().map(|(n, _)| *n).collect();
        names.sort();
        let len_before = names.len();
        names.dedup();
        assert_eq!(names.len(), len_before, "FACE_NAMES has duplicate entries");
    }

    #[test]
    fn test_face_name_from_filename_known() {
        assert_eq!(face_name_from_filename("cool"), Some(Face::Cool));
        assert_eq!(face_name_from_filename("battery_low"), Some(Face::BatteryLow));
    }

    #[test]
    fn test_face_name_from_filename_case_insensitive() {
        assert_eq!(face_name_from_filename("Cool"), Some(Face::Cool));
        assert_eq!(face_name_from_filename("ANGRY"), Some(Face::Angry));
    }

    #[test]
    fn test_face_name_from_filename_unknown() {
        assert_eq!(face_name_from_filename("unknown"), None);
        assert_eq!(face_name_from_filename(""), None);
    }

    #[test]
    fn test_facepack_empty() {
        let pack = FacePack::empty();
        assert_eq!(pack.name, "default");
        assert_eq!(pack.face_count(), 0);
    }

    #[test]
    fn test_raw_face_size_constant() {
        assert_eq!(RAW_FACE_SIZE, 990);
        assert_eq!((120 * 66) / 8, 990);
    }

    #[test]
    fn test_png_to_raw_valid_120x66_grayscale() {
        let png_bytes = encode_test_png_gray(120, 66, |_x, y| {
            if y < 8 { 0 } else { 255 }
        });

        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &png_bytes).unwrap();

        let raw = png_to_raw(tmp.path()).unwrap();
        assert_eq!(raw.len(), 990);
        // First 8 rows: 8 * 15 = 120 bytes of 0xFF
        assert!(raw[..120].iter().all(|&b| b == 0xFF), "first 8 rows should be black");
        assert!(raw[120..].iter().all(|&b| b == 0x00), "remaining rows should be white");
    }

    #[test]
    fn test_png_to_raw_rejects_wrong_size() {
        let png_bytes = encode_test_png_gray(100, 50, |_, _| 0);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &png_bytes).unwrap();

        let err = png_to_raw(tmp.path()).unwrap_err();
        match err {
            FacePackError::WrongSize { width: 100, height: 50 } => {}
            _ => panic!("expected WrongSize, got {:?}", err),
        }
    }

    #[test]
    fn test_png_to_raw_rejects_garbage() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"not a png").unwrap();

        let err = png_to_raw(tmp.path()).unwrap_err();
        match err {
            FacePackError::Decode(_) => {}
            _ => panic!("expected Decode error, got {:?}", err),
        }
    }

    #[test]
    fn test_write_atomic_creates_file() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let target = tmp_dir.path().join("test.raw");
        write_atomic(&target, b"hello world").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"hello world");
        assert!(!tmp_dir.path().join("test.raw.tmp").exists());
    }

    #[test]
    fn test_write_atomic_overwrites() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let target = tmp_dir.path().join("test.raw");
        std::fs::write(&target, b"old").unwrap();
        write_atomic(&target, b"new").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
    }

    #[test]
    fn test_validate_pack_name_valid() {
        assert!(validate_pack_name("my_pack").is_ok());
        assert!(validate_pack_name("MyPack").is_ok());
        assert!(validate_pack_name("pack-1").is_ok());
        assert!(validate_pack_name("a").is_ok());
        assert!(validate_pack_name(&"a".repeat(32)).is_ok());
    }

    #[test]
    fn test_validate_pack_name_invalid() {
        assert!(validate_pack_name("").is_err());
        assert!(validate_pack_name(".hidden").is_err());
        assert!(validate_pack_name(".cache").is_err());
        assert!(validate_pack_name("default").is_err());
        assert!(validate_pack_name("has space").is_err());
        assert!(validate_pack_name("has/slash").is_err());
        assert!(validate_pack_name("has..dots").is_err());
        assert!(validate_pack_name(&"a".repeat(33)).is_err());
        assert!(validate_pack_name("emoji🎉").is_err());
    }

    #[test]
    fn test_discover_packs_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let packs = discover_packs(tmp.path()).unwrap();
        assert!(packs.is_empty());
    }

    #[test]
    fn test_discover_packs_finds_valid_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("pack_a")).unwrap();
        std::fs::create_dir(tmp.path().join("pack_b")).unwrap();
        std::fs::create_dir(tmp.path().join(".cache")).unwrap();
        std::fs::create_dir(tmp.path().join("bad name")).unwrap();
        std::fs::write(tmp.path().join("not_a_dir.txt"), b"hi").unwrap();

        let mut packs = discover_packs(tmp.path()).unwrap();
        packs.sort();
        assert_eq!(packs, vec!["pack_a".to_string(), "pack_b".to_string()]);
    }

    #[test]
    fn test_find_stale_png_no_cache_means_stale() {
        let tmp = tempfile::tempdir().unwrap();
        let pack_dir = tmp.path().join("mypack");
        std::fs::create_dir(&pack_dir).unwrap();
        let png = pack_dir.join("cool.png");
        std::fs::write(&png, b"fake png").unwrap();

        let cache_dir = tmp.path().join(".cache").join("mypack");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let result = find_stale_png(&pack_dir, &cache_dir).unwrap();
        assert!(result.is_some());
        let (stale_png, raw_path) = result.unwrap();
        assert_eq!(stale_png, png);
        assert_eq!(raw_path, cache_dir.join("cool.raw"));
    }

    #[test]
    fn test_find_stale_png_fresh_cache_means_not_stale() {
        let tmp = tempfile::tempdir().unwrap();
        let pack_dir = tmp.path().join("mypack");
        std::fs::create_dir(&pack_dir).unwrap();
        let png = pack_dir.join("cool.png");
        std::fs::write(&png, b"fake png").unwrap();

        let cache_dir = tmp.path().join(".cache").join("mypack");
        std::fs::create_dir_all(&cache_dir).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(cache_dir.join("cool.raw"), vec![0u8; 990]).unwrap();

        let result = find_stale_png(&pack_dir, &cache_dir).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_stale_png_ignores_unknown_names() {
        let tmp = tempfile::tempdir().unwrap();
        let pack_dir = tmp.path().join("mypack");
        std::fs::create_dir(&pack_dir).unwrap();
        std::fs::write(pack_dir.join("unknown_face.png"), b"fake").unwrap();

        let cache_dir = tmp.path().join(".cache").join("mypack");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let result = find_stale_png(&pack_dir, &cache_dir).unwrap();
        assert!(result.is_none());
    }

    fn encode_test_png_gray(w: u32, h: u32, mut pixel: impl FnMut(u32, u32) -> u8) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buf, w, h);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            let mut data = Vec::with_capacity((w * h) as usize);
            for y in 0..h {
                for x in 0..w {
                    data.push(pixel(x, y));
                }
            }
            writer.write_image_data(&data).unwrap();
        }
        buf
    }
}

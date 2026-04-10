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
}

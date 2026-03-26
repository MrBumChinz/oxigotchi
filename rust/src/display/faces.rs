//! Bull face bitmap sprites for the e-ink display.
//!
//! Each face is a 120x66 1-bit packed bitmap (MSB first, row-major).
//! Bit 1 = black pixel, bit 0 = white pixel.
//! 15 bytes per row, 66 rows = 990 bytes per face.
//!
//! Converted from the bull PNG files at faces/eink/.

use crate::personality::Face;

/// Width of each bull face sprite in pixels.
pub const FACE_WIDTH: u32 = 120;
/// Height of each bull face sprite in pixels.
pub const FACE_HEIGHT: u32 = 66;
/// Bytes per row in the face bitmap (120 / 8 = 15).
pub const FACE_STRIDE: usize = 15;

// Embed all 28 face bitmaps at compile time.
static ANGRY: &[u8] = include_bytes!("../../faces/angry.raw");
static AO_CRASHED: &[u8] = include_bytes!("../../faces/ao_crashed.raw");
static AWAKE: &[u8] = include_bytes!("../../faces/awake.raw");
static BATTERY_CRITICAL: &[u8] = include_bytes!("../../faces/battery_critical.raw");
static BATTERY_LOW: &[u8] = include_bytes!("../../faces/battery_low.raw");
static BORED: &[u8] = include_bytes!("../../faces/bored.raw");
static BROKEN: &[u8] = include_bytes!("../../faces/broken.raw");
static COOL: &[u8] = include_bytes!("../../faces/cool.raw");
static DEBUG: &[u8] = include_bytes!("../../faces/debug.raw");
static DEMOTIVATED: &[u8] = include_bytes!("../../faces/demotivated.raw");
static EXCITED: &[u8] = include_bytes!("../../faces/excited.raw");
static FRIEND: &[u8] = include_bytes!("../../faces/friend.raw");
static FW_CRASH: &[u8] = include_bytes!("../../faces/fw_crash.raw");
static GRATEFUL: &[u8] = include_bytes!("../../faces/grateful.raw");
static HAPPY: &[u8] = include_bytes!("../../faces/happy.raw");
static INTENSE: &[u8] = include_bytes!("../../faces/intense.raw");
static LONELY: &[u8] = include_bytes!("../../faces/lonely.raw");
static LOOK_L: &[u8] = include_bytes!("../../faces/look_l.raw");
static LOOK_L_HAPPY: &[u8] = include_bytes!("../../faces/look_l_happy.raw");
static LOOK_R: &[u8] = include_bytes!("../../faces/look_r.raw");
static LOOK_R_HAPPY: &[u8] = include_bytes!("../../faces/look_r_happy.raw");
static MOTIVATED: &[u8] = include_bytes!("../../faces/motivated.raw");
static SAD: &[u8] = include_bytes!("../../faces/sad.raw");
static SHUTDOWN: &[u8] = include_bytes!("../../faces/shutdown.raw");
static SLEEP: &[u8] = include_bytes!("../../faces/sleep.raw");
static SMART: &[u8] = include_bytes!("../../faces/smart.raw");
static UPLOAD: &[u8] = include_bytes!("../../faces/upload.raw");
static WIFI_DOWN: &[u8] = include_bytes!("../../faces/wifi_down.raw");
static RAGING: &[u8] = include_bytes!("../../faces/raging.raw");
static GRAZING: &[u8] = include_bytes!("../../faces/grazing.raw");

/// Get the raw bitmap data for a face. Returns the 990-byte packed bitmap.
pub fn bitmap_for_face(face: &Face) -> &'static [u8] {
    match face {
        Face::Awake => AWAKE,
        Face::Happy => HAPPY,
        Face::Excited => EXCITED,
        Face::Bored => BORED,
        Face::Sad => SAD,
        Face::Demotivated => DEMOTIVATED,
        Face::BatteryCritical => BATTERY_CRITICAL,
        Face::BatteryLow => BATTERY_LOW,
        Face::Shutdown => SHUTDOWN,
        Face::WifiDown => WIFI_DOWN,
        Face::FwCrash => FW_CRASH,
        Face::AoCrashed => AO_CRASHED,
        Face::Broken => BROKEN,
        Face::Sleep => SLEEP,
        Face::Intense => INTENSE,
        Face::Cool => COOL,
        Face::Angry => ANGRY,
        Face::Friend => FRIEND,
        Face::Debug => DEBUG,
        Face::Upload => UPLOAD,
        Face::Lonely => LONELY,
        Face::Grateful => GRATEFUL,
        Face::Motivated => MOTIVATED,
        Face::Smart => SMART,
        Face::Raging => RAGING,
        Face::Grazing => GRAZING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_faces_have_correct_size() {
        for face in Face::all() {
            let data = bitmap_for_face(&face);
            assert_eq!(
                data.len(),
                FACE_STRIDE * FACE_HEIGHT as usize,
                "face {:?} should be {} bytes",
                face,
                FACE_STRIDE * FACE_HEIGHT as usize
            );
        }
    }

    #[test]
    fn test_awake_face_has_pixels() {
        let data = bitmap_for_face(&Face::Awake);
        let black_pixels: u32 = data.iter().map(|b| b.count_ones()).sum();
        assert!(
            black_pixels > 100,
            "awake face should have significant black pixels"
        );
    }

    #[test]
    fn test_faces_are_different() {
        let awake = bitmap_for_face(&Face::Awake);
        let sleep = bitmap_for_face(&Face::Sleep);
        assert_ne!(
            awake, sleep,
            "different faces should have different bitmaps"
        );
    }
}

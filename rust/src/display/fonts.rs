//! Font presets matching the Python pwnagotchi display.
//!
//! Python uses: Small (9pt), Medium (10pt), Bold (10pt), Huge (35pt).
//! We use ProFont bitmap fonts at the closest available sizes.

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use profont::{
    PROFONT_7_POINT, PROFONT_9_POINT, PROFONT_10_POINT, PROFONT_12_POINT, PROFONT_24_POINT,
};

/// Small font for labels, indicators, IP display (Python: 9pt)
pub fn small() -> MonoTextStyle<'static, BinaryColor> {
    MonoTextStyle::new(&PROFONT_9_POINT, BinaryColor::On)
}

/// Medium font for channel, APS, status text (Python: 10pt)
pub fn medium() -> MonoTextStyle<'static, BinaryColor> {
    MonoTextStyle::new(&PROFONT_10_POINT, BinaryColor::On)
}

/// Bold font for name, mode, handshake labels (Python: 10pt bold)
/// ProFont doesn't have bold — use 12pt for visual weight
pub fn bold() -> MonoTextStyle<'static, BinaryColor> {
    MonoTextStyle::new(&PROFONT_12_POINT, BinaryColor::On)
}

/// Large font for face kaomoji (Python: 35pt huge)
/// ProFont maxes at 24pt — closest we can get for ASCII faces
pub fn face() -> MonoTextStyle<'static, BinaryColor> {
    MonoTextStyle::new(&PROFONT_24_POINT, BinaryColor::On)
}

/// Tiny font for dense indicators (7pt)
pub fn tiny() -> MonoTextStyle<'static, BinaryColor> {
    MonoTextStyle::new(&PROFONT_7_POINT, BinaryColor::On)
}

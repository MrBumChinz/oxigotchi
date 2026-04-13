// ---------------------------------------------------------------------------
// Shared 802.11 beacon / probe-response parser
// ---------------------------------------------------------------------------
//
// This module provides a portable, cfg-free parser for radiotap-encapsulated
// 802.11 management frames (beacons and probe responses).  It is used by
// `wifi::mod` for live frame processing and will also serve the SSID resolver
// (pcapng replay) introduced in a later task.
//
// Frame layout expected on input:
//   [radiotap header (variable)] [802.11 management header (24 B)] [fixed params (12 B)] [IEs...]

// ---------------------------------------------------------------------------
// 802.11 frame control constants
// ---------------------------------------------------------------------------

/// Frame control type field: Management frame (bits 3:2 = 00).
const TYPE_MGMT: u8 = 0x00;
/// Frame control subtype: Beacon (bits 7:4 = 1000).
const SUBTYPE_BEACON: u8 = 0x80;
/// Frame control subtype: Probe Response (bits 7:4 = 0101).
const SUBTYPE_PROBE_RESP: u8 = 0x50;

/// Tagged parameter ID for SSID (IE 0).
const TAG_SSID: u8 = 0x00;
/// Tagged parameter ID for DS Parameter Set / channel (IE 3).
const TAG_DS_PARAM: u8 = 0x03;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Parsed information extracted from a beacon or probe-response frame.
#[derive(Debug, Clone)]
pub struct BeaconInfo {
    /// BSSID (address 3 from the 802.11 header).
    pub bssid: [u8; 6],
    /// SSID string (may be empty for hidden networks).
    pub ssid: String,
    /// Operating channel reported in the DS Parameter Set IE.
    pub channel: u8,
    /// dBm RSSI from the radiotap header, or `i16::MIN` if not present.
    pub rssi: i16,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a radiotap-encapsulated 802.11 beacon or probe-response frame.
///
/// Returns `None` if the frame is malformed, too short, or is not a beacon /
/// probe-response.
pub fn parse_beacon(raw: &[u8]) -> Option<BeaconInfo> {
    // Need at least 4 bytes to read radiotap header length field.
    if raw.len() < 4 {
        return None;
    }

    // Radiotap header length is at bytes 2-3 (little-endian u16).
    let rt_len = u16::from_le_bytes([raw[2], raw[3]]) as usize;
    if raw.len() < rt_len {
        return None;
    }

    // Try to extract dBm RSSI from radiotap header.
    let rssi = extract_radiotap_rssi(raw, rt_len)
        .map(|v| v as i16)
        .unwrap_or(i16::MIN);

    let dot11 = &raw[rt_len..];

    // Need at least 24 bytes for the 802.11 management header.
    if dot11.len() < 24 {
        return None;
    }

    let frame_control = dot11[0];
    let frame_type = frame_control & 0x0C; // bits 3:2
    let frame_subtype = frame_control & 0xF0; // bits 7:4

    if frame_type != TYPE_MGMT {
        return None;
    }
    if frame_subtype != SUBTYPE_BEACON && frame_subtype != SUBTYPE_PROBE_RESP {
        return None;
    }

    // BSSID is addr3: bytes 16..22 of the 802.11 header.
    let mut bssid = [0u8; 6];
    bssid.copy_from_slice(&dot11[16..22]);

    // Tagged parameters start at offset 36 (24-byte header + 12-byte fixed params).
    let tagged_start = 36;
    if dot11.len() < tagged_start {
        return None;
    }

    let mut ssid = String::new();
    let mut channel: u8 = 0;

    let mut pos = tagged_start;
    while pos + 2 <= dot11.len() {
        let tag_id = dot11[pos];
        let tag_len = dot11[pos + 1] as usize;
        pos += 2;

        if pos + tag_len > dot11.len() {
            break; // malformed IE, stop gracefully
        }

        match tag_id {
            TAG_SSID => {
                ssid = String::from_utf8_lossy(&dot11[pos..pos + tag_len]).to_string();
            }
            TAG_DS_PARAM => {
                if tag_len >= 1 {
                    channel = dot11[pos];
                }
            }
            _ => {}
        }

        pos += tag_len;
    }

    Some(BeaconInfo {
        bssid,
        ssid,
        channel,
        rssi,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extract dBm antenna signal from a radiotap header.
///
/// Handles extended present bitmasks and the standard field ordering
/// for bits 0-5.  Returns `None` if bit 5 (dBm Antenna Signal) is not set
/// or the header is too short.
fn extract_radiotap_rssi(raw: &[u8], rt_len: usize) -> Option<i8> {
    if rt_len < 8 || raw.len() < 8 {
        return None;
    }

    let present = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);

    // Bit 5 = dBm Antenna Signal — bail early if not set.
    if present & (1 << 5) == 0 {
        return None;
    }

    // Walk field offsets: start after the fixed 8-byte header.
    let mut offset: usize = 8;

    // Skip any extended present bitmask words (bit 31 of each word = more follow).
    let mut extra_present = present;
    while extra_present & (1 << 31) != 0 {
        if offset + 4 > rt_len {
            return None;
        }
        extra_present = u32::from_le_bytes([
            raw[offset],
            raw[offset + 1],
            raw[offset + 2],
            raw[offset + 3],
        ]);
        offset += 4;
    }

    // bit 0: TSFT (8 bytes, aligned to 8)
    if present & (1 << 0) != 0 {
        offset = (offset + 7) & !7;
        offset += 8;
    }
    // bit 1: Flags (1 byte)
    if present & (1 << 1) != 0 {
        offset += 1;
    }
    // bit 2: Rate (1 byte)
    if present & (1 << 2) != 0 {
        offset += 1;
    }
    // bit 3: Channel (4 bytes, aligned to 2)
    if present & (1 << 3) != 0 {
        offset = (offset + 1) & !1;
        offset += 4;
    }
    // bit 4: FHSS (2 bytes)
    if present & (1 << 4) != 0 {
        offset += 2;
    }
    // bit 5: dBm Antenna Signal (1 byte) — this is what we want.
    if offset < rt_len {
        Some(raw[offset] as i8)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Frame builder helper
    // ------------------------------------------------------------------

    /// Build a synthetic beacon or probe-response frame with a radiotap
    /// header that encodes `rssi` in the dBm Antenna Signal field.
    ///
    /// Radiotap layout (16 bytes total):
    ///   [version=0][pad=0][length=16 LE][present=0x2E LE]
    ///   [flags=0][rate=2][channel_freq LE u16][channel_flags=0xA0 LE u16][rssi u8][pad=0]
    ///
    /// present = (1<<1)|(1<<2)|(1<<3)|(1<<5) = 0x2E
    ///   bit1=Flags, bit2=Rate, bit3=Channel, bit5=dBm Antenna Signal
    fn make_beacon(bssid: [u8; 6], ssid: &str, channel: u8, rssi: i8, fc_byte0: u8) -> Vec<u8> {
        let mut f = Vec::new();

        // Radiotap header (16 bytes)
        f.push(0x00); // version
        f.push(0x00); // pad
        f.extend_from_slice(&16u16.to_le_bytes()); // length
        f.extend_from_slice(&0x0000_002Eu32.to_le_bytes()); // present
        f.push(0x00); // Flags
        f.push(0x02); // Rate
        // Channel frequency (2 bytes) — simple approximation
        let freq: u16 = if channel >= 1 && channel <= 13 {
            2412 + (channel as u16 - 1) * 5
        } else {
            2437
        };
        f.extend_from_slice(&freq.to_le_bytes());
        f.extend_from_slice(&0x00A0u16.to_le_bytes()); // channel flags
        f.push(rssi as u8); // dBm Antenna Signal
        f.push(0x00); // pad to 16

        // 802.11 management header (24 bytes)
        f.push(fc_byte0); // frame control byte 0
        f.push(0x00); // frame control byte 1
        f.extend_from_slice(&[0x00, 0x00]); // duration
        f.extend_from_slice(&[0xFF; 6]); // DA (broadcast)
        f.extend_from_slice(&bssid); // SA
        f.extend_from_slice(&bssid); // BSSID (addr3)
        f.extend_from_slice(&[0x00, 0x00]); // sequence control

        // Fixed parameters (12 bytes)
        f.extend_from_slice(&[0x00; 8]); // timestamp
        f.extend_from_slice(&100u16.to_le_bytes()); // beacon interval
        f.extend_from_slice(&[0x31, 0x04]); // capability info

        // Tagged parameters
        f.push(TAG_SSID);
        f.push(ssid.len() as u8);
        f.extend_from_slice(ssid.as_bytes());

        f.push(TAG_DS_PARAM);
        f.push(0x01);
        f.push(channel);

        f
    }

    // ------------------------------------------------------------------
    // Tests
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_beacon_valid() {
        let bssid = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let frame = make_beacon(bssid, "MyNetwork", 6, -42, SUBTYPE_BEACON);

        let info = parse_beacon(&frame).expect("should parse beacon");
        assert_eq!(info.bssid, bssid);
        assert_eq!(info.ssid, "MyNetwork");
        assert_eq!(info.channel, 6);
        assert_eq!(info.rssi, -42i16);
    }

    #[test]
    fn test_parse_probe_response() {
        let bssid = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let frame = make_beacon(bssid, "ProbeNet", 11, -55, SUBTYPE_PROBE_RESP);

        let info = parse_beacon(&frame).expect("probe response should parse");
        assert_eq!(info.bssid, bssid);
        assert_eq!(info.ssid, "ProbeNet");
        assert_eq!(info.channel, 11);
        assert_eq!(info.rssi, -55i16);
    }

    #[test]
    fn test_parse_non_beacon_returns_none() {
        // Data frame: FC byte0 = 0x08 (type=2/data, subtype=0)
        let mut frame = vec![0u8; 50];
        frame[2] = 0x08; // radiotap length = 8
        frame[8] = 0x08; // 802.11 FC byte0: data frame
        assert!(parse_beacon(&frame).is_none());
    }

    #[test]
    fn test_parse_truncated_returns_none() {
        assert!(parse_beacon(&[]).is_none());
        assert!(parse_beacon(&[0, 0, 0]).is_none());
        // Radiotap says 8 bytes long but frame is only 8 bytes total — no 802.11 header
        assert!(parse_beacon(&[0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00]).is_none());
    }

    #[test]
    fn test_parse_hidden_ssid() {
        let bssid = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01];
        let frame = make_beacon(bssid, "", 1, -70, SUBTYPE_BEACON);

        let info = parse_beacon(&frame).expect("hidden beacon should parse");
        assert_eq!(info.bssid, bssid);
        assert_eq!(info.ssid, "");
        assert_eq!(info.channel, 1);
    }
}

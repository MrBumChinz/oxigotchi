// ---------------------------------------------------------------------------
// SSID Resolver — incremental pcapng reader + JSON persistence
// ---------------------------------------------------------------------------
//
// Reads AngryOxide's pcapng capture files incrementally, extracts BSSID->SSID
// mappings from beacon/probe-response frames, and persists them to a JSON file.
// The `tick()` method is unix-only; everything else is portable.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsidEntry {
    pub ssid: String,
    pub last_seen: u64, // unix timestamp (seconds)
}

/// Byte order of the current pcapng section (from SHB).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Endian {
    Little,
    Big,
}

pub struct SsidResolver {
    map: HashMap<[u8; 6], SsidEntry>,
    file_offset: u64,
    current_file: Option<PathBuf>,
    endian: Endian,
    linktype: HashMap<u32, u16>, // interface_id -> link_type
    json_path: PathBuf,
    dirty: bool,
    max_entries: usize,
}

// ---------------------------------------------------------------------------
// MAC formatting helpers
// ---------------------------------------------------------------------------

fn format_mac(mac: &[u8; 6]) -> String {
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

fn parse_mac(s: &str) -> Option<[u8; 6]> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return None;
    }
    let mut mac = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        mac[i] = u8::from_str_radix(part, 16).ok()?;
    }
    Some(mac)
}

// ---------------------------------------------------------------------------
// Endian-aware read helpers
// ---------------------------------------------------------------------------

fn read_u16(data: &[u8], endian: Endian) -> u16 {
    match endian {
        Endian::Little => u16::from_le_bytes([data[0], data[1]]),
        Endian::Big => u16::from_be_bytes([data[0], data[1]]),
    }
}

fn read_u32(data: &[u8], endian: Endian) -> u32 {
    match endian {
        Endian::Little => u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
        Endian::Big => u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
    }
}

// ---------------------------------------------------------------------------
// pcapng block type constants
// ---------------------------------------------------------------------------

const BLOCK_SHB: u32 = 0x0A0D_0D0A;
const BLOCK_IDB: u32 = 0x0000_0001;
const BLOCK_EPB: u32 = 0x0000_0006;

/// pcapng byte-order magic value (native endian in the SHB).
const BOM_MAGIC: u32 = 0x1A2B_3C4D;

/// Radiotap link type in pcapng IDB.
const LINKTYPE_IEEE802_11_RADIOTAP: u16 = 127;

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl SsidResolver {
    pub fn new(json_path: PathBuf) -> Self {
        Self {
            map: HashMap::new(),
            file_offset: 0,
            current_file: None,
            endian: Endian::Little,
            linktype: HashMap::new(),
            json_path,
            dirty: false,
            max_entries: 10_000,
        }
    }

    /// Look up SSID by BSSID.
    pub fn get(&self, bssid: &[u8; 6]) -> Option<&str> {
        self.map.get(bssid).map(|e| e.ssid.as_str())
    }

    /// Insert a BSSID->SSID mapping. Skips empty SSIDs. Never overwrites
    /// a non-empty SSID with an empty one.
    pub fn insert(&mut self, bssid: [u8; 6], ssid: &str) {
        if ssid.is_empty() {
            return;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.map.insert(
            bssid,
            SsidEntry {
                ssid: ssid.to_string(),
                last_seen: now,
            },
        );
        self.dirty = true;
        self.prune();
    }

    /// Load from JSON file. Silent on any error.
    pub fn load(&mut self) {
        let data = match std::fs::read_to_string(&self.json_path) {
            Ok(d) => d,
            Err(_) => return,
        };
        let parsed: HashMap<String, SsidEntry> = match serde_json::from_str(&data) {
            Ok(p) => p,
            Err(_) => return,
        };
        for (mac_str, entry) in parsed {
            if let Some(mac) = parse_mac(&mac_str) {
                self.map.insert(mac, entry);
            }
        }
    }

    /// Atomic write: write to .json.tmp, then rename. No-op if not dirty.
    pub fn flush(&mut self) {
        if !self.dirty {
            return;
        }
        let serializable: HashMap<String, &SsidEntry> = self
            .map
            .iter()
            .map(|(k, v)| (format_mac(k), v))
            .collect();

        let tmp_path = self.json_path.with_extension("json.tmp");
        let json = match serde_json::to_string_pretty(&serializable) {
            Ok(j) => j,
            Err(_) => return,
        };
        if std::fs::write(&tmp_path, json).is_err() {
            return;
        }
        if std::fs::rename(&tmp_path, &self.json_path).is_ok() {
            self.dirty = false;
        }
    }

    /// Number of entries in the map.
    pub fn entry_count(&self) -> usize {
        self.map.len()
    }

    /// Return a cloned snapshot of all BSSID->SSID mappings.
    /// Used to avoid borrow conflicts when passing a lookup closure alongside
    /// a mutable borrow of another Daemon field.
    pub fn snapshot(&self) -> HashMap<[u8; 6], String> {
        self.map.iter().map(|(k, v)| (*k, v.ssid.clone())).collect()
    }

    /// Prune oldest entries if map exceeds max_entries.
    fn prune(&mut self) {
        while self.map.len() > self.max_entries {
            // Find the entry with the smallest last_seen.
            let oldest = self
                .map
                .iter()
                .min_by_key(|(_, v)| v.last_seen)
                .map(|(k, _)| *k);
            if let Some(key) = oldest {
                self.map.remove(&key);
            } else {
                break;
            }
        }
    }

    // -----------------------------------------------------------------------
    // pcapng incremental reader (unix only)
    // -----------------------------------------------------------------------

    /// Scan for the newest pcapng capture file and read new blocks.
    #[cfg(unix)]
    pub fn tick(&mut self, ao_output_dir: &Path) {
        let newest = match find_newest_pcapng(ao_output_dir) {
            Some(p) => p,
            None => return,
        };

        // File changed or was truncated? Reset parser state.
        let file_changed = self
            .current_file
            .as_ref()
            .map_or(true, |cur| cur != &newest);
        let file_len = std::fs::metadata(&newest)
            .map(|m| m.len())
            .unwrap_or(0);
        if file_changed || file_len < self.file_offset {
            self.file_offset = 0;
            self.endian = Endian::Little;
            self.linktype.clear();
            self.current_file = Some(newest.clone());
        }

        if file_len <= self.file_offset {
            return;
        }

        // Read new bytes from file_offset to end.
        use std::io::{Read, Seek, SeekFrom};
        let mut file = match std::fs::File::open(&newest) {
            Ok(f) => f,
            Err(_) => return,
        };
        if file.seek(SeekFrom::Start(self.file_offset)).is_err() {
            return;
        }
        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).is_err() {
            return;
        }

        self.parse_blocks(&buf);
    }

    #[cfg(not(unix))]
    pub fn tick(&mut self, _ao_output_dir: &Path) {
        // No-op on non-unix platforms.
    }

    /// Parse pcapng blocks from a byte buffer, advancing file_offset for
    /// each complete block processed.
    fn parse_blocks(&mut self, data: &[u8]) {
        let mut pos = 0usize;
        // Track next IDB interface_id assignment.
        let mut next_idb_id: u32 = if self.file_offset == 0 {
            0
        } else {
            self.linktype.len() as u32
        };

        while pos + 12 <= data.len() {
            // Every block starts with: block_type (4B) + block_total_length (4B)
            // We must detect endian from SHB before using read_u32 for SHB itself.
            let raw_type_bytes = &data[pos..pos + 4];
            let raw_type_le = u32::from_le_bytes([
                raw_type_bytes[0],
                raw_type_bytes[1],
                raw_type_bytes[2],
                raw_type_bytes[3],
            ]);
            let _raw_type_be = u32::from_be_bytes([
                raw_type_bytes[0],
                raw_type_bytes[1],
                raw_type_bytes[2],
                raw_type_bytes[3],
            ]);

            // SHB is endian-neutral in its type field (0x0A0D0D0A is a palindrome).
            let is_shb = raw_type_le == BLOCK_SHB;

            let (block_type, block_len) = if is_shb {
                // For SHB, we need to read the byte-order magic first to know endian.
                // BOM is at offset 8 inside the block body (after type + length).
                if pos + 16 > data.len() {
                    break; // truncated SHB
                }
                // Try LE first.
                let bom_le = u32::from_le_bytes([
                    data[pos + 8],
                    data[pos + 9],
                    data[pos + 10],
                    data[pos + 11],
                ]);
                if bom_le == BOM_MAGIC {
                    self.endian = Endian::Little;
                } else {
                    self.endian = Endian::Big;
                }
                let blen = read_u32(&data[pos + 4..], self.endian);
                (BLOCK_SHB, blen)
            } else {
                let bt = read_u32(&data[pos..], self.endian);
                let blen = read_u32(&data[pos + 4..], self.endian);
                (bt, blen)
            };

            // block_len must be at least 12 (type + length + trailing length).
            let block_len = block_len as usize;
            if block_len < 12 {
                break; // corrupt
            }

            // Truncated block: stop, don't advance offset.
            if pos + block_len > data.len() {
                break;
            }

            let body = &data[pos + 8..pos + block_len - 4];

            match block_type {
                BLOCK_SHB => {
                    // Body: BOM(4) + major(2) + minor(2) + section_length(8)
                    // Reset linktype map for new section.
                    self.linktype.clear();
                    next_idb_id = 0;
                }
                BLOCK_IDB => {
                    // Body: link_type(2) + reserved(2) + snap_len(4) + options...
                    if body.len() >= 4 {
                        let lt = read_u16(body, self.endian);
                        self.linktype.insert(next_idb_id, lt);
                        next_idb_id += 1;
                    }
                }
                BLOCK_EPB => {
                    // Body: interface_id(4) + ts_high(4) + ts_low(4) + captured_len(4) +
                    //       original_len(4) + packet_data(captured_len padded to 4) + options...
                    if body.len() >= 20 {
                        let iface_id = read_u32(body, self.endian);
                        let captured_len = read_u32(&body[12..], self.endian) as usize;

                        // Check that this interface uses radiotap.
                        let lt = self.linktype.get(&iface_id).copied().unwrap_or(0);
                        if lt == LINKTYPE_IEEE802_11_RADIOTAP && body.len() >= 20 + captured_len {
                            let frame_data = &body[20..20 + captured_len];
                            if let Some(info) = crate::ieee80211::parse_beacon(frame_data) {
                                if !info.ssid.is_empty() {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs();
                                    self.map.insert(
                                        info.bssid,
                                        SsidEntry {
                                            ssid: info.ssid,
                                            last_seen: now,
                                        },
                                    );
                                    self.dirty = true;
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Unknown block type — skip.
                }
            }

            // Advance past this complete block.
            pos += block_len;
            self.file_offset += block_len as u64;
        }

        self.prune();
    }
}

// ---------------------------------------------------------------------------
// find_newest_pcapng (unix only)
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn find_newest_pcapng(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_str()?;
        if name.starts_with("capture-") && name.ends_with(".pcapng") {
            if let Ok(meta) = std::fs::metadata(&path) {
                if let Ok(mtime) = meta.modified() {
                    match &best {
                        Some((_, best_mtime)) if mtime <= *best_mtime => {}
                        _ => best = Some((path, mtime)),
                    }
                }
            }
        }
    }
    best.map(|(p, _)| p)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_skips_empty_ssid() {
        let mut resolver = SsidResolver::new(PathBuf::from("/tmp/test.json"));
        let bssid = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        resolver.insert(bssid, "");
        assert_eq!(resolver.entry_count(), 0);
        assert!(resolver.get(&bssid).is_none());
    }

    #[test]
    fn test_insert_no_overwrite_with_empty() {
        let mut resolver = SsidResolver::new(PathBuf::from("/tmp/test.json"));
        let bssid = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        resolver.insert(bssid, "RealNetwork");
        assert_eq!(resolver.get(&bssid), Some("RealNetwork"));

        // Try to overwrite with empty — should be rejected.
        resolver.insert(bssid, "");
        assert_eq!(resolver.get(&bssid), Some("RealNetwork"));
    }

    #[test]
    fn test_insert_overwrites_with_new_ssid() {
        let mut resolver = SsidResolver::new(PathBuf::from("/tmp/test.json"));
        let bssid = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        resolver.insert(bssid, "OldName");
        assert_eq!(resolver.get(&bssid), Some("OldName"));

        resolver.insert(bssid, "NewName");
        assert_eq!(resolver.get(&bssid), Some("NewName"));
    }

    #[test]
    fn test_prune_oldest() {
        let mut resolver = SsidResolver::new(PathBuf::from("/tmp/test.json"));
        resolver.max_entries = 3;

        // Insert 3 entries with explicit timestamps.
        resolver.map.insert(
            [0x01, 0x00, 0x00, 0x00, 0x00, 0x00],
            SsidEntry {
                ssid: "First".to_string(),
                last_seen: 100,
            },
        );
        resolver.map.insert(
            [0x02, 0x00, 0x00, 0x00, 0x00, 0x00],
            SsidEntry {
                ssid: "Second".to_string(),
                last_seen: 200,
            },
        );
        resolver.map.insert(
            [0x03, 0x00, 0x00, 0x00, 0x00, 0x00],
            SsidEntry {
                ssid: "Third".to_string(),
                last_seen: 300,
            },
        );

        // Adding a 4th should evict the oldest (last_seen=100).
        resolver.insert([0x04, 0x00, 0x00, 0x00, 0x00, 0x00], "Fourth");
        assert_eq!(resolver.entry_count(), 3);
        assert!(resolver.get(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00]).is_none());
        assert!(resolver.get(&[0x02, 0x00, 0x00, 0x00, 0x00, 0x00]).is_some());
        assert!(resolver.get(&[0x03, 0x00, 0x00, 0x00, 0x00, 0x00]).is_some());
        assert!(resolver.get(&[0x04, 0x00, 0x00, 0x00, 0x00, 0x00]).is_some());
    }

    #[test]
    fn test_format_parse_mac_roundtrip() {
        let mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let formatted = format_mac(&mac);
        assert_eq!(formatted, "AA:BB:CC:DD:EE:FF");

        let parsed = parse_mac(&formatted).expect("should parse");
        assert_eq!(parsed, mac);

        // Lower-case should also parse.
        let lower = "aa:bb:cc:dd:ee:ff";
        let parsed_lower = parse_mac(lower).expect("should parse lower");
        assert_eq!(parsed_lower, mac);
    }

    #[test]
    fn test_json_roundtrip() {
        let dir = std::env::temp_dir();
        let json_path = dir.join("ssid_test_roundtrip.json");

        // Clean up from any prior run.
        let _ = std::fs::remove_file(&json_path);

        let bssid1 = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let bssid2 = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];

        // Write some entries.
        {
            let mut resolver = SsidResolver::new(json_path.clone());
            resolver.insert(bssid1, "Network1");
            resolver.insert(bssid2, "Network2");
            assert_eq!(resolver.entry_count(), 2);
            resolver.flush();
        }

        // Read them back in a fresh resolver.
        {
            let mut resolver = SsidResolver::new(json_path.clone());
            assert_eq!(resolver.entry_count(), 0);
            resolver.load();
            assert_eq!(resolver.entry_count(), 2);
            assert_eq!(resolver.get(&bssid1), Some("Network1"));
            assert_eq!(resolver.get(&bssid2), Some("Network2"));
        }

        // Clean up.
        let _ = std::fs::remove_file(&json_path);
    }

    #[test]
    fn test_parse_blocks_shb_idb_epb() {
        // Build a minimal pcapng with SHB + IDB(radiotap) + EPB(beacon frame).
        let mut pcapng = Vec::new();

        // --- SHB ---
        // Block type (0x0A0D0D0A) + Block total length + BOM + version + section len + BTL
        let shb_body_len = 4 + 2 + 2 + 8; // BOM + major + minor + section_length
        let shb_total = 12 + shb_body_len; // type(4) + len(4) + body + trailing_len(4)
        pcapng.extend_from_slice(&BLOCK_SHB.to_le_bytes());
        pcapng.extend_from_slice(&(shb_total as u32).to_le_bytes());
        pcapng.extend_from_slice(&BOM_MAGIC.to_le_bytes()); // byte-order magic
        pcapng.extend_from_slice(&1u16.to_le_bytes()); // major version
        pcapng.extend_from_slice(&0u16.to_le_bytes()); // minor version
        pcapng.extend_from_slice(&(-1i64 as u64).to_le_bytes()); // section length (unspecified)
        pcapng.extend_from_slice(&(shb_total as u32).to_le_bytes()); // trailing block length

        // --- IDB ---
        let idb_body_len = 2 + 2 + 4; // link_type + reserved + snap_len
        let idb_total = 12 + idb_body_len;
        pcapng.extend_from_slice(&BLOCK_IDB.to_le_bytes());
        pcapng.extend_from_slice(&(idb_total as u32).to_le_bytes());
        pcapng.extend_from_slice(&LINKTYPE_IEEE802_11_RADIOTAP.to_le_bytes()); // link_type
        pcapng.extend_from_slice(&0u16.to_le_bytes()); // reserved
        pcapng.extend_from_slice(&0u32.to_le_bytes()); // snap_len
        pcapng.extend_from_slice(&(idb_total as u32).to_le_bytes());

        // --- EPB with a beacon frame ---
        let bssid = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01];
        let frame = build_test_beacon(bssid, "TestNet", 6);
        let captured_len = frame.len() as u32;
        let padded_len = ((frame.len() + 3) / 4) * 4; // pad to 4-byte boundary
        let epb_body_len = 4 + 4 + 4 + 4 + 4 + padded_len; // iface_id + ts_high + ts_low + cap_len + orig_len + data
        let epb_total = 12 + epb_body_len;
        pcapng.extend_from_slice(&BLOCK_EPB.to_le_bytes());
        pcapng.extend_from_slice(&(epb_total as u32).to_le_bytes());
        pcapng.extend_from_slice(&0u32.to_le_bytes()); // interface_id = 0
        pcapng.extend_from_slice(&0u32.to_le_bytes()); // timestamp high
        pcapng.extend_from_slice(&0u32.to_le_bytes()); // timestamp low
        pcapng.extend_from_slice(&captured_len.to_le_bytes()); // captured length
        pcapng.extend_from_slice(&captured_len.to_le_bytes()); // original length
        pcapng.extend_from_slice(&frame);
        // Pad to 4-byte boundary.
        for _ in frame.len()..padded_len {
            pcapng.push(0x00);
        }
        pcapng.extend_from_slice(&(epb_total as u32).to_le_bytes());

        // Parse it.
        let mut resolver = SsidResolver::new(PathBuf::from("/tmp/test.json"));
        resolver.parse_blocks(&pcapng);

        assert_eq!(resolver.entry_count(), 1);
        assert_eq!(resolver.get(&bssid), Some("TestNet"));
    }

    #[test]
    fn test_parse_blocks_truncated_block() {
        // Build an SHB followed by a truncated IDB.
        let mut pcapng = Vec::new();

        // SHB
        let shb_body_len = 4 + 2 + 2 + 8;
        let shb_total = 12 + shb_body_len;
        pcapng.extend_from_slice(&BLOCK_SHB.to_le_bytes());
        pcapng.extend_from_slice(&(shb_total as u32).to_le_bytes());
        pcapng.extend_from_slice(&BOM_MAGIC.to_le_bytes());
        pcapng.extend_from_slice(&1u16.to_le_bytes());
        pcapng.extend_from_slice(&0u16.to_le_bytes());
        pcapng.extend_from_slice(&(-1i64 as u64).to_le_bytes());
        pcapng.extend_from_slice(&(shb_total as u32).to_le_bytes());

        // Truncated IDB — says it's 20 bytes, but we only write 10.
        let idb_total = 20u32;
        pcapng.extend_from_slice(&BLOCK_IDB.to_le_bytes());
        pcapng.extend_from_slice(&idb_total.to_le_bytes());
        pcapng.extend_from_slice(&[0x00, 0x00]); // only 2 more bytes instead of 12

        let mut resolver = SsidResolver::new(PathBuf::from("/tmp/test.json"));
        resolver.parse_blocks(&pcapng);

        // SHB was parsed, but IDB was truncated so should not crash.
        // file_offset should be at end of SHB only.
        assert_eq!(resolver.file_offset, shb_total as u64);
    }

    /// Helper: build a radiotap + beacon frame for testing.
    fn build_test_beacon(bssid: [u8; 6], ssid: &str, channel: u8) -> Vec<u8> {
        let mut f = Vec::new();

        // Radiotap header (8 bytes, minimal: no fields).
        f.push(0x00); // version
        f.push(0x00); // pad
        f.extend_from_slice(&8u16.to_le_bytes()); // length
        f.extend_from_slice(&0u32.to_le_bytes()); // present (no fields)

        // 802.11 management header (24 bytes).
        f.push(0x80); // beacon frame control byte 0
        f.push(0x00); // frame control byte 1
        f.extend_from_slice(&[0x00, 0x00]); // duration
        f.extend_from_slice(&[0xFF; 6]); // DA (broadcast)
        f.extend_from_slice(&bssid); // SA
        f.extend_from_slice(&bssid); // BSSID (addr3)
        f.extend_from_slice(&[0x00, 0x00]); // sequence control

        // Fixed parameters (12 bytes).
        f.extend_from_slice(&[0x00; 8]); // timestamp
        f.extend_from_slice(&100u16.to_le_bytes()); // beacon interval
        f.extend_from_slice(&[0x31, 0x04]); // capability info

        // Tagged parameters.
        f.push(0x00); // SSID tag
        f.push(ssid.len() as u8);
        f.extend_from_slice(ssid.as_bytes());
        f.push(0x03); // DS Parameter Set tag
        f.push(0x01);
        f.push(channel);

        f
    }
}

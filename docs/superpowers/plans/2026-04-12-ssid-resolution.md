# SSID Resolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show real AP names in the Nearby Networks dashboard, rename capture files with SSIDs, and add whitelist visibility badges.

**Architecture:** Extract shared beacon parser from wifi/mod.rs into ieee80211.rs. Add pcapng-reading SSID resolver (ssid.rs) that incrementally extracts BSSID→SSID from AO's live capture. Wire into sync_to_web, capture naming, and dashboard rendering.

**Tech Stack:** Pure Rust, no new crate dependencies. serde_json (already in deps) for persistence.

---

## File Structure

| File | Responsibility |
|------|---------------|
| `rust/src/ieee80211.rs` | **NEW** — Shared radiotap+802.11 beacon parser (portable) |
| `rust/src/ssid.rs` | **NEW** — pcapng incremental reader + SSID resolver + JSON persistence |
| `rust/src/wifi/mod.rs` | Refactor to call ieee80211 parser |
| `rust/src/main.rs` | Wire resolver, update sync_to_web SSID priority, whitelist propagation |
| `rust/src/capture/mod.rs` | SSID-based filename generation + rename in both capture modes |
| `rust/src/web/mod.rs` | Add `whitelisted` field to ApEntry |
| `rust/src/web/html.rs` | WL badge in both AP table render paths |

---

### Task 1: Extract shared beacon parser into ieee80211.rs

**Files:**
- Create: `rust/src/ieee80211.rs`
- Modify: `rust/src/main.rs` (add `mod ieee80211;`)
- Modify: `rust/src/wifi/mod.rs:438-520` (replace inline parser with call)

- [ ] **Step 1: Write tests for the shared parser**

In `rust/src/ieee80211.rs`:

```rust
//! Shared 802.11 beacon/probe-response parser.
//! Accepts full radiotap+802.11 bytes, extracts BSSID, SSID, channel, RSSI.
//! Portable — no platform-specific code.

/// Parsed beacon or probe-response frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeaconInfo {
    pub bssid: [u8; 6],
    pub ssid: String,
    pub channel: u8,
    pub rssi: i16,
}

/// Parse a full radiotap+802.11 frame. Returns None if not a beacon/probe-response.
pub fn parse_beacon(_raw: &[u8]) -> Option<BeaconInfo> {
    None // stub — tests should fail
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal valid beacon frame:
    // Radiotap: version=0, pad=0, len=8 (LE), present=0x00000000 (no fields)
    // 802.11: FC=0x80 0x00 (beacon), duration=0, DA=ff:ff:ff:ff:ff:ff,
    //         SA=AA:BB:CC:DD:EE:FF, BSSID=AA:BB:CC:DD:EE:FF, seq=0
    // Fixed params: 12 bytes (timestamp=0, interval=100, capabilities=0)
    // Tagged: SSID tag (id=0, len=7, "TestNet"), DS param (id=3, len=1, ch=6)
    fn make_beacon(ssid: &[u8], channel: u8, bssid: [u8; 6]) -> Vec<u8> {
        let mut frame = Vec::new();
        // Radiotap header: version=0, pad=0, length=8 (LE), present=0
        frame.extend_from_slice(&[0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00]);
        // 802.11 header: FC=0x80 0x00 (beacon mgmt frame)
        frame.extend_from_slice(&[0x80, 0x00]);
        // Duration
        frame.extend_from_slice(&[0x00, 0x00]);
        // DA (broadcast)
        frame.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        // SA = BSSID
        frame.extend_from_slice(&bssid);
        // BSSID
        frame.extend_from_slice(&bssid);
        // Sequence control
        frame.extend_from_slice(&[0x00, 0x00]);
        // Fixed params: timestamp (8), beacon interval (2), capability (2)
        frame.extend_from_slice(&[0; 8]); // timestamp
        frame.extend_from_slice(&[0x64, 0x00]); // interval = 100
        frame.extend_from_slice(&[0x00, 0x00]); // capability
        // Tag: SSID (id=0)
        frame.push(0x00); // tag id
        frame.push(ssid.len() as u8); // tag length
        frame.extend_from_slice(ssid);
        // Tag: DS Parameter Set (id=3)
        frame.push(0x03); // tag id
        frame.push(0x01); // tag length
        frame.push(channel);
        frame
    }

    #[test]
    fn test_parse_beacon_valid() {
        let bssid = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let raw = make_beacon(b"TestNet", 6, bssid);
        let info = parse_beacon(&raw).expect("should parse valid beacon");
        assert_eq!(info.bssid, bssid);
        assert_eq!(info.ssid, "TestNet");
        assert_eq!(info.channel, 6);
    }

    #[test]
    fn test_parse_probe_response() {
        let bssid = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let mut raw = make_beacon(b"ProbeNet", 11, bssid);
        // Change FC from beacon (0x80) to probe response (0x50)
        raw[8] = 0x50;
        let info = parse_beacon(&raw).expect("should parse probe response");
        assert_eq!(info.ssid, "ProbeNet");
        assert_eq!(info.channel, 11);
    }

    #[test]
    fn test_parse_non_beacon_returns_none() {
        let bssid = [0xAA; 6];
        let mut raw = make_beacon(b"Test", 1, bssid);
        // Change FC to data frame (type=2, subtype=0)
        raw[8] = 0x08;
        assert!(parse_beacon(&raw).is_none());
    }

    #[test]
    fn test_parse_truncated_returns_none() {
        assert!(parse_beacon(&[0x00, 0x00, 0x08, 0x00]).is_none());
        assert!(parse_beacon(&[]).is_none());
    }

    #[test]
    fn test_parse_hidden_ssid() {
        let bssid = [0xAA; 6];
        let raw = make_beacon(b"", 1, bssid);
        let info = parse_beacon(&raw).expect("should parse hidden SSID beacon");
        assert_eq!(info.ssid, "");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rust && cargo test ieee80211 2>&1 | tail -10`
Expected: 5 tests FAIL (stub returns None)

- [ ] **Step 3: Implement the parser**

Replace the stub `parse_beacon` in `rust/src/ieee80211.rs`:

```rust
// Constants
const IEEE80211_TYPE_MGMT: u8 = 0x00;
const IEEE80211_SUBTYPE_BEACON: u8 = 0x80;
const IEEE80211_SUBTYPE_PROBE_RESP: u8 = 0x50;
const TAG_SSID: u8 = 0;
const TAG_DS_PARAM: u8 = 3;

/// Parse a full radiotap+802.11 frame. Returns None if not a beacon/probe-response.
pub fn parse_beacon(raw: &[u8]) -> Option<BeaconInfo> {
    if raw.len() < 4 {
        return None;
    }
    // Radiotap header length (bytes 2-3, little-endian)
    let rt_len = u16::from_le_bytes([raw[2], raw[3]]) as usize;
    if raw.len() < rt_len {
        return None;
    }

    // Extract RSSI from radiotap if present (simplified: scan for dBm signal byte)
    let rssi = extract_rssi(raw, rt_len);

    let dot11 = &raw[rt_len..];
    if dot11.len() < 24 {
        return None;
    }

    let fc = dot11[0];
    let frame_type = fc & 0x0C;
    let frame_subtype = fc & 0xF0;

    if frame_type != IEEE80211_TYPE_MGMT {
        return None;
    }
    if frame_subtype != IEEE80211_SUBTYPE_BEACON && frame_subtype != IEEE80211_SUBTYPE_PROBE_RESP {
        return None;
    }

    let mut bssid = [0u8; 6];
    bssid.copy_from_slice(&dot11[16..22]);

    let tagged_start = 36; // 24-byte header + 12-byte fixed params
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
            break;
        }
        match tag_id {
            TAG_SSID => {
                ssid = String::from_utf8_lossy(&dot11[pos..pos + tag_len]).to_string();
            }
            TAG_DS_PARAM if tag_len >= 1 => {
                channel = dot11[pos];
            }
            _ => {}
        }
        pos += tag_len;
    }

    Some(BeaconInfo { bssid, ssid, channel, rssi })
}

/// Simplified radiotap RSSI extraction.
/// Scans the present bitmask for the dBm Antenna Signal field (bit 5).
fn extract_rssi(raw: &[u8], rt_len: usize) -> i16 {
    if rt_len < 8 || raw.len() < 8 {
        return -128;
    }
    let present = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
    if present & (1 << 5) == 0 {
        return -128; // no signal field
    }
    // Count fields before signal (bits 0-4 of present)
    // Each field has known sizes; simplified: signal byte is typically at offset 14 or 22
    // in common radiotap layouts. We scan for a plausible dBm value.
    for i in 8..rt_len {
        let val = raw[i] as i8;
        if (-100..=-10).contains(&(val as i16)) {
            return val as i16;
        }
    }
    -128
}
```

- [ ] **Step 4: Add mod declaration to main.rs**

At the top of `rust/src/main.rs`, after `mod ao;`:

```rust
mod ieee80211;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd rust && cargo test ieee80211 2>&1 | tail -10`
Expected: 5 tests PASS

- [ ] **Step 6: Refactor wifi/mod.rs to call shared parser**

In `rust/src/wifi/mod.rs`, replace the body of `parse_beacon_frame()` (lines 438-520) with a call to the shared parser:

```rust
pub fn parse_beacon_frame(raw: &[u8], rssi_override: Option<i8>) -> Option<ParsedBeacon> {
    let info = crate::ieee80211::parse_beacon(raw)?;
    Some(ParsedBeacon {
        bssid: info.bssid,
        ssid: info.ssid,
        channel: info.channel,
        rssi: rssi_override.map(|r| r as i16).unwrap_or(info.rssi) as i8,
    })
}
```

- [ ] **Step 7: Run full test suite**

Run: `cd rust && cargo test 2>&1 | tail -5`
Expected: All tests PASS (no regressions from refactor)

- [ ] **Step 8: Commit**

```bash
git add rust/src/ieee80211.rs rust/src/main.rs rust/src/wifi/mod.rs
git commit -m "refactor: extract shared beacon parser into ieee80211.rs"
```

---

### Task 2: SSID Resolver — data model, pcapng reader, JSON persistence

**Files:**
- Create: `rust/src/ssid.rs`
- Modify: `rust/src/main.rs` (add `mod ssid;`)

- [ ] **Step 1: Write pcapng block parsing tests**

Create `rust/src/ssid.rs` with data model and tests:

```rust
//! SSID resolver: incremental pcapng beacon parser + JSON persistence.
//! Reads AO's live capture file, extracts BSSID→SSID from beacons.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A resolved SSID entry with last-seen timestamp.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SsidEntry {
    pub ssid: String,
    pub last_seen: u64, // unix timestamp seconds
}

/// Byte order for pcapng section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Endian {
    Little,
    Big,
}

impl Default for Endian {
    fn default() -> Self {
        Endian::Little
    }
}

/// Incremental pcapng SSID resolver.
pub struct SsidResolver {
    map: HashMap<[u8; 6], SsidEntry>,
    file_offset: u64,
    current_file: Option<PathBuf>,
    endian: Endian,
    linktype: HashMap<u32, u16>, // interface_id → link_type
    json_path: PathBuf,
    dirty: bool,
    max_entries: usize,
}

// pcapng block type constants
const BT_SHB: u32 = 0x0A0D_0D0A;
const BT_IDB: u32 = 0x0000_0001;
const BT_EPB: u32 = 0x0000_0006;
const LINKTYPE_IEEE80211_RADIOTAP: u16 = 127;
const BYTE_ORDER_MAGIC: u32 = 0x1A2B_3C4D;

impl SsidResolver {
    pub fn new(json_path: PathBuf) -> Self {
        Self {
            map: HashMap::new(),
            file_offset: 0,
            current_file: None,
            endian: Endian::default(),
            linktype: HashMap::new(),
            json_path,
            dirty: false,
            max_entries: 10_000,
        }
    }

    /// Look up SSID for a BSSID. Returns None if unknown.
    pub fn get(&self, bssid: &[u8; 6]) -> Option<&str> {
        self.map.get(bssid).map(|e| e.ssid.as_str())
    }

    /// Insert a BSSID→SSID mapping. Skips empty SSIDs. Never overwrites
    /// a non-empty SSID with an empty one.
    pub fn insert(&mut self, bssid: [u8; 6], ssid: &str) {
        if ssid.is_empty() {
            return;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if let Some(existing) = self.map.get(&bssid) {
            if !existing.ssid.is_empty() && ssid.is_empty() {
                return; // don't overwrite known SSID
            }
        }
        self.map.insert(bssid, SsidEntry {
            ssid: ssid.to_string(),
            last_seen: now,
        });
        self.dirty = true;
        self.prune_if_needed();
    }

    fn prune_if_needed(&mut self) {
        if self.map.len() <= self.max_entries {
            return;
        }
        // Find oldest entry and remove it
        if let Some(oldest_key) = self
            .map
            .iter()
            .min_by_key(|(_, v)| v.last_seen)
            .map(|(k, _)| *k)
        {
            self.map.remove(&oldest_key);
        }
    }

    /// Load from JSON file. Errors silently (fresh start).
    pub fn load(&mut self) {
        let content = match std::fs::read_to_string(&self.json_path) {
            Ok(c) => c,
            Err(_) => return,
        };
        // JSON format: { "AA:BB:CC:DD:EE:FF": { "ssid": "...", "last_seen": N }, ... }
        let parsed: HashMap<String, SsidEntry> = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(_) => return,
        };
        for (mac_str, entry) in parsed {
            if let Some(bssid) = parse_mac(&mac_str) {
                self.map.insert(bssid, entry);
            }
        }
    }

    /// Flush to JSON file atomically (tmp + rename). No-op if not dirty.
    pub fn flush(&mut self) {
        if !self.dirty {
            return;
        }
        let serializable: HashMap<String, &SsidEntry> = self
            .map
            .iter()
            .map(|(k, v)| (format_mac(k), v))
            .collect();
        let json = match serde_json::to_string_pretty(&serializable) {
            Ok(j) => j,
            Err(_) => return,
        };
        let tmp = self.json_path.with_extension("json.tmp");
        if std::fs::write(&tmp, &json).is_ok() {
            let _ = std::fs::rename(&tmp, &self.json_path);
            self.dirty = false;
        }
    }

    /// Read a u32 from a byte slice with the section's endianness.
    fn read_u32(&self, bytes: &[u8]) -> u32 {
        let arr = [bytes[0], bytes[1], bytes[2], bytes[3]];
        match self.endian {
            Endian::Little => u32::from_le_bytes(arr),
            Endian::Big => u32::from_be_bytes(arr),
        }
    }

    fn read_u16(&self, bytes: &[u8]) -> u16 {
        let arr = [bytes[0], bytes[1]];
        match self.endian {
            Endian::Little => u16::from_le_bytes(arr),
            Endian::Big => u16::from_be_bytes(arr),
        }
    }

    /// Process new blocks from the current pcapng file.
    /// Called by the daemon on a 30s WallTimer.
    #[cfg(unix)]
    pub fn tick(&mut self, ao_output_dir: &Path) {
        use std::io::{Read, Seek, SeekFrom};

        // Find newest capture-*.pcapng
        let pcapng = match find_newest_pcapng(ao_output_dir) {
            Some(p) => p,
            None => return,
        };

        // Detect file rotation
        let file_len = std::fs::metadata(&pcapng)
            .map(|m| m.len())
            .unwrap_or(0);
        if self.current_file.as_ref() != Some(&pcapng) || file_len < self.file_offset {
            self.file_offset = 0;
            self.endian = Endian::default();
            self.linktype.clear();
            self.current_file = Some(pcapng.clone());
        }

        let mut file = match std::fs::File::open(&pcapng) {
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

        let mut pos: usize = 0;
        while pos + 8 <= buf.len() {
            // Read block type and length
            let block_type = self.read_u32(&buf[pos..pos + 4]);
            let block_len = self.read_u32(&buf[pos + 4..pos + 8]) as usize;

            if block_len < 12 || pos + block_len > buf.len() {
                // Truncated block — roll back to this boundary
                break;
            }

            match block_type {
                BT_SHB => {
                    if block_len >= 16 && pos + 16 <= buf.len() {
                        let magic = u32::from_le_bytes([
                            buf[pos + 8], buf[pos + 9], buf[pos + 10], buf[pos + 11],
                        ]);
                        self.endian = if magic == BYTE_ORDER_MAGIC {
                            Endian::Little
                        } else {
                            Endian::Big
                        };
                        self.linktype.clear();
                    }
                }
                BT_IDB => {
                    if block_len >= 16 && pos + 12 <= buf.len() {
                        let lt = self.read_u16(&buf[pos + 8..pos + 10]);
                        let iface_id = self.linktype.len() as u32;
                        self.linktype.insert(iface_id, lt);
                    }
                }
                BT_EPB => {
                    if block_len >= 28 && pos + 28 <= buf.len() {
                        let iface_id = self.read_u32(&buf[pos + 8..pos + 12]);
                        let cap_len = self.read_u32(&buf[pos + 20..pos + 24]) as usize;
                        let data_start = pos + 28;
                        if data_start + cap_len <= pos + block_len
                            && self.linktype.get(&iface_id) == Some(&LINKTYPE_IEEE80211_RADIOTAP)
                        {
                            let frame = &buf[data_start..data_start + cap_len];
                            if let Some(info) = crate::ieee80211::parse_beacon(frame) {
                                self.insert(info.bssid, &info.ssid);
                            }
                        }
                    }
                }
                _ => {} // skip unknown block types
            }

            pos += block_len;
        }

        self.file_offset += pos as u64;
    }

    #[cfg(not(unix))]
    pub fn tick(&mut self, _ao_output_dir: &Path) {
        // No-op on non-unix (dev/test)
    }

    pub fn entry_count(&self) -> usize {
        self.map.len()
    }
}

fn format_mac(bssid: &[u8; 6]) -> String {
    bssid
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
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

#[cfg(unix)]
fn find_newest_pcapng(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("capture-") && n.ends_with(".pcapng"))
        {
            if let Ok(meta) = entry.metadata() {
                let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                if best.as_ref().is_none_or(|(_, t)| mtime > *t) {
                    best = Some((path, mtime));
                }
            }
        }
    }
    best.map(|(p, _)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_skips_empty_ssid() {
        let mut r = SsidResolver::new(PathBuf::from("/tmp/test.json"));
        r.insert([0xAA; 6], "");
        assert_eq!(r.entry_count(), 0);
    }

    #[test]
    fn test_insert_no_overwrite_with_empty() {
        let mut r = SsidResolver::new(PathBuf::from("/tmp/test.json"));
        r.insert([0xAA; 6], "MyNet");
        r.insert([0xAA; 6], "");
        assert_eq!(r.get(&[0xAA; 6]), Some("MyNet"));
    }

    #[test]
    fn test_insert_overwrites_with_new_ssid() {
        let mut r = SsidResolver::new(PathBuf::from("/tmp/test.json"));
        r.insert([0xAA; 6], "OldNet");
        r.insert([0xAA; 6], "NewNet");
        assert_eq!(r.get(&[0xAA; 6]), Some("NewNet"));
    }

    #[test]
    fn test_prune_oldest() {
        let mut r = SsidResolver::new(PathBuf::from("/tmp/test.json"));
        r.max_entries = 2;
        r.map.insert([0x01; 6], SsidEntry { ssid: "First".into(), last_seen: 100 });
        r.map.insert([0x02; 6], SsidEntry { ssid: "Second".into(), last_seen: 200 });
        r.insert([0x03; 6], "Third"); // triggers prune, removes "First" (oldest)
        assert!(r.get(&[0x01; 6]).is_none());
        assert!(r.get(&[0x02; 6]).is_some());
        assert!(r.get(&[0x03; 6]).is_some());
    }

    #[test]
    fn test_format_parse_mac_roundtrip() {
        let mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let s = format_mac(&mac);
        assert_eq!(s, "AA:BB:CC:DD:EE:FF");
        assert_eq!(parse_mac(&s), Some(mac));
    }

    #[test]
    fn test_json_roundtrip() {
        let mut r = SsidResolver::new(std::env::temp_dir().join("ssid_test.json"));
        r.insert([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF], "TestNet");
        r.flush();
        let mut r2 = SsidResolver::new(r.json_path.clone());
        r2.load();
        assert_eq!(r2.get(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]), Some("TestNet"));
        let _ = std::fs::remove_file(&r.json_path);
    }
}
```

- [ ] **Step 2: Add mod declaration to main.rs**

After `mod ieee80211;`:

```rust
mod ssid;
```

- [ ] **Step 3: Run tests**

Run: `cd rust && cargo test ssid::tests 2>&1 | tail -10`
Expected: All 6 tests PASS

- [ ] **Step 4: Commit**

```bash
git add rust/src/ssid.rs rust/src/main.rs
git commit -m "feat: add SSID resolver with pcapng reader and JSON persistence"
```

---

### Task 3: Wire SSID resolver into Daemon

**Files:**
- Modify: `rust/src/main.rs` (Daemon struct, boot, main loop)

- [ ] **Step 1: Add SsidResolver to Daemon struct**

In `rust/src/main.rs`, add to the `Daemon` struct fields (after `captures`):

```rust
    ssid_resolver: ssid::SsidResolver,
```

And add a WallTimer field:

```rust
    ssid_resolve_timer: timer::WallTimer,
    ssid_flush_timer: timer::WallTimer,
```

- [ ] **Step 2: Initialize in Daemon construction**

In the `Daemon` constructor, add:

```rust
    ssid_resolver: ssid::SsidResolver::new(
        std::path::PathBuf::from("/home/pi/bssid_ssid.json")
    ),
    ssid_resolve_timer: timer::WallTimer::new(Duration::from_secs(30)),
    ssid_flush_timer: timer::WallTimer::new(Duration::from_secs(60)),
```

- [ ] **Step 3: Load JSON on boot**

In `Daemon::boot()`, after captures initialization:

```rust
    self.ssid_resolver.load();
    info!("SSID resolver: loaded {} cached entries", self.ssid_resolver.entry_count());
```

- [ ] **Step 4: Add tick + flush to main epoch loop**

In `run_epoch()`, after the capture phase and before `sync_to_web`:

```rust
    // SSID resolver: parse new beacon frames from AO pcapng
    if self.ssid_resolve_timer.check() {
        let ao_dir = std::path::Path::new(&self.ao.config.output_dir);
        self.ssid_resolver.tick(ao_dir);
    }
    // Flush SSID map to disk periodically
    if self.ssid_flush_timer.check() {
        self.ssid_resolver.flush();
    }
```

- [ ] **Step 5: Run full test suite**

Run: `cd rust && cargo test 2>&1 | tail -5`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add rust/src/main.rs
git commit -m "feat: wire SSID resolver into daemon loop with 30s/60s timers"
```

---

### Task 4: Update sync_to_web SSID priority chain

**Files:**
- Modify: `rust/src/main.rs:3077-3081` (AO-only AP SSID lookup)

- [ ] **Step 1: Replace "(AO)" fallback with priority chain**

In `sync_to_web`, replace the current SSID lookup for AO-only APs (lines ~3077-3081):

```rust
                // SSID priority: resolver map -> capture .22000 -> "(AO)"
                let ssid = self
                    .ssid_resolver
                    .get(&ao_bssid_bytes)
                    .map(|s| s.to_string())
                    .or_else(|| {
                        self.captures
                            .ssid_for(&ao_bssid_bytes)
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_else(|| "(AO)".into());
```

- [ ] **Step 2: Run full test suite**

Run: `cd rust && cargo test 2>&1 | tail -5`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add rust/src/main.rs
git commit -m "feat: SSID priority chain — resolver map before capture fallback"
```

---

### Task 5: Add whitelisted field to ApEntry + dashboard badge

**Files:**
- Modify: `rust/src/web/mod.rs:1268-1275` (ApEntry struct)
- Modify: `rust/src/main.rs:3017-3098` (set whitelisted in sync_to_web)
- Modify: `rust/src/web/html.rs` (both JS render paths)

- [ ] **Step 1: Add field to ApEntry**

In `rust/src/web/mod.rs`, add to the `ApEntry` struct:

```rust
    #[serde(default)]
    pub whitelisted: bool,
```

- [ ] **Step 2: Set whitelisted in sync_to_web for WiFi tracker APs**

In the WiFi tracker AP mapping in sync_to_web, add `whitelisted` to the `ApEntry`:

```rust
                    has_handshake: has_hs,
                    whitelisted: ap.whitelisted,
```

(The `AccessPoint` struct already has a `whitelisted` field set by `ApTracker::update`.)

- [ ] **Step 3: Set whitelisted in sync_to_web for AO-only APs**

In the AO-only AP section, after building the `ssid`, check whitelist:

```rust
                let whitelisted = self
                    .wifi
                    .tracker
                    .ssid_whitelist
                    .iter()
                    .any(|w| w.eq_ignore_ascii_case(&ssid))
                    || self.attacks.whitelist.iter().any(|w| *w == ao_bssid_bytes);
```

And add to the `ApEntry`:

```rust
                    has_handshake: has_hs,
                    whitelisted,
```

- [ ] **Step 4: Add WL badge CSS to html.rs**

In `rust/src/web/html.rs`, add to the `<style>` block:

```css
.wl-badge{display:inline-block;font-size:10px;background:#444;color:#999;padding:1px 5px;border-radius:8px;margin-left:4px;vertical-align:middle}
```

- [ ] **Step 5: Update refreshAps() in html.rs**

Replace the SSID cell in `refreshAps()`:

```javascript
            return '<tr' + (ap.whitelisted ? ' style="opacity:0.5"' : '') + '><td>' + esc(ap.ssid || '<hidden>') + (ap.whitelisted ? '<span class="wl-badge">WL</span>' : '') + '</td>' +
```

- [ ] **Step 6: Update updateApsFromWs() in html.rs**

Same change in `updateApsFromWs()`:

```javascript
            return '<tr' + (ap.whitelisted ? ' style="opacity:0.5"' : '') + '><td>' + esc(ap.ssid || '<hidden>') + (ap.whitelisted ? '<span class="wl-badge">WL</span>' : '') + '</td>' +
```

- [ ] **Step 7: Update test fixtures**

In any `ApEntry` test fixtures in `web/mod.rs`, add `whitelisted: false`.

- [ ] **Step 8: Run full test suite**

Run: `cd rust && cargo test 2>&1 | tail -5`
Expected: All tests PASS

- [ ] **Step 9: Commit**

```bash
git add rust/src/web/mod.rs rust/src/web/html.rs rust/src/main.rs
git commit -m "feat: whitelist badge on Nearby Networks — WL indicator + dimmed row"
```

---

### Task 6: Fix whitelist removal symmetry + AO propagation

**Files:**
- Modify: `rust/src/main.rs` (whitelist handlers, AO whitelist build)

- [ ] **Step 1: Add build_ao_whitelist helper**

In `rust/src/main.rs`, add a method to `Daemon`:

```rust
    /// Build merged whitelist for AO: SSID entries + formatted BSSID entries.
    fn build_ao_whitelist(&self) -> Vec<String> {
        let mut wl: Vec<String> = self.wifi.tracker.ssid_whitelist.clone();
        for mac in &self.attacks.whitelist {
            wl.push(
                mac.iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<Vec<_>>()
                    .join(":"),
            );
        }
        wl
    }
```

- [ ] **Step 2: Replace all 3 ao.config.whitelist assignment sites**

At each of the 3 sites (~lines 506, 2059, 2091) replace:

```rust
self.ao.config.whitelist = self.wifi.tracker.ssid_whitelist.clone();
```

with:

```rust
self.ao.config.whitelist = self.build_ao_whitelist();
```

- [ ] **Step 3: Fix whitelist remove handler to check both stores**

In `process_web_commands()`, in the whitelist remove section, add SSID removal:

```rust
    // Remove from SSID whitelist too (symmetry with add)
    self.wifi.tracker.ssid_whitelist.retain(|s| s != &value);
```

- [ ] **Step 4: Run full test suite**

Run: `cd rust && cargo test 2>&1 | tail -5`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add rust/src/main.rs
git commit -m "fix: whitelist removal symmetry + AO gets BSSID entries too"
```

---

### Task 7: Capture file naming with SSID

**Files:**
- Modify: `rust/src/capture/mod.rs` (add generate_ssid_filename, update move paths)

- [ ] **Step 1: Write filename generation tests**

In `rust/src/capture/mod.rs`, in the `tests` module:

```rust
    #[test]
    fn test_sanitize_ssid() {
        assert_eq!(sanitize_ssid("Normal"), "Normal");
        assert_eq!(sanitize_ssid("Has/Slash"), "Has_Slash");
        assert_eq!(sanitize_ssid("Has:Colon"), "Has_Colon");
        assert_eq!(sanitize_ssid("A\"B*C?D"), "A_B_C_D");
        assert_eq!(sanitize_ssid(""), "");
        // Cap at 32 chars
        assert_eq!(sanitize_ssid(&"A".repeat(50)).len(), 32);
    }

    #[test]
    fn test_generate_ssid_filename() {
        let name = generate_ssid_filename("TestNet", "aabbccddeeff", "20260412-153000");
        assert_eq!(name, "TestNet_aabbccddeeff_20260412-153000");
    }

    #[test]
    fn test_generate_ssid_filename_unknown() {
        let name = generate_ssid_filename("", "aabbccddeeff", "20260412-153000");
        assert_eq!(name, "unknown_aabbccddeeff_20260412-153000");
    }
```

- [ ] **Step 2: Implement sanitize_ssid and generate_ssid_filename**

In `rust/src/capture/mod.rs`:

```rust
/// Sanitize an SSID for use in filenames.
fn sanitize_ssid(ssid: &str) -> String {
    let sanitized: String = ssid
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c == ':' || c == '*'
                || c == '?' || c == '"' || c == '<' || c == '>'
                || c == '|' || c.is_control()
            {
                '_'
            } else {
                c
            }
        })
        .collect();
    if sanitized.len() > 32 {
        sanitized[..32].to_string()
    } else {
        sanitized
    }
}

/// Generate a capture filename stem: {SSID}_{BSSID}_{TIMESTAMP}
pub fn generate_ssid_filename(ssid: &str, bssid_hex: &str, timestamp: &str) -> String {
    let safe_ssid = sanitize_ssid(ssid);
    if safe_ssid.is_empty() {
        format!("unknown_{bssid_hex}_{timestamp}")
    } else {
        format!("{safe_ssid}_{bssid_hex}_{timestamp}")
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd rust && cargo test capture::tests 2>&1 | tail -10`
Expected: All capture tests PASS

- [ ] **Step 4: Update move_validated_captures to rename**

In `move_validated_captures`, after `fs::copy(&path, &pcapng_dest)` succeeds, add SSID-based rename logic. The function needs access to the SSID resolver, so add it as a parameter:

Change the function signature to accept an optional SSID lookup closure:

```rust
pub fn move_validated_captures(
    tmpfs_dir: &Path,
    permanent_dir: &Path,
    manager: &mut CaptureManager,
    ssid_lookup: impl Fn(&[u8; 6]) -> Option<String>,
) -> (usize, usize) {
```

Inside the validated-handshake branch, after copying to permanent_dir, rename using SSID:

```rust
                if fs::copy(&path, &pcapng_dest).is_ok() {
                    let _ = fs::copy(&companion, &companion_dest);
                    let _ = fs::remove_file(&path);
                    let _ = fs::remove_file(&companion);

                    // Rename with SSID if available
                    let (bssid, ssid_22k) = parse_22000_metadata(&pcapng_dest);
                    let ssid = if !ssid_22k.is_empty() {
                        ssid_22k
                    } else {
                        ssid_lookup(&bssid).unwrap_or_default()
                    };
                    let bssid_hex: String = bssid.iter().map(|b| format!("{b:02x}")).collect();
                    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
                    let stem = generate_ssid_filename(&ssid, &bssid_hex, &ts);
                    let new_pcapng = permanent_dir.join(format!("{stem}.pcapng"));
                    let new_companion = permanent_dir.join(format!("{stem}.22000"));

                    // Handle collision
                    let final_pcapng = resolve_collision(&new_pcapng);
                    let final_companion = final_pcapng.with_extension("22000");

                    let _ = fs::rename(&pcapng_dest, &final_pcapng);
                    if companion_dest.exists() {
                        let _ = fs::rename(&companion_dest, &final_companion);
                    }

                    moved += 1;
                    log::info!("capture: moved {} to SD", final_pcapng.file_name().unwrap().to_string_lossy());
                }
```

- [ ] **Step 5: Add resolve_collision helper**

```rust
/// If path exists, append _01, _02, etc. until it doesn't.
fn resolve_collision(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
    let ext = path.extension().unwrap().to_string_lossy().to_string();
    let parent = path.parent().unwrap();
    for i in 1..=99 {
        let candidate = parent.join(format!("{stem}_{i:02}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf() // give up after 99
}
```

- [ ] **Step 6: Update call site in main.rs**

Where `move_validated_captures` is called in main.rs, pass the SSID lookup:

```rust
    let resolver = &self.ssid_resolver;
    let (moved, deleted) = capture::move_validated_captures(
        &tmpfs_dir,
        &permanent_dir,
        &mut self.captures,
        |bssid| resolver.get(bssid).map(|s| s.to_string()),
    );
```

- [ ] **Step 7: Run full test suite**

Run: `cd rust && cargo test 2>&1 | tail -5`
Expected: All tests PASS

- [ ] **Step 8: Cross-compile**

Run: `wsl -d Ubuntu -- bash -lc "source ~/.cargo/env && cd /mnt/c/msys64/home/gelum/oxigotchi/rust && cargo build --release --target aarch64-unknown-linux-gnu 2>&1 | tail -3"`
Expected: Build succeeds

- [ ] **Step 9: Commit**

```bash
git add rust/src/capture/mod.rs rust/src/main.rs
git commit -m "feat: capture files renamed to SSID_BSSID_TIMESTAMP format"
```

---

### Task 8: Deploy and verify on Pi

- [ ] **Step 1: Deploy binary to Pi**

```bash
scp rust/target/aarch64-unknown-linux-gnu/release/oxigotchi pi@10.0.0.2:/home/pi/
ssh pi@10.0.0.2 'sudo systemctl stop rusty-oxigotchi && sudo cp /home/pi/oxigotchi /usr/local/bin/rusty-oxigotchi && sudo systemctl start rusty-oxigotchi'
```

- [ ] **Step 2: Verify SSID resolution**

```bash
ssh pi@10.0.0.2 'sleep 60 && curl -s http://localhost:8080/api/aps | python3 -m json.tool | head -30'
```

Expected: APs with SSIDs instead of "(AO)" for networks within beacon range.

- [ ] **Step 3: Verify whitelist badge**

```bash
ssh pi@10.0.0.2 'curl -s http://localhost:8080/api/aps | python3 -c "import json,sys; [print(a[\"ssid\"],a[\"whitelisted\"]) for a in json.load(sys.stdin)]"'
```

Expected: Whitelisted SSIDs show `True`.

- [ ] **Step 4: Verify JSON persistence**

```bash
ssh pi@10.0.0.2 'sleep 90 && cat /home/pi/bssid_ssid.json | python3 -m json.tool | head -10'
```

Expected: JSON file with BSSID→SSID entries.

- [ ] **Step 5: Push**

```bash
git push origin master
```

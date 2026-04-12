# SSID Resolution, Capture Naming & Whitelist Visibility

## Goal

Show real AP names (SSIDs) in the Nearby Networks dashboard card instead of "(AO)", rename capture files to include the SSID, and show a "WL" badge on whitelisted APs so users can confirm the whitelist is working. Dashboard-only — e-ink display is not in scope.

## Architecture

Three components share one BSSID-to-SSID mapping:

1. **SSID Resolver** — incremental pcapng beacon parser that builds the mapping
2. **Capture Renamer** — uses the mapping (with `.22000` priority) to name files on SD
3. **Whitelist Badge** — checks both SSID and BSSID whitelist stores to tag APs in the dashboard

All three consume a single `HashMap<[u8;6], SsidEntry>` owned by the Daemon.

## Tech Stack

Pure Rust, no new crate dependencies (`serde_json` already in deps). Reuses the existing `parse_beacon_frame()` from `wifi/mod.rs` for 802.11 frame parsing. pcapng block-level reading is new code (~80 lines) but frame parsing is shared.

---

## 1. Shared 802.11 Frame Parser (extract from wifi/mod.rs)

**Current state:** `wifi/mod.rs` has `parse_beacon_frame()` that handles radiotap header length extraction, RSSI, frame type detection (beacon 0x80 / probe-response 0x50), BSSID extraction from addr3, and SSID extraction from tagged parameters.

**Change:** Extract the frame-parsing core into a shared helper function in a new `rust/src/ieee80211.rs` module:

```rust
pub struct BeaconInfo {
    pub bssid: [u8; 6],
    pub ssid: String,       // empty string for hidden networks
    pub channel: u8,        // from DS Parameter Set tag (tag 3), 0 if absent
    pub rssi: Option<i16>,  // from radiotap, if present
}

/// Parse a full radiotap+802.11 frame and extract beacon/probe-response info.
/// Handles radiotap header length, skips to 802.11 frame, extracts RSSI.
/// Returns None if the frame is not a beacon or probe-response.
pub fn parse_beacon(radiotap_and_frame: &[u8]) -> Option<BeaconInfo>
```

The function accepts full radiotap+802.11 bytes (matching the current `wifi/mod.rs` approach). Both the live WiFi tracker and the pcapng resolver call this. One parser, no drift.

Parser code is **portable** (no `#[cfg(unix)]`) — pure byte-level parsing that runs on any platform for testing.

## 2. SSID Resolver (new module: rust/src/ssid.rs)

### Data model

```rust
pub struct SsidEntry {
    pub ssid: String,
    pub last_seen: u64,     // unix timestamp (seconds) — serializable across reboots
}

pub struct SsidResolver {
    map: HashMap<[u8; 6], SsidEntry>,
    // pcapng parser state persisted across ticks:
    file_offset: u64,
    current_file: Option<PathBuf>,
    section_endian: Endian,             // from SHB, persisted across ticks
    interface_linktype: HashMap<u32, u16>,  // IDB interface_id → linktype
    json_path: PathBuf,                 // /home/pi/bssid_ssid.json
    dirty: bool,
}
```

### Incremental pcapng reading

Runs on a 30-second `WallTimer` in the daemon loop. Each tick:

1. Find the current AO pcapng file in the **active AO output directory** (`self.ao.config.output_dir` — `/tmp/ao_captures/` in verified mode, `/home/pi/captures/` in collect-all mode). Newest `capture-*.pcapng` by mtime.
2. If file changed (different path or file size < offset = rotation/truncation): reset all parser state (offset, endian, linktype map) to zero.
3. Seek to `file_offset`, read new pcapng blocks:
   - **SHB** (`0x0A0D0D0A`): read byte-order magic, store `section_endian`. Reset `interface_linktype`.
   - **IDB** (`0x00000001`): read `link_type`, store in `interface_linktype[interface_id]`. Only linktype 127 (radiotap) is supported; skip others.
   - **EPB** (`0x00000006`): read `interface_id`, check linktype is radiotap, extract frame data, call `ieee80211::parse_beacon()`.
   - **Truncated block at EOF** (block length exceeds remaining bytes): roll back `file_offset` to the start of this block boundary and stop. Retry next tick when AO has written more.
   - **Any other block type**: skip by block length.
4. Update `file_offset` to current position (always on a block boundary).
5. For each parsed beacon: insert into map only if SSID is non-empty. Never overwrite a non-empty SSID with an empty one (hidden SSIDs are stored as `"(hidden)"` separately from `""`).

### Persistence

Every 60 seconds (separate WallTimer), if `dirty`:
- Serialize map to JSON: `{ "AA:BB:CC:DD:EE:FF": { "ssid": "MyNetwork", "last_seen": 1681234567 }, ... }`
- Write to `{json_path}.tmp`, then atomic rename to `{json_path}`
- Set `dirty = false`

On boot: load `json_path` if it exists, seed the map. Boot-loaded entries keep their original `last_seen` timestamps (not reset to now) so pruning order remains accurate.

Bounded at 10,000 entries max. If exceeded, prune entries with oldest `last_seen`.

### Platform guard

File-watching and pcapng reading are `#[cfg(unix)]`. The `ieee80211` parser and JSON serialization are portable and tested on host.

## 3. AP List Integration (modify sync_to_web in main.rs)

### SSID priority chain

Where we currently build `ApEntry` for AO-only APs, change the SSID lookup to:

1. WiFi tracker SSID (live beacon data — already used for tracked APs)
2. SSID resolver map (pcapng beacon backfill)
3. Capture database `ssid_for()` (from `.22000` metadata)
4. `hcxpcapngtool --lts` backfill (already runs in capture pipeline, populates `captures.files[].ssid`)
5. `"(AO)"` fallback

### Whitelist field

Add `whitelisted: bool` to `ApEntry`. Set by checking:
- AP's SSID against `wifi.tracker.ssid_whitelist` (SSID entries)
- AP's BSSID against `attacks.whitelist` (MAC entries)

Both stores are checked (the codebase has two separate stores — this is acknowledged, not a "unified model"). The `whitelisted` field reflects the union of both.

### AO whitelist propagation

Currently `main.rs` builds the AO whitelist file from `wifi.tracker.ssid_whitelist` only. Change to also include formatted BSSID strings from `attacks.whitelist` so AO itself skips MAC-whitelisted APs too. No `ao.rs` change needed — `ao.rs` just writes `Vec<String>` to the file; the change is in how `main.rs` builds that vector.

## 4. Capture File Naming (modify capture/mod.rs)

### Naming format

`{SSID}_{BSSID}_{TIMESTAMP}.pcapng` (and matching `.22000`)

- SSID sanitized: replace `/ \ : * ? " < > |` and control chars with `_`, cap at 32 chars
- BSSID: 12-char lowercase hex (no colons)
- TIMESTAMP: `YYYYMMDD-HHMMSS`
- If SSID unknown: `unknown_{BSSID}_{TIMESTAMP}`

Example: `Asiliukas_40ae30102906_20260412-153000.pcapng`

### Source priority for naming

1. `.22000` metadata SSID (first line — validated handshake, authoritative)
2. `hcxpcapngtool --lts` SSID (already extracted during capture processing)
3. SSID resolver map (best-effort from beacons)
4. `"unknown"` fallback

### Multi-network captures

If a `.22000` companion contains multiple lines (multiple APs in one pcapng), use the SSID/BSSID from the **first line**. This matches the current parser behavior which reads the first line only.

### Collision handling

If target filename already exists on SD, append `_01`, `_02`, etc. Check existence before rename.

### Both capture modes

- **Verified mode** (tmpfs→SD): rename during `move_validated_captures()` when copying to SD
- **Collect-all mode** (AO→SD direct): rename during `scan_directory()` for files that have a `.22000` companion and haven't been renamed yet. Only rename files whose mtime is >60 seconds old (file is stable/closed — AO is no longer appending).

Same `generate_ssid_filename()` function called from both paths.

### Upload queue consistency

If a file is renamed after being enqueued in `UploadQueue`, the queue entry's path must be updated to match. `generate_ssid_filename()` returns the new path so callers can propagate.

## 5. Dashboard Whitelist Badge (modify web/html.rs)

### JSON change

`ApEntry` gains `whitelisted: bool`. Serialized in both `/api/aps` and WebSocket AP updates.

### Rendering

In **both** `refreshAps()` and `updateApsFromWs()`:
- If `whitelisted === true`: append `<span class="wl-badge">WL</span>` after SSID text
- Row gets `opacity: 0.5` styling
- Badge CSS: small grey rounded pill, `font-size: 10px`, `background: #444`, `color: #999`

## 6. Testing

### Unit tests (host, cargo test)

- `ieee80211::parse_beacon` — valid beacon, probe-response, non-beacon frame, truncated frame, hidden SSID, RSSI extraction (portable, runs on Windows)
- `ssid.rs` — pcapng block parsing with crafted test pcapng bytes (SHB+IDB+EPB with beacon), truncated EPB at EOF, file rotation detection, empty-SSID-no-overwrite rule, max entries pruning, JSON round-trip with `last_seen` timestamps (parser tests portable, file-watching tests `#[cfg(unix)]`)
- `capture/mod.rs` — `generate_ssid_filename()` with various SSIDs (ASCII, Unicode, special chars, empty), collision suffix generation, mtime stability check for collect-all rename
- `web/mod.rs` — `ApEntry` serialization includes `whitelisted` field
- Whitelist propagation — BSSID whitelist entries appear in AO whitelist file alongside SSIDs

### Integration verification (on Pi)

- Walk with Pi, check dashboard shows SSIDs instead of "(AO)"
- Add SSID to whitelist, verify "WL" badge appears
- Add BSSID to whitelist, verify "WL" badge appears AND AO skips it
- Verify capture files on SD have `SSID_BSSID_TIMESTAMP` names
- Power-cycle Pi, verify JSON map persists and SSIDs appear immediately on next boot

---

## Files Modified

| File | Change |
|------|--------|
| `rust/src/ieee80211.rs` | **NEW** — shared beacon parser extracted from wifi/mod.rs (portable) |
| `rust/src/ssid.rs` | **NEW** — SSID resolver with incremental pcapng reader + JSON persistence |
| `rust/src/wifi/mod.rs` | Refactor: call `ieee80211::parse_beacon()` instead of inline parsing |
| `rust/src/main.rs` | Add `SsidResolver` to Daemon, wire WallTimers, update `sync_to_web` SSID priority + whitelist field, propagate BSSID whitelist to AO file |
| `rust/src/capture/mod.rs` | Add `generate_ssid_filename()`, update verified + collect-all move paths, upload queue path update |
| `rust/src/web/mod.rs` | Add `whitelisted: bool` to `ApEntry` |
| `rust/src/web/html.rs` | WL badge rendering in both JS render paths |

# SSID Resolution, Capture Naming & Whitelist Visibility

## Goal

Show real AP names (SSIDs) in the Nearby Networks dashboard card and e-ink display instead of "(AO)", rename capture files to include the SSID, and show a "WL" badge on whitelisted APs so users can confirm the whitelist is working.

## Architecture

Three components share one BSSID-to-SSID mapping:

1. **SSID Resolver** — incremental pcapng beacon parser that builds the mapping
2. **Capture Renamer** — uses the mapping (with `.22000` priority) to name files on SD
3. **Whitelist Badge** — uses unified whitelist model to tag APs in the dashboard

All three consume a single `HashMap<[u8;6], SsidEntry>` owned by the Daemon.

## Tech Stack

Pure Rust, zero new crate dependencies. Reuses the existing `parse_beacon_frame()` from `wifi/mod.rs` for 802.11 frame parsing (codex finding #1). pcapng block-level reading is new code (~80 lines) but frame parsing is shared.

---

## 1. Shared 802.11 Frame Parser (extract from wifi/mod.rs)

**Current state:** `wifi/mod.rs` has `parse_beacon_frame()` that handles radiotap header skipping, frame type detection (beacon 0x80 / probe-response 0x50), BSSID extraction from addr3, and SSID extraction from tagged parameters.

**Change:** Extract the frame-parsing core into a shared helper function in a new `rust/src/ieee80211.rs` module:

```rust
pub struct BeaconInfo {
    pub bssid: [u8; 6],
    pub ssid: String,       // empty string for hidden networks
    pub channel: u8,        // from DS Parameter Set tag (tag 3), 0 if absent
}

/// Parse a raw 802.11 frame (after radiotap) and extract beacon/probe-response info.
/// Returns None if the frame is not a beacon or probe-response.
pub fn parse_beacon(radiotap_frame: &[u8]) -> Option<BeaconInfo>
```

Both the live WiFi tracker and the pcapng resolver call this. One parser, no drift.

## 2. SSID Resolver (new module: rust/src/ssid.rs)

### Data model

```rust
pub struct SsidEntry {
    pub ssid: String,
    pub last_seen: Instant,
}

pub struct SsidResolver {
    map: HashMap<[u8; 6], SsidEntry>,
    file_offset: u64,           // tracks position in current pcapng
    current_file: Option<PathBuf>,
    json_path: PathBuf,         // /home/pi/bssid_ssid.json
    dirty: bool,                // true if map changed since last flush
}
```

### Incremental pcapng reading

Runs on a 30-second `WallTimer` in the daemon loop. Each tick:

1. Find the current AO pcapng file in `/tmp/ao_captures/` (newest `capture-*.pcapng`)
2. If file changed (different path or smaller size than offset = file rotation): reset offset to 0
3. Seek to `file_offset`, read new pcapng blocks:
   - Parse Section Header Block (SHB): validate magic `0x0A0D0D0A`, read byte order
   - Parse Interface Description Block (IDB): read `link_type` (must be 127 = radiotap)
   - Parse Enhanced Packet Blocks (EPB): extract frame data, call `ieee80211::parse_beacon()`
   - On any read error or truncated block at EOF: stop, keep current offset (retry next tick)
4. Update `file_offset` to current position
5. For each parsed beacon: insert into map if SSID is non-empty (never overwrite non-empty with empty — codex finding #8)

### Persistence

Every 60 seconds (separate WallTimer), if `dirty`:
- Write to `{json_path}.tmp` as JSON: `{ "AA:BB:CC:DD:EE:FF": "MyNetwork", ... }`
- Atomic rename `{json_path}.tmp` -> `{json_path}` (codex finding #7)
- Set `dirty = false`

On boot: load `json_path` if it exists, seed the map. Entries are best-effort enrichment — stale entries are harmless (just show an old SSID until overwritten by a fresh beacon).

Bounded at 10,000 entries max. If exceeded, prune entries with oldest `last_seen`. In practice a walk session sees ~200-500 APs.

### Platform guard

Entire module is `#[cfg(unix)]`. On Windows (dev/test), resolver is a no-op that returns an empty map.

## 3. AP List Integration (modify sync_to_web in main.rs)

### SSID priority chain

Where we currently build `ApEntry` for AO-only APs, change the SSID lookup to:

1. WiFi tracker SSID (live beacon data — already used for tracked APs)
2. SSID resolver map (pcapng beacon backfill)
3. Capture database `ssid_for()` (from `.22000` metadata)
4. `"(AO)"` fallback

### Whitelist field

Add `whitelisted: bool` to `ApEntry`. Set by checking:
- AP's SSID against `wifi.tracker.ssid_whitelist` (SSID entries)
- AP's BSSID against `attacks.whitelist` (MAC entries)

Both paths checked — unified model (codex finding #2). Also propagate BSSID whitelist entries to AO's whitelist file so AO itself skips them too (currently AO only gets SSID entries).

## 4. Capture File Naming (modify capture/mod.rs)

### Naming format

`{SSID}_{BSSID}_{TIMESTAMP}.pcapng` (and matching `.22000`)

- SSID sanitized: replace `/ \ : * ? " < > |` and control chars with `_`, cap at 32 chars
- BSSID: 12-char lowercase hex (no colons)
- TIMESTAMP: `YYYYMMDD-HHMMSS`
- If SSID unknown: `unknown_{BSSID}_{TIMESTAMP}`

Example: `Asiliukas_40ae30102906_20260412-153000.pcapng`

### Source priority for naming (codex finding #6)

1. `.22000` metadata SSID (validated handshake, authoritative)
2. SSID resolver map (best-effort from beacons)
3. `"unknown"` fallback

### Collision handling (codex finding #5)

If target filename already exists on SD, append `_01`, `_02`, etc. Check existence before rename.

### Both capture modes (codex finding #4)

- **Verified mode** (tmpfs→SD): rename during `move_validated_captures()`
- **Collect-all mode** (AO→SD direct): rename during periodic capture scan in `refresh_captures()`

Same `generate_ssid_filename()` function called from both paths.

## 5. Dashboard Whitelist Badge (modify web/html.rs)

### JSON change

`ApEntry` gains `whitelisted: bool`. Serialized in both `/api/aps` and WebSocket AP updates.

### Rendering

In **both** `refreshAps()` and `updateApsFromWs()` (codex finding #9):
- If `whitelisted === true`: append `<span class="wl-badge">WL</span>` after SSID text
- Row gets `opacity: 0.5` styling
- Badge CSS: small grey rounded pill, `font-size: 10px`, `background: #444`, `color: #999`

## 6. Testing

### Unit tests (host, cargo test)

- `ieee80211::parse_beacon` — valid beacon, probe-response, non-beacon frame, truncated frame, hidden SSID
- `ssid.rs` — pcapng block parsing with crafted test pcapng bytes (SHB+IDB+EPB with beacon), truncated EPB at EOF, file rotation detection, empty-SSID-no-overwrite rule, max entries pruning
- `capture/mod.rs` — `generate_ssid_filename()` with various SSIDs (ASCII, Unicode, special chars, empty), collision suffix generation
- `web/mod.rs` — `ApEntry` serialization includes `whitelisted` field
- Whitelist unification — BSSID whitelist entry appears in AO whitelist file

### Integration verification (on Pi)

- Walk with Pi, check dashboard shows SSIDs instead of "(AO)"
- Add SSID to whitelist, verify "WL" badge appears
- Verify capture files on SD have `SSID_BSSID_TIMESTAMP` names
- Power-cycle Pi, verify JSON map persists and SSIDs appear immediately on next boot

---

## Files Modified

| File | Change |
|------|--------|
| `rust/src/ieee80211.rs` | **NEW** — shared beacon parser extracted from wifi/mod.rs |
| `rust/src/ssid.rs` | **NEW** — SSID resolver with incremental pcapng reader |
| `rust/src/wifi/mod.rs` | Refactor: call `ieee80211::parse_beacon()` instead of inline parsing |
| `rust/src/main.rs` | Add `SsidResolver` to Daemon, wire WallTimers, update `sync_to_web` SSID priority + whitelist field, propagate BSSID whitelist to AO |
| `rust/src/capture/mod.rs` | Add `generate_ssid_filename()`, update both move paths to rename |
| `rust/src/web/mod.rs` | Add `whitelisted: bool` to `ApEntry` |
| `rust/src/web/html.rs` | WL badge rendering in both JS render paths |
| `rust/src/ao.rs` | Accept BSSID whitelist entries in addition to SSIDs |

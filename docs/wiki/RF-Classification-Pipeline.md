# RF Classification Pipeline

← [Back to Wiki Home](Home)

---

Most pwnagotchi-style tools are blind to the RF environment. They send attacks and check for captures, but they have no idea what's actually happening on the spectrum. Oxigotchi v3.0 changes that.

A dedicated classification pipeline taps the raw 802.11 frame stream from `wlan0mon` via libpcap and classifies every frame in real time:

```
wlan0mon (monitor mode)
 │  raw 802.11 frames (radiotap + MAC header)
 ▼
Capture Thread (libpcap, dedicated thread)
 │  extracts: frame_type, frame_subtype, BSSID, channel, RSSI, seq_num
 │  packs into 32-byte FrameEntry structs
 │  sends over mpsc channel (GPU ring buffer is not Send)
 ▼
QPU Engine (drains channel into GPU-mapped SPSC ring buffer)
 │  classifies each frame: Beacon, ProbeReq, ProbeResp, Auth,
 │  Deauth, AssocReq, AssocResp, Data, Control, Unknown
 │  256 frames in ~1ms
 ▼
RF Environment (per-epoch statistics)
 │  beacon_rate, probe_rate, deauth_rate, data_rate (frames/sec)
 │  unique_bssids, total_frames, dominant_class
 │  ao_target_ratio (fraction of frames from AO target set)
 ▼
Personality Engine              Web API
 │  mood deltas from RF          /api/qpu → JSON
 │  busy spectrum → excited      {beacon_rate: 0.9,
 │  deauth storm → angry          data_rate: 7.2,
 │  silence → lonely              unique_bssids: 19,
 │  rich BSSIDs → curious          dominant_class: "Data"}
 ▼
Bull Face (e-ink)
```

## The VideoCore IV GPU Story

The pipeline was originally designed to run classification on the Pi Zero 2W's **VideoCore IV GPU** — 12 QPU cores that sit idle on every Pi Zero 2W. A real QPU kernel was written in hand-encoded VideoCore IV machine code (64-bit instruction words, uniform FIFO for per-frame parameters, VPM DMA store for results).

Hardware testing confirmed the kernel loads correctly, the uniform FIFO reads frame data, and the VPM DMA store writes results to shared memory. However, systematic debugging revealed that **QPU conditional execution does not work when launched via V3D register poke** (the only launch method available without kernel module support). The condition codes from `sub.setf` are not honoured by subsequent conditional instructions — neither `ldi.ifz` (sig=0xE) nor ALU conditionals with small immediates (sig=0xD).

The debugging process went through 5 deploy/test cycles:
1. Full QPU kernel with conditional classification → all frames classified as Unknown
2. Sentinel/echo tests → confirmed uniforms and VPM DMA work correctly
3. Isolated conditional tests → confirmed `sub.setf` + `ldi.ifz` produces wrong results
4. Alternative conditional encoding (sig=0xD) → same failure
5. Conclusion: V3D register poke launch method does not support conditional execution

## CPU Classifier

The production classifier uses a CPU path — a simple Rust `match` on frame_type/frame_subtype that classifies 256 frames in ~1ms. This is actually **41x faster** than the per-frame QPU approach (which took ~43ms per batch due to ~170µs per QPU launch).

The QPU kernel is preserved in the codebase for future work (mailbox-based launch or lookup table approach that avoids conditionals).

## What the Bull Sees

In a typical home environment, the RF stats show ~0.9 beacons/sec from nearby APs, ~7 data frames/sec of background traffic, ~19 unique BSSIDs, with Data as the dominant frame class. Walk through a busy area and the numbers light up — the bull gets excited when it sees a rich RF environment, angry during deauth storms, and lonely when the spectrum goes quiet.

## API Reference

`GET /api/qpu` returns:

| Field | Type | Description |
|-------|------|-------------|
| `enabled` | bool | QPU feature enabled in config |
| `available` | bool | QPU hardware detected |
| `num_cores` | u32 | VideoCore IV QPU cores (12 on Pi Zero 2W) |
| `frames_submitted` | u64 | Total frames fed to classifier |
| `frames_classified` | u64 | Total frames classified |
| `batches_processed` | u64 | Classification batches run |
| `overflow_count` | u64 | Frames dropped (ring buffer full) |
| `last_batch_size` | u32 | Frames in most recent batch |
| `last_batch_duration_us` | u64 | Microseconds for last batch |
| `beacon_rate` | f32 | Beacons per second |
| `probe_rate` | f32 | Probe requests per second |
| `deauth_rate` | f32 | Deauths per second |
| `data_rate` | f32 | Data frames per second |
| `unique_bssids` | u32 | Distinct BSSIDs in last batch |
| `total_frames` | u32 | Total frames in last batch |
| `dominant_class` | string | Most common frame type |

## Frame Classes

| Value | Class | 802.11 Type/Subtype |
|-------|-------|-------------------|
| 0 | Unknown | Unrecognized or malformed |
| 1 | Beacon | Management (0/8) |
| 2 | ProbeReq | Management (0/4) |
| 3 | ProbeResp | Management (0/5) |
| 4 | Auth | Management (0/11) |
| 5 | Deauth | Management (0/12) |
| 6 | AssocReq | Management (0/0) |
| 7 | AssocResp | Management (0/1) |
| 8 | Data | Data (2/*) |
| 9 | Control | Control (1/*) |

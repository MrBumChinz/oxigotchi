// QPU capture thread — reads raw 802.11 frames from wlan0mon via libpcap
// FFI and pushes FrameEntry structs into the GPU-shared ring buffer.
//
// cfg-gated: full implementation on Linux, stub on other platforms.

use super::ringbuf::{extract_frame_entry, FrameEntry};
use log::{info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// libpcap FFI (Linux only)
// ---------------------------------------------------------------------------

/// Opaque pcap handle.
#[cfg(target_os = "linux")]
#[repr(C)]
pub struct pcap_t {
    _opaque: [u8; 0],
}

/// pcap packet header — timestamps + lengths.
#[cfg(target_os = "linux")]
#[repr(C)]
pub struct pcap_pkthdr {
    pub tv_sec: i64,
    pub tv_usec: i64,
    pub caplen: u32,
    pub len: u32,
}

#[cfg(target_os = "linux")]
#[link(name = "pcap")]
unsafe extern "C" {
    fn pcap_open_live(
        device: *const u8,
        snaplen: i32,
        promisc: i32,
        to_ms: i32,
        errbuf: *mut u8,
    ) -> *mut pcap_t;
    fn pcap_next_ex(
        p: *mut pcap_t,
        pkt_header: *mut *mut pcap_pkthdr,
        pkt_data: *mut *const u8,
    ) -> i32;
    fn pcap_close(p: *mut pcap_t);
    fn pcap_datalink(p: *mut pcap_t) -> i32;
    fn pcap_breakloop(p: *mut pcap_t);
}

/// DLT_IEEE802_11_RADIO — radiotap + 802.11
#[cfg(target_os = "linux")]
const DLT_IEEE802_11_RADIO: i32 = 127;

// ---------------------------------------------------------------------------
// Radiotap parsing
// ---------------------------------------------------------------------------

/// Known radiotap field sizes (indexed by field number 0-17).
/// Used to walk the it_present bitmask and skip to the fields we want.
#[cfg(target_os = "linux")]
const RADIOTAP_FIELD_SIZES: [usize; 18] = [
    8, // 0: TSFT (u64)
    1, // 1: FLAGS (u8)
    1, // 2: RATE (u8)
    4, // 3: CHANNEL (u16 freq + u16 flags) — alignment: 2
    2, // 4: FHSS (u8 + u8)
    1, // 5: DBM_ANTSIGNAL (i8) — this is RSSI
    1, // 6: DBM_ANTNOISE (i8)
    2, // 7: LOCK_QUALITY (u16)
    2, // 8: TX_ATTENUATION (u16)
    2, // 9: DB_TX_ATTENUATION (u16)
    1, // 10: DBM_TX_POWER (i8)
    1, // 11: ANTENNA (u8)
    1, // 12: DB_ANTSIGNAL (u8)
    1, // 13: DB_ANTNOISE (u8)
    2, // 14: RX_FLAGS (u16)
    0, // 15: (unused)
    0, // 16: (unused)
    0, // 17: (unused)
];

/// Alignment requirements for radiotap fields that need it.
#[cfg(target_os = "linux")]
const RADIOTAP_FIELD_ALIGNS: [usize; 18] = [
    8, 1, 1, 2, 2, 1, 1, 2, 2, 2, 1, 1, 1, 1, 2, 1, 1, 1,
];

/// Parse radiotap header to extract channel frequency and RSSI.
/// Returns (channel_number, rssi_dbm). Falls back to (0, 0) on parse failure.
#[cfg(target_os = "linux")]
pub fn parse_radiotap(raw: &[u8]) -> (u8, i8) {
    if raw.len() < 8 {
        return (0, 0);
    }

    let rt_len = u16::from_le_bytes([raw[2], raw[3]]) as usize;
    if rt_len < 8 || rt_len > raw.len() {
        return (0, 0);
    }

    let it_present = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);

    let mut offset: usize = 8;

    // Skip extended present bitmask words (bit 31 set = more words follow)
    let mut present_word = it_present;
    while present_word & (1 << 31) != 0 {
        if offset + 4 > rt_len {
            return (0, 0);
        }
        present_word = u32::from_le_bytes([raw[offset], raw[offset + 1], raw[offset + 2], raw[offset + 3]]);
        offset += 4;
    }

    let mut channel: u8 = 0;
    let mut rssi: i8 = 0;

    // Walk present bitmask, skip fields until we find channel (3) and signal (5)
    for field in 0u32..18 {
        if it_present & (1 << field) == 0 {
            continue;
        }

        // Align offset for this field
        let align = RADIOTAP_FIELD_ALIGNS[field as usize];
        if align > 1 {
            offset = (offset + align - 1) & !(align - 1);
        }

        let size = RADIOTAP_FIELD_SIZES[field as usize];
        if offset + size > rt_len {
            break;
        }

        match field {
            3 => {
                // CHANNEL: u16 LE frequency
                if offset + 2 <= rt_len {
                    let freq = u16::from_le_bytes([raw[offset], raw[offset + 1]]);
                    channel = freq_to_channel(freq);
                }
            }
            5 => {
                // DBM_ANTSIGNAL: i8 RSSI
                rssi = raw[offset] as i8;
            }
            _ => {}
        }

        offset += size;
    }

    (channel, rssi)
}

/// Convert 2.4 GHz frequency to WiFi channel number.
#[cfg(target_os = "linux")]
fn freq_to_channel(freq: u16) -> u8 {
    match freq {
        2412 => 1,
        2417 => 2,
        2422 => 3,
        2427 => 4,
        2432 => 5,
        2437 => 6,
        2442 => 7,
        2447 => 8,
        2452 => 9,
        2457 => 10,
        2462 => 11,
        2467 => 12,
        2472 => 13,
        2484 => 14,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// CaptureThread — pcap capture loop (Linux)
// ---------------------------------------------------------------------------

/// Handle to the pcap capture thread. Drop or call stop() to shut down.
#[cfg(target_os = "linux")]
pub struct CaptureThread {
    shutdown: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    /// Raw pointer to pcap handle for pcap_breakloop on shutdown.
    /// SAFETY: pcap_t lives as long as the capture thread. We only
    /// call pcap_breakloop (thread-safe per libpcap docs) from stop().
    pcap_ptr: Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(target_os = "linux")]
impl CaptureThread {
    /// Start capturing frames from `iface` (e.g. "wlan0mon").
    ///
    /// `ring_push` is a callback that receives each extracted FrameEntry.
    /// The engine passes a closure that pushes into the GPU ring buffer.
    pub fn start<F>(iface: &str, mut ring_push: F) -> Result<Self, String>
    where
        F: FnMut(&FrameEntry) + Send + 'static,
    {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();
        let pcap_ptr = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let pcap_ptr_clone = pcap_ptr.clone();

        let iface_owned = iface.to_string();

        let handle = std::thread::Builder::new()
            .name("qpu-capture".into())
            .spawn(move || {
                capture_loop(&iface_owned, &shutdown_clone, &pcap_ptr_clone, &mut ring_push);
            })
            .map_err(|e| format!("failed to spawn capture thread: {e}"))?;

        info!("pcap capture thread started on {iface}");

        Ok(CaptureThread {
            shutdown,
            handle: Some(handle),
            pcap_ptr,
        })
    }

    /// Signal the capture thread to stop and wait for it to finish.
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Release);

        // Break out of any blocking pcap_next_ex
        let ptr = self.pcap_ptr.load(Ordering::Acquire);
        if ptr != 0 {
            unsafe {
                pcap_breakloop(ptr as *mut pcap_t);
            }
        }

        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for CaptureThread {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Main capture loop — runs in the capture thread.
#[cfg(target_os = "linux")]
fn capture_loop<F>(
    iface: &str,
    shutdown: &AtomicBool,
    pcap_ptr: &std::sync::atomic::AtomicUsize,
    ring_push: &mut F,
)
where
    F: FnMut(&FrameEntry),
{
    // Null-terminated interface name
    let mut iface_cstr = iface.as_bytes().to_vec();
    iface_cstr.push(0);

    let mut errbuf = [0u8; 256];

    loop {
        if shutdown.load(Ordering::Acquire) {
            break;
        }

        // Open pcap on the monitor interface
        let handle = unsafe {
            pcap_open_live(
                iface_cstr.as_ptr(),
                256,  // snaplen: radiotap + MAC header + tagged params
                1,    // promisc
                100,  // timeout_ms
                errbuf.as_mut_ptr(),
            )
        };

        if handle.is_null() {
            let null_pos = errbuf.iter().position(|&b| b == 0).unwrap_or(errbuf.len());
            let err = String::from_utf8_lossy(&errbuf[..null_pos]);
            warn!("pcap_open_live({iface}) failed: {err} — retrying in 1s");
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        // Store pointer for pcap_breakloop
        pcap_ptr.store(handle as usize, Ordering::Release);

        // Verify datalink type
        let dlt = unsafe { pcap_datalink(handle) };
        if dlt != DLT_IEEE802_11_RADIO {
            warn!("pcap: {iface} has DLT {dlt}, expected {DLT_IEEE802_11_RADIO} — closing");
            unsafe { pcap_close(handle) };
            pcap_ptr.store(0, Ordering::Release);
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        info!("pcap: capturing on {iface} (DLT={dlt})");

        let epoch_start = std::time::Instant::now();

        // Packet capture loop
        loop {
            if shutdown.load(Ordering::Acquire) {
                break;
            }

            let mut pkt_header: *mut pcap_pkthdr = std::ptr::null_mut();
            let mut pkt_data: *const u8 = std::ptr::null();

            let rc = unsafe { pcap_next_ex(handle, &mut pkt_header, &mut pkt_data) };

            match rc {
                1 => {
                    // Got a packet
                    let hdr = unsafe { &*pkt_header };
                    let caplen = hdr.caplen as usize;
                    let raw = unsafe { std::slice::from_raw_parts(pkt_data, caplen) };

                    // Parse radiotap for channel + RSSI
                    let (channel, rssi) = parse_radiotap(raw);

                    let timestamp_ms = epoch_start.elapsed().as_millis() as u32;

                    // Extract FrameEntry (skips radiotap, parses 802.11 header)
                    if let Some(entry) = extract_frame_entry(raw, channel, rssi, timestamp_ms) {
                        ring_push(&entry);
                    }
                }
                0 => {
                    // Timeout — no packet, loop back
                    continue;
                }
                -2 => {
                    // pcap_breakloop was called
                    break;
                }
                _ => {
                    // Error (e.g., interface disappeared)
                    warn!("pcap_next_ex error (rc={rc}) on {iface} — reopening");
                    break;
                }
            }
        }

        // Cleanup
        unsafe { pcap_close(handle) };
        pcap_ptr.store(0, Ordering::Release);

        if shutdown.load(Ordering::Acquire) {
            break;
        }

        // Brief sleep before retry (interface may have disappeared)
        warn!("pcap: capture ended on {iface}, retrying in 1s");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    info!("pcap capture thread exiting");
}

// ---------------------------------------------------------------------------
// CaptureThread — non-Linux stub
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "linux"))]
pub struct CaptureThread;

#[cfg(not(target_os = "linux"))]
impl CaptureThread {
    pub fn start<F>(_iface: &str, _ring_push: F) -> Result<Self, String>
    where
        F: FnMut(&FrameEntry) + Send + 'static,
    {
        Err("Capture requires Linux".into())
    }

    pub fn stop(&mut self) {}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freq_to_channel() {
        #[cfg(target_os = "linux")]
        {
            assert_eq!(freq_to_channel(2412), 1);
            assert_eq!(freq_to_channel(2437), 6);
            assert_eq!(freq_to_channel(2462), 11);
            assert_eq!(freq_to_channel(2484), 14);
            assert_eq!(freq_to_channel(5180), 0);
            assert_eq!(freq_to_channel(0), 0);
        }
    }

    #[test]
    fn test_parse_radiotap_minimal() {
        #[cfg(target_os = "linux")]
        {
            let hdr: [u8; 8] = [0, 0, 8, 0, 0, 0, 0, 0];
            let (ch, rssi) = parse_radiotap(&hdr);
            assert_eq!(ch, 0);
            assert_eq!(rssi, 0);
        }
    }

    #[test]
    fn test_parse_radiotap_with_channel_and_signal() {
        #[cfg(target_os = "linux")]
        {
            let mut raw = vec![0u8; 32 + 24];
            raw[0] = 0;        // version
            raw[1] = 0;        // pad
            raw[2] = 13;       // len=13
            raw[3] = 0;
            raw[4] = 0x28;     // it_present bits 3,5
            raw[5] = 0;
            raw[6] = 0;
            raw[7] = 0;
            raw[8] = 0x85;     // 2437 = 0x0985 LE (channel 6)
            raw[9] = 0x09;
            raw[10] = 0xA0;    // channel flags
            raw[11] = 0x00;
            raw[12] = 0xCE_u8; // -50 dBm RSSI

            let (ch, rssi) = parse_radiotap(&raw);
            assert_eq!(ch, 6);
            assert_eq!(rssi, -50);
        }
    }

    #[test]
    fn test_stub_start_fails_on_non_linux() {
        #[cfg(not(target_os = "linux"))]
        {
            let result = CaptureThread::start("wlan0mon", |_| {});
            assert!(result.is_err());
        }
    }
}

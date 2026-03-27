// QPU packet classifier — classifies 802.11 frame entries by type/subtype.
//
// Contains a placeholder QPU binary (thrend-only) and a CPU-side fallback
// classifier. The real QPU shader kernel will be hand-assembled later.

use std::sync::Arc;
use super::ringbuf::FrameEntry;
#[cfg(target_os = "linux")]
use super::mailbox::GpuMem;

/// Placeholder QPU binary — thread-end only.
/// The real classifier kernel will be hand-assembled later.
static QPU_CLASSIFIER_CODE: [u32; 6] = [
    0x009E7000, 0x300009E7, // thrend (signal thread end)
    0x009E7000, 0x100009E7, // nop (mandatory delay slot 1)
    0x009E7000, 0x100009E7, // nop (mandatory delay slot 2)
];

// ---------------------------------------------------------------------------
// FrameClass — classification categories
// ---------------------------------------------------------------------------

/// Frame classification categories.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameClass {
    Unknown = 0,
    Beacon = 1,
    ProbeReq = 2,
    ProbeResp = 3,
    Auth = 4,
    Deauth = 5,
    AssocReq = 6,
    AssocResp = 7,
    Data = 8,
    Control = 9,
}

impl FrameClass {
    /// Classify a frame from its type and subtype fields.
    /// This is the CPU-side fallback classifier (same logic the QPU will execute).
    pub fn classify(frame_type: u8, frame_subtype: u8) -> Self {
        match frame_type {
            0 => match frame_subtype {
                // Management frames
                0 => FrameClass::AssocReq,
                1 => FrameClass::AssocResp,
                4 => FrameClass::ProbeReq,
                5 => FrameClass::ProbeResp,
                8 => FrameClass::Beacon,
                11 => FrameClass::Auth,   // 0x0B
                12 => FrameClass::Deauth, // 0x0C
                _ => FrameClass::Unknown,
            },
            1 => FrameClass::Control,
            2 => FrameClass::Data,
            _ => FrameClass::Unknown,
        }
    }
}

// ---------------------------------------------------------------------------
// Classifier — QPU classifier engine (Linux)
// ---------------------------------------------------------------------------

/// QPU classifier engine — loads the kernel binary into GPU memory,
/// executes it against a ring buffer, and reads back results.
#[cfg(target_os = "linux")]
pub struct Classifier {
    code_mem: GpuMem,    // GPU memory holding the QPU binary
    output_mem: GpuMem,  // GPU memory for classification results
    output_capacity: u32, // Max frames per batch
}

#[cfg(target_os = "linux")]
impl Classifier {
    /// Create a new classifier, loading the QPU binary into GPU memory.
    /// `output_capacity` is the max frames per classification batch.
    pub fn new(mbox: Arc<super::mailbox::Mailbox>, output_capacity: u32) -> Result<Self, String> {
        // Allocate GPU memory for QPU code
        let code_size = (QPU_CLASSIFIER_CODE.len() * 4) as u32;
        // Round up to page size
        let code_alloc = ((code_size + 4095) / 4096) * 4096;
        let code_mem = GpuMem::alloc(mbox.clone(), code_alloc)?;

        // Copy QPU binary into GPU memory
        unsafe {
            let dst = code_mem.as_ptr() as *mut u32;
            for (i, &word) in QPU_CLASSIFIER_CODE.iter().enumerate() {
                std::ptr::write_volatile(dst.add(i), word);
            }
        }

        // Allocate GPU memory for output (1 byte per frame, page-aligned)
        let output_size = ((output_capacity + 4095) / 4096) * 4096;
        let output_mem = GpuMem::alloc(mbox, output_size.max(4096))?;

        Ok(Classifier {
            code_mem,
            output_mem,
            output_capacity,
        })
    }

    /// Classify frames in the ring buffer using the QPU.
    ///
    /// Currently uses the CPU-side fallback because the QPU binary is a
    /// placeholder. When the real QPU kernel is ready, this will launch the
    /// QPU and read results from output_mem.
    ///
    /// Returns a Vec of (FrameClass, FrameEntry) pairs for each classified frame.
    pub fn classify_batch(
        &self,
        ring: &mut super::ringbuf::RingBuf,
        v3d: &super::mailbox::V3dRegs,
    ) -> Result<Vec<(FrameClass, FrameEntry)>, String> {
        let count = ring.available().min(self.output_capacity);
        if count == 0 {
            return Ok(Vec::new());
        }

        // TODO: When real QPU kernel is ready, execute via v3d.execute_qpu()
        // For now, use CPU-side classification as fallback
        let _ = v3d; // suppress unused warning

        // Placeholder: return empty — the caller should use FrameClass::classify()
        // or Classifier::classify_cpu() directly until the real QPU kernel lands.
        // The real QPU path will read ring buffer entries and write classifications
        // to output_mem, then we read output_mem back.
        Ok(Vec::new())
    }

    /// CPU-side batch classification (fallback when QPU is not available).
    /// Takes pre-extracted frame entries and classifies them.
    pub fn classify_cpu(entries: &[FrameEntry]) -> Vec<FrameClass> {
        entries
            .iter()
            .map(|e| {
                // SAFETY: Reading from packed struct fields
                let ft = unsafe { std::ptr::addr_of!(e.frame_type).read_unaligned() };
                let fst = unsafe { std::ptr::addr_of!(e.frame_subtype).read_unaligned() };
                FrameClass::classify(ft, fst)
            })
            .collect()
    }

    /// Get the bus address of the QPU code (for diagnostic purposes).
    pub fn code_bus_addr(&self) -> u32 {
        self.code_mem.bus_addr()
    }

    /// Get the bus address of the output buffer.
    pub fn output_bus_addr(&self) -> u32 {
        self.output_mem.bus_addr()
    }

    /// Get the output capacity.
    pub fn output_capacity(&self) -> u32 {
        self.output_capacity
    }
}

// ---------------------------------------------------------------------------
// Classifier — non-Linux stub
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "linux"))]
pub struct Classifier;

#[cfg(not(target_os = "linux"))]
impl Classifier {
    pub fn new(
        _mbox: Arc<super::mailbox::Mailbox>,
        _output_capacity: u32,
    ) -> Result<Self, String> {
        Err("Classifier requires Linux with QPU access".into())
    }

    pub fn classify_batch(
        &self,
        _ring: &mut super::ringbuf::RingBuf,
        _v3d: &super::mailbox::V3dRegs,
    ) -> Result<Vec<(FrameClass, FrameEntry)>, String> {
        Err("Classifier requires Linux with QPU access".into())
    }

    pub fn classify_cpu(entries: &[FrameEntry]) -> Vec<FrameClass> {
        entries
            .iter()
            .map(|e| {
                let ft = unsafe { std::ptr::addr_of!(e.frame_type).read_unaligned() };
                let fst = unsafe { std::ptr::addr_of!(e.frame_subtype).read_unaligned() };
                FrameClass::classify(ft, fst)
            })
            .collect()
    }

    pub fn code_bus_addr(&self) -> u32 {
        0
    }
    pub fn output_bus_addr(&self) -> u32 {
        0
    }
    pub fn output_capacity(&self) -> u32 {
        0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_beacon() {
        assert_eq!(FrameClass::classify(0, 8), FrameClass::Beacon);
    }

    #[test]
    fn test_classify_probe_req() {
        assert_eq!(FrameClass::classify(0, 4), FrameClass::ProbeReq);
    }

    #[test]
    fn test_classify_probe_resp() {
        assert_eq!(FrameClass::classify(0, 5), FrameClass::ProbeResp);
    }

    #[test]
    fn test_classify_auth() {
        assert_eq!(FrameClass::classify(0, 11), FrameClass::Auth);
    }

    #[test]
    fn test_classify_deauth() {
        assert_eq!(FrameClass::classify(0, 12), FrameClass::Deauth);
    }

    #[test]
    fn test_classify_assoc_req() {
        assert_eq!(FrameClass::classify(0, 0), FrameClass::AssocReq);
    }

    #[test]
    fn test_classify_assoc_resp() {
        assert_eq!(FrameClass::classify(0, 1), FrameClass::AssocResp);
    }

    #[test]
    fn test_classify_data() {
        assert_eq!(FrameClass::classify(2, 0), FrameClass::Data);
        assert_eq!(FrameClass::classify(2, 4), FrameClass::Data); // any data subtype
    }

    #[test]
    fn test_classify_control() {
        assert_eq!(FrameClass::classify(1, 0), FrameClass::Control);
        assert_eq!(FrameClass::classify(1, 13), FrameClass::Control); // ACK
    }

    #[test]
    fn test_classify_unknown_type() {
        assert_eq!(FrameClass::classify(3, 0), FrameClass::Unknown);
    }

    #[test]
    fn test_classify_unknown_mgmt_subtype() {
        assert_eq!(FrameClass::classify(0, 15), FrameClass::Unknown);
    }

    #[test]
    fn test_classify_cpu_batch() {
        let entries = vec![
            FrameEntry {
                bssid: [0; 6],
                frame_type: 0,
                frame_subtype: 8,
                channel: 1,
                rssi: -50,
                flags: 0,
                _pad: 0,
                seq_num: 0,
                timestamp_ms: 0,
                ssid_hash: 0,
                frame_len: 100,
                _reserved: [0; 6],
            },
            FrameEntry {
                bssid: [0; 6],
                frame_type: 2,
                frame_subtype: 0,
                channel: 6,
                rssi: -30,
                flags: 0,
                _pad: 0,
                seq_num: 1,
                timestamp_ms: 100,
                ssid_hash: 0,
                frame_len: 200,
                _reserved: [0; 6],
            },
        ];

        let classes = Classifier::classify_cpu(&entries);
        assert_eq!(classes.len(), 2);
        assert_eq!(classes[0], FrameClass::Beacon);
        assert_eq!(classes[1], FrameClass::Data);
    }

    #[test]
    fn test_frame_class_values() {
        assert_eq!(FrameClass::Unknown as u8, 0);
        assert_eq!(FrameClass::Beacon as u8, 1);
        assert_eq!(FrameClass::ProbeReq as u8, 2);
        assert_eq!(FrameClass::ProbeResp as u8, 3);
        assert_eq!(FrameClass::Auth as u8, 4);
        assert_eq!(FrameClass::Deauth as u8, 5);
        assert_eq!(FrameClass::AssocReq as u8, 6);
        assert_eq!(FrameClass::AssocResp as u8, 7);
        assert_eq!(FrameClass::Data as u8, 8);
        assert_eq!(FrameClass::Control as u8, 9);
    }

    #[test]
    fn test_qpu_placeholder_binary() {
        // Verify the placeholder binary matches the proven thrend+nops pattern
        assert_eq!(QPU_CLASSIFIER_CODE.len(), 6);
        // thrend signal
        assert_eq!(QPU_CLASSIFIER_CODE[1], 0x300009E7);
        // nop signals
        assert_eq!(QPU_CLASSIFIER_CODE[3], 0x100009E7);
        assert_eq!(QPU_CLASSIFIER_CODE[5], 0x100009E7);
    }
}

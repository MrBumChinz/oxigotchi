//! BLE connection hijack — framework stub.
//!
//! A real BLE connection hijack requires Link Layer (LL) timing knowledge
//! (connection interval, anchor point, hop increment) to inject packets
//! into an existing connection. This is a placeholder for future work.

use std::time::Instant;

use super::hci::HciSocket;
use super::{BtAttackResult, BtAttackType};

/// BLE connection hijack — framework stub.
///
/// Returns an error explaining that LL timing data is required for a real
/// hijack, which needs either a sniffer or firmware-level access to the
/// connection parameters.
pub fn run(hci: &HciSocket, target_addr: &str) -> BtAttackResult {
    let start = Instant::now();
    let _ = hci; // acknowledge parameter
    log::info!(
        "ble_hijack: framework stub for {} — needs LL timing data",
        target_addr
    );

    BtAttackResult {
        attack_type: BtAttackType::BleConnHijack,
        target_address: target_addr.to_string(),
        target_name: None,
        success: false,
        capture: None,
        error: Some(
            "BLE connection hijack requires >5KB patchram for LL hooks. BCM43430B0 has \
             1,393B largest contiguous free block at 0x212A77. Hardware-limited — \
             insufficient patchram space."
                .into(),
        ),
        timestamp: start,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn test_run_stub() {
        let hci = HciSocket::open(0).unwrap();
        let result = run(&hci, "AA:BB:CC:DD:EE:FF");
        assert_eq!(result.attack_type, BtAttackType::BleConnHijack);
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("Hardware-limited"));
    }
}

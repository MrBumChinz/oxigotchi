use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtDeviceObservation {
    pub id: String,
    pub address: String,
    pub name: Option<String>,
    pub rssi: Option<i16>,
    pub ts: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BtDiscoveryObservation {
    ScanStarted,
    ScanStopped,
    DeviceSeen(BtDeviceObservation),
    DeviceLost { id: String, ts: DateTime<Utc> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BtControllerObservation {
    ControllerPresent {
        ts: DateTime<Utc>,
    },
    ControllerMissing {
        ts: DateTime<Utc>,
    },
    ProbeResult {
        probe_name: String,
        ok: bool,
        detail: Option<String>,
        ts: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RfObservation {
    WifiScanStarted {
        ts: DateTime<Utc>,
    },
    WifiScanStopped {
        ts: DateTime<Utc>,
    },
    WifiError {
        detail: Option<String>,
        ts: DateTime<Utc>,
    },
    BtDiscoveryStarted {
        ts: DateTime<Utc>,
    },
    BtDiscoveryStopped {
        ts: DateTime<Utc>,
    },
}

use chrono::{DateTime, Utc};

use crate::bluetooth::model::observation::RfObservation;

#[derive(Debug, Clone, Default)]
pub struct WifiBtCoexAdapter;

impl WifiBtCoexAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn wifi_scan_started(&self, ts: DateTime<Utc>) -> RfObservation {
        RfObservation::WifiScanStarted { ts }
    }
}

use chrono::{DateTime, Utc};

use crate::bluetooth::model::observation::RfObservation;
use crate::bluetooth::model::state::BtCoexState;

#[derive(Debug, Default)]
pub struct BtCoexWorker {
    wifi_scan_started_at: Option<DateTime<Utc>>,
    bt_discovery_started_at: Option<DateTime<Utc>>,
    state: BtCoexState,
}

impl BtCoexWorker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, observation: RfObservation) -> &BtCoexState {
        match observation {
            RfObservation::WifiScanStarted { ts } => self.wifi_scan_started_at = Some(ts),
            RfObservation::WifiScanStopped { .. } => self.wifi_scan_started_at = None,
            RfObservation::BtDiscoveryStarted { ts } => self.bt_discovery_started_at = Some(ts),
            RfObservation::BtDiscoveryStopped { .. } => self.bt_discovery_started_at = None,
            RfObservation::WifiError { .. } => {
                self.state.contention_score = self.state.contention_score.saturating_add(10);
            }
        }

        self.state.overlap_active =
            self.wifi_scan_started_at.is_some() && self.bt_discovery_started_at.is_some();
        if self.state.overlap_active {
            self.state.contention_score = self.state.contention_score.max(1);
        }
        &self.state
    }
}

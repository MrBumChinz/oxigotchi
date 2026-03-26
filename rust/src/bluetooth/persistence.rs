use crate::bluetooth::model::state::{BtControllerState, BtSummaryState};

#[derive(Debug, Default)]
pub struct BtPersistence;

impl BtPersistence {
    pub fn new() -> Self {
        Self
    }

    pub fn summarize(&self, summary: &BtSummaryState, controller: &BtControllerState) -> String {
        format!(
            "bt.devices_now={} bt.strongest_rssi_recent={:?} bt.last_probe_status={:?}",
            summary.devices_now, summary.strongest_rssi_recent, controller.last_probe_status
        )
    }
}

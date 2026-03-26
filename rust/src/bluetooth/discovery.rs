use std::collections::HashMap;

use crate::bluetooth::model::observation::{BtDeviceObservation, BtDiscoveryObservation};
use crate::bluetooth::model::state::BtSummaryState;

#[derive(Debug, Default)]
pub struct BtDiscoveryWorker {
    devices: HashMap<String, BtDeviceObservation>,
}

impl BtDiscoveryWorker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.devices.clear();
    }

    pub fn apply(&mut self, observation: BtDiscoveryObservation) -> BtSummaryState {
        match observation {
            BtDiscoveryObservation::DeviceSeen(device) => {
                self.devices.insert(device.id.clone(), device);
            }
            BtDiscoveryObservation::DeviceLost { id, .. } => {
                self.devices.remove(&id);
            }
            BtDiscoveryObservation::ScanStarted | BtDiscoveryObservation::ScanStopped => {}
        }

        let strongest = self.devices.values().filter_map(|d| d.rssi).max();
        BtSummaryState {
            devices_now: self.devices.len() as u32,
            strongest_rssi_recent: strongest,
        }
    }

    pub fn summary(&self) -> BtSummaryState {
        let strongest = self.devices.values().filter_map(|d| d.rssi).max();
        BtSummaryState {
            devices_now: self.devices.len() as u32,
            strongest_rssi_recent: strongest,
        }
    }
}

use crate::bluetooth::model::observation::{BtDeviceObservation, BtDiscoveryObservation};

pub struct BluezDiscoveryAdapter;

impl BluezDiscoveryAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn normalize_seen(&self, device: BtDeviceObservation) -> BtDiscoveryObservation {
        BtDiscoveryObservation::DeviceSeen(device)
    }
}

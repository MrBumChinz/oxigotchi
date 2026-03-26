use chrono::{DateTime, Utc};

use crate::bluetooth::model::observation::BtControllerObservation;

#[derive(Debug, Clone, Default)]
pub struct ControllerInfoAdapter;

impl ControllerInfoAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn present(&self, ts: DateTime<Utc>) -> BtControllerObservation {
        BtControllerObservation::ControllerPresent { ts }
    }
}

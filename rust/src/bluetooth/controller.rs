use chrono::Utc;

use crate::bluetooth::model::observation::BtControllerObservation;
use crate::bluetooth::model::state::BtControllerState;

#[derive(Debug, Default)]
pub struct BtControllerWorker {
    state: BtControllerState,
}

impl BtControllerWorker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, observation: BtControllerObservation) -> &BtControllerState {
        match observation {
            BtControllerObservation::ControllerPresent { ts } => {
                self.state.last_snapshot_at = Some(ts);
                self.state.last_probe_status = Some("present".into());
            }
            BtControllerObservation::ControllerMissing { ts } => {
                self.state.last_snapshot_at = Some(ts);
                self.state.last_probe_status = Some("missing".into());
            }
            BtControllerObservation::ProbeResult {
                probe_name, ok, ts, ..
            } => {
                self.state.last_snapshot_at = Some(ts);
                self.state.last_probe_status =
                    Some(format!("{probe_name}:{}", if ok { "ok" } else { "fail" }));
            }
        }
        &self.state
    }

    pub fn snapshot_now(&mut self, status: &str) -> &BtControllerState {
        self.state.last_snapshot_at = Some(Utc::now());
        self.state.last_probe_status = Some(status.to_string());
        &self.state
    }
}

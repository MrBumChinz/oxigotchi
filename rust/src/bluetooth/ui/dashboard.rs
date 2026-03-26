use crate::bluetooth::model::state::BtFeatureState;

#[derive(Debug, Clone, Default)]
pub struct BtDashboardSummary {
    pub mode: String,
    pub devices_now: u32,
    pub strongest_rssi_recent: Option<i16>,
    pub contention_score: u32,
}

impl From<&BtFeatureState> for BtDashboardSummary {
    fn from(state: &BtFeatureState) -> Self {
        Self {
            mode: format!("{:?}", state.mode),
            devices_now: state.summary.devices_now,
            strongest_rssi_recent: state.summary.strongest_rssi_recent,
            contention_score: state.coex.contention_score,
        }
    }
}

use crate::bluetooth::model::state::BtFeatureState;

#[derive(Debug, Clone, Default)]
pub struct BtEinkSummary {
    pub headline: String,
    pub detail: String,
}

impl From<&BtFeatureState> for BtEinkSummary {
    fn from(state: &BtFeatureState) -> Self {
        Self {
            headline: format!("BT {:?}", state.mode),
            detail: format!("{} dev", state.summary.devices_now),
        }
    }
}

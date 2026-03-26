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

/// Data needed to render the BT mode e-ink display.
#[derive(Debug, Clone, Default)]
pub struct BtEinkData {
    pub devices: u32,
    pub active_attacks: u32,
    pub captures: u32,
    pub patchram_state: String,
}

/// Collect BT subsystem stats into a struct ready for e-ink rendering.
pub fn prepare_eink_data(
    discovery: &crate::bluetooth::discovery::BtDiscoveryWorker,
    scheduler: &crate::bluetooth::attacks::BtAttackScheduler,
    captures: &crate::bluetooth::capture::BtCaptureManager,
    patchram: &crate::bluetooth::patchram::PatchramManager,
) -> BtEinkData {
    let summary = discovery.summary();
    BtEinkData {
        devices: summary.devices_now,
        active_attacks: scheduler.active_count(),
        captures: captures.total_captures(),
        patchram_state: patchram.state.as_str().to_string(),
    }
}

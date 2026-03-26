use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::config::BtMode;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BtHealthState {
    pub stack_up: bool,
    pub controller_present: bool,
    pub degraded: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BtSummaryState {
    pub devices_now: u32,
    pub strongest_rssi_recent: Option<i16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BtControllerState {
    pub last_snapshot_at: Option<DateTime<Utc>>,
    pub last_probe_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BtCoexState {
    pub overlap_active: bool,
    pub overlap_duration_ms: u64,
    pub contention_score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtFeatureState {
    pub mode: BtMode,
    pub health: BtHealthState,
    pub summary: BtSummaryState,
    pub controller: BtControllerState,
    pub coex: BtCoexState,
}

impl Default for BtFeatureState {
    fn default() -> Self {
        Self {
            mode: BtMode::Off,
            health: BtHealthState::default(),
            summary: BtSummaryState::default(),
            controller: BtControllerState::default(),
            coex: BtCoexState::default(),
        }
    }
}

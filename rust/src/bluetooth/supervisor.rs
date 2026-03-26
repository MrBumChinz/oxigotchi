use crate::bluetooth::model::config::{BtFeatureConfig, BtMode};
use crate::bluetooth::model::state::{BtFeatureState, BtHealthState};

#[derive(Debug)]
pub struct BtSupervisor {
    pub config: BtFeatureConfig,
    pub state: BtFeatureState,
}

impl BtSupervisor {
    pub fn new(config: BtFeatureConfig) -> Self {
        let state = BtFeatureState {
            mode: if config.enabled {
                config.mode.clone()
            } else {
                BtMode::Off
            },
            ..BtFeatureState::default()
        };
        Self { config, state }
    }

    pub fn set_mode(&mut self, mode: BtMode) {
        self.state.mode = if self.config.enabled {
            mode
        } else {
            BtMode::Off
        };
    }

    pub fn mark_degraded(&mut self, error: impl Into<String>) {
        self.state.health = BtHealthState {
            degraded: true,
            last_error: Some(error.into()),
            ..self.state.health.clone()
        };
    }
}

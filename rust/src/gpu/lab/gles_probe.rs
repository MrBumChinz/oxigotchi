#[derive(Debug, Clone, Default)]
pub struct GlesProbeConfig {
    pub enabled: bool,
    pub require_explicit_flag: bool,
}

impl GlesProbeConfig {
    pub fn can_run(&self) -> bool {
        self.enabled && self.require_explicit_flag
    }
}

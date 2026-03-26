#[derive(Debug, Clone, Default)]
pub struct WorkBatcher {
    pub queued: u32,
}

impl WorkBatcher {
    pub fn push(&mut self) {
        self.queued = self.queued.saturating_add(1);
    }

    pub fn drain(&mut self) -> u32 {
        let out = self.queued;
        self.queued = 0;
        out
    }
}

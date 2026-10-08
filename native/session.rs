//! Invalidation gate shared by hotkey/UI orchestration and worker completion.
#[derive(Debug, Default)]
pub struct Sessions {
    generation: u64,
    active: Option<u64>,
    paused: bool,
}

impl Sessions {
    pub fn begin(&mut self) -> Option<u64> {
        if self.paused || self.active.is_some() {
            return None;
        }
        self.generation = self.generation.checked_add(1)?;
        self.active = Some(self.generation);
        self.active
    }

    /// Atomically consume the active result before attempting insertion.
    pub fn accept(&mut self, id: u64) -> bool {
        if self.active == Some(id) && !self.paused {
            self.active = None;
            true
        } else {
            false
        }
    }

    pub fn cancel(&mut self) {
        self.active = None;
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        if paused {
            self.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escape_rejects_a_late_result_even_after_next_session_starts() {
        let mut sessions = Sessions::default();
        let old = sessions.begin().unwrap();
        sessions.cancel();
        let new = sessions.begin().unwrap();
        assert!(!sessions.accept(old));
        assert!(sessions.accept(new));
        assert!(!sessions.accept(new));
    }
    #[test]
    fn pause_invalidates_work_and_prevents_start() {
        let mut sessions = Sessions::default();
        let id = sessions.begin().unwrap();
        sessions.set_paused(true);
        assert!(!sessions.accept(id));
        assert!(sessions.begin().is_none());
        sessions.set_paused(false);
        assert!(sessions.begin().is_some());
    }
}

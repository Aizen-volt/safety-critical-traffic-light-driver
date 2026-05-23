use crate::types::{Mode, Phase, PreemptState};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LogEvent {
    TopStateChanged,
    ModeChanged { from: Mode, to: Mode },
    PhaseChanged { from: Phase, to: Phase },
    PreemptChanged { from: PreemptState, to: PreemptState },
    InvalidEmergencyRequest,
    PreemptTimeout,
    PedDemandLatched { crossing_index: usize },
    PedDemandCleared { crossing_index: usize },
}

pub const LOG_CAPACITY: usize = 16;

#[derive(Debug, Copy, Clone)]
pub struct LogBuffer {
    events: [Option<LogEvent>; LOG_CAPACITY],
    len: usize,
    overflow: bool,
}

impl LogBuffer {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            events: [None; LOG_CAPACITY],
            len: 0,
            overflow: false,
        }
    }

    pub fn push(&mut self, event: LogEvent) {
        if let Some(slot) = self.events.get_mut(self.len) {
            *slot = Some(event);
            self.len = self.len.saturating_add(1);
        } else {
            self.overflow = true;
        }
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub const fn overflowed(&self) -> bool {
        self.overflow
    }

    pub fn iter(&self) -> impl Iterator<Item = &LogEvent> {
        self.events.iter().take(self.len).filter_map(Option::as_ref)
    }

    pub fn clear(&mut self) {
        self.events = [None; LOG_CAPACITY];
        self.len = 0;
        self.overflow = false;
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}
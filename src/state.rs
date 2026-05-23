use crate::config::NUM_LEGS;
use crate::types::{Mode, Phase, PreemptState, TopState};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct State {
    pub top: TopState,

    /// Operating mode inside Operational
    pub mode: Mode,

    /// Current phase inside Auto
    pub phase: Phase,

    /// State of the preemption region
    pub preempt: PreemptState,

    /// Latched pedestrian requests for crossings
    pub ped_demand: [bool; NUM_LEGS],

    /// Steps passed since the preemption became active
    pub preempt_steps: u16,

    /// Previous-step value of `ped_button`
    pub prev_ped_button: [bool; NUM_LEGS],

    /// Steps passed in the current phase
    pub phase_steps: u8,
}

impl State {
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            top: TopState::Off,
            mode: Mode::Auto,
            phase: Phase::GreenNs,
            preempt: PreemptState::NoPreempt,
            ped_demand: [false; NUM_LEGS],
            preempt_steps: 0,
            prev_ped_button: [false; NUM_LEGS],
            phase_steps: 0,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::initial()
    }
}
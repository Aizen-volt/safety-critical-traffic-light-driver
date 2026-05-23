use crate::config::{NUM_LEGS, NUM_VEHICLE_GROUPS};
use crate::types::Mode;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Input {
    /// Vehicle presence on the induction loop
    pub vehicle_present: [bool; NUM_LEGS],

    /// Pedestrian request button state at each crossing
    pub ped_button: [bool; NUM_LEGS],

    /// Operator key switch position
    pub operator_switch: Mode,

    /// Aggregate fault signal from the critical-lamp burnout detector
    pub lamp_fault: bool,

    /// Main power present
    pub power_ok: bool,

    /// Preemption request from the dispatch system
    pub emergency_request: [bool; NUM_VEHICLE_GROUPS],

    /// Manual service-reset info
    pub service_reset: bool,
}

impl Input {
    #[must_use]
    pub const fn idle() -> Self {
        Self {
            vehicle_present: [false; NUM_LEGS],
            ped_button: [false; NUM_LEGS],
            operator_switch: Mode::Auto,
            lamp_fault: false,
            power_ok: true,
            emergency_request: [false; NUM_VEHICLE_GROUPS],
            service_reset: false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct InputValidation {
    pub conflicting_emergency_request: bool,
}

impl InputValidation {
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        !self.conflicting_emergency_request
    }
}

#[must_use]
pub const fn validate(input: &Input) -> InputValidation {
    let [req_ns, req_ew] = input.emergency_request;
    InputValidation {
        conflicting_emergency_request: req_ns && req_ew,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_input_is_clean() {
        let input = Input::idle();
        let validation = validate(&input);
        assert!(validation.is_clean());
    }

    #[test]
    fn conflicting_emergency_request_is_flagged() {
        let mut input = Input::idle();
        input.emergency_request = [true, true];
        let validation = validate(&input);
        assert!(validation.conflicting_emergency_request);
        assert!(!validation.is_clean());
    }

    #[test]
    fn single_direction_emergency_request_is_clean() {
        let mut input = Input::idle();
        input.emergency_request = [true, false];
        assert!(validate(&input).is_clean());

        input.emergency_request = [false, true];
        assert!(validate(&input).is_clean());
    }
}
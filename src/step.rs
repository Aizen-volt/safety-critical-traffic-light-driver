use crate::config::{
    MIN_ALLRED_STEPS, MIN_GREEN_STEPS, MIN_YELLOW_STEPS, NUM_LEGS, PREEMPT_MAX_STEPS,
};
use crate::input::{validate, Input};
use crate::log::{LogBuffer, LogEvent};
use crate::output::{output_for, Output};
use crate::state::State;
use crate::types::{Mode, PedColor, Phase, PreemptState, TopState};

#[must_use]
pub fn step(state: State, input: Input) -> (State, Output, LogBuffer) {
    let mut log = LogBuffer::new();

    let validation = validate(&input);
    if validation.conflicting_emergency_request {
        log.push(LogEvent::InvalidEmergencyRequest);
    }

    let mut next = state;

    let prev_top = next.top;
    next = advance_top_state(next, &input);
    if next.top != prev_top {
        log.push(LogEvent::TopStateChanged);
    }

    if matches!(next.top, TopState::Initializing) && input.lamp_fault {
        log.push(LogEvent::InitSelfTestFailed);
    }

    if !matches!(next.top, TopState::Operational) {
        let output = output_for(next.top, next.mode, next.phase);
        return (next, output, log);
    }

    let prev_ped_demand = next.ped_demand;
    next = latch_ped_demand_rising_edge(next, &input);

    let prev_preempt = next.preempt;
    let prev_preempt_steps = next.preempt_steps;
    next = update_preempt_region(next, &input);
    if next.preempt != prev_preempt {
        log.push(LogEvent::PreemptChanged {
            from: prev_preempt,
            to: next.preempt,
        });
    }
    if prev_preempt_steps <= PREEMPT_MAX_STEPS && next.preempt_steps > PREEMPT_MAX_STEPS {
        log.push(LogEvent::PreemptTimeout);
    }

    let prev_mode = next.mode;
    next = evaluate_emergency_transition(next, &input);
    if next.mode != prev_mode {
        log.push(LogEvent::ModeChanged {
            from: prev_mode,
            to: next.mode,
        });
    }

    let prev_phase = next.phase;
    next = advance_phase(next, &input);
    if next.phase != prev_phase {
        log.push(LogEvent::PhaseChanged {
            from: prev_phase,
            to: next.phase,
        });
    }

    next = update_phase_steps(next, prev_phase);

    let output = output_for(next.top, next.mode, next.phase);

    next = clear_ped_demand_on_green(next, &output);
    record_ped_demand_changes(&prev_ped_demand, &next.ped_demand, &mut log);

    (next, output, log)
}

fn advance_top_state(mut state: State, input: &Input) -> State {
    match state.top {
        TopState::Off => {
            if input.power_ok {
                state.top = TopState::Initializing;
            }
        }
        TopState::Initializing => {
            if !input.power_ok {
                state.top = TopState::Off;
            } else if !input.lamp_fault && matches!(input.operator_switch, Mode::Auto) {
                state.top = TopState::Operational;
                state.reset_to_clean_auto();
            }
        }
        TopState::Operational => {
            if !input.power_ok {
                state.top = TopState::Off;
            }
        }
    }
    state
}

fn latch_ped_demand_rising_edge(mut state: State, input: &Input) -> State {
    let buttons_now = input.ped_button;
    let buttons_prev = state.prev_ped_button;

    for ((demand, &button_now), &button_prev) in state
        .ped_demand
        .iter_mut()
        .zip(buttons_now.iter())
        .zip(buttons_prev.iter())
    {
        if button_now && !button_prev {
            *demand = true;
        }
    }

    state.prev_ped_button = input.ped_button;
    state
}

fn clear_ped_demand_on_green(mut state: State, output: &Output) -> State {
    let ped_colors = output.ped_signal;

    for (demand, &color) in state.ped_demand.iter_mut().zip(ped_colors.iter()) {
        if matches!(color, PedColor::Green) {
            *demand = false;
        }
    }
    state
}

fn record_ped_demand_changes(
    prev: &[bool; NUM_LEGS],
    curr: &[bool; NUM_LEGS],
    log: &mut LogBuffer,
) {
    for (index, (&p, &c)) in prev.iter().zip(curr.iter()).enumerate() {
        if !p && c {
            log.push(LogEvent::PedDemandLatched { crossing_index: index });
        } else if p && !c {
            log.push(LogEvent::PedDemandCleared { crossing_index: index });
        }
    }
}

fn update_preempt_region(mut state: State, input: &Input) -> State {
    let [req_ns, req_ew] = input.emergency_request;
    state.preempt = match (req_ns, req_ew) {
        (false, false) => {
            state.preempt_steps = 0;
            PreemptState::NoPreempt
        }
        (true, false) => {
            state.preempt_steps = state.preempt_steps.saturating_add(1);
            PreemptState::PreemptNs
        }
        (false, true) => {
            state.preempt_steps = state.preempt_steps.saturating_add(1);
            PreemptState::PreemptEw
        }
        (true, true) => {
            state.preempt_steps = 0;
            PreemptState::NoPreempt
        }
    };
    state
}

fn evaluate_emergency_transition(mut state: State, input: &Input) -> State {
    match state.mode {
        Mode::Auto => {
            let fault = input.lamp_fault
                || !input.power_ok
                || matches!(input.operator_switch, Mode::Emergency)
                || state.preempt_steps > PREEMPT_MAX_STEPS;
            if fault {
                state.mode = Mode::Emergency;
            }
        }
        Mode::Emergency => {
            let can_exit = input.service_reset
                && !input.lamp_fault
                && input.power_ok
                && matches!(input.operator_switch, Mode::Auto);
            if can_exit {
                state.mode = Mode::Auto;
                state.reset_to_clean_auto();
            }
        }
    }
    state
}

fn demand_ns(state: &State, input: &Input) -> bool {
    let [vp_n, _vp_e, vp_s, _vp_w] = input.vehicle_present;
    let [_pd_n, pd_e, _pd_s, pd_w] = state.ped_demand;
    vp_n || vp_s || pd_e || pd_w
}

fn demand_ew(state: &State, input: &Input) -> bool {
    let [_vp_n, vp_e, _vp_s, vp_w] = input.vehicle_present;
    let [pd_n, _pd_e, pd_s, _pd_w] = state.ped_demand;
    vp_e || vp_w || pd_n || pd_s
}

fn advance_phase(mut state: State, input: &Input) -> State {
    if matches!(state.mode, Mode::Emergency) {
        return state;
    }

    let d_ns = demand_ns(&state, input);
    let d_ew = demand_ew(&state, input);
    let min_green_reached = state.phase_steps >= MIN_GREEN_STEPS;
    let min_yellow_reached = state.phase_steps >= MIN_YELLOW_STEPS;
    let min_allred_reached = state.phase_steps >= MIN_ALLRED_STEPS;

    // Preempt for this direction holds green
    // Preempt for the other direction forces change immediately
    let should_yield_ns = matches!(state.preempt, PreemptState::PreemptEw)
        || (min_green_reached && d_ew && !matches!(state.preempt, PreemptState::PreemptNs));
    let should_yield_ew = matches!(state.preempt, PreemptState::PreemptNs)
        || (min_green_reached && d_ns && !matches!(state.preempt, PreemptState::PreemptEw));

    state.phase = match state.phase {
        Phase::GreenNs => {
            if should_yield_ns {
                Phase::YellowNs
            } else {
                Phase::GreenNs
            }
        }
        Phase::YellowNs => {
            if min_yellow_reached {
                Phase::AllRed1
            } else {
                Phase::YellowNs
            }
        }
        Phase::AllRed1 => {
            if min_allred_reached {
                if matches!(state.preempt, PreemptState::PreemptNs) {
                    Phase::GreenNs
                } else {
                    Phase::GreenEw
                }
            } else {
                Phase::AllRed1
            }
        }
        Phase::GreenEw => {
            if should_yield_ew {
                Phase::YellowEw
            } else {
                Phase::GreenEw
            }
        }
        Phase::YellowEw => {
            if min_yellow_reached {
                Phase::AllRed2
            } else {
                Phase::YellowEw
            }
        }
        Phase::AllRed2 => {
            if min_allred_reached {
                if matches!(state.preempt, PreemptState::PreemptEw) {
                    Phase::GreenEw
                } else {
                    Phase::GreenNs
                }
            } else {
                Phase::AllRed2
            }
        }
    };
    state
}

fn update_phase_steps(mut state: State, prev_phase: Phase) -> State {
    if matches!(state.mode, Mode::Emergency) {
        state.phase_steps = 0;
        return state;
    }
    if state.phase == prev_phase {
        state.phase_steps = state.phase_steps.saturating_add(1);
    } else {
        state.phase_steps = 1;
    }
    state
}
#![allow(clippy::disallowed_types, clippy::indexing_slicing)]

use tlc4::config::{MIN_ALLRED_STEPS, MIN_GREEN_STEPS, MIN_YELLOW_STEPS};
use tlc4::input::Input;
use tlc4::log::LogBuffer;
use tlc4::output::Output;
use tlc4::state::State;
use tlc4::step;
use tlc4::types::{PedColor, Phase, PreemptState, VehicleColor};

fn main() {
    println!("==============================================================");
    println!(" MIN_GREEN={MIN_GREEN_STEPS}  MIN_YELLOW={MIN_YELLOW_STEPS}  MIN_ALLRED={MIN_ALLRED_STEPS}");
    println!("==============================================================");

    scenario_powerup_and_idle_cycle();
    scenario_pedestrian_button_on_north();
    scenario_preemption_for_ew();
    scenario_lamp_fault_to_emergency_and_reset();
    scenario_invalid_emergency_request_is_rejected();
}

// ----------------------------------------------------------------------
// Scenario 1: power up then run with vehicle demand on EW
// ----------------------------------------------------------------------
fn scenario_powerup_and_idle_cycle() {
    print_header("1. Power up and first full phase transition");

    let mut state = State::initial();
    let mut input = Input::idle();

    let (s, o, log) = step(state, input);
    print_step(1, &input, &s, &o, &log);
    state = s;

    let (s, o, log) = step(state, input);
    print_step(2, &input, &s, &o, &log);
    state = s;

    input.vehicle_present = [false, true, false, true];

    let mut step_no = 3_u32;
    loop {
        let (s, o, log) = step(state, input);
        if step_no <= 4 || matches!(s.phase, Phase::YellowNs | Phase::AllRed1 | Phase::GreenEw)
            && s.phase != state.phase
        {
            print_step(step_no, &input, &s, &o, &log);
        }
        state = s;
        if matches!(state.phase, Phase::GreenEw) {
            break;
        }
        step_no = step_no.saturating_add(1);
        if step_no > 200 {
            println!("Stopped after 200 steps without reaching Green_EW");
            break;
        }
    }
}

// ----------------------------------------------------------------------
// Scenario 2: pedestrian button at crossing N.
// ----------------------------------------------------------------------
fn scenario_pedestrian_button_on_north() {
    print_header("2. Pedestrian button on crossing N");

    let (state, _o, _l) = bring_to_operational();
    let mut state = state;
    let mut input = Input::idle();

    input.ped_button = [true, false, false, false];
    let (s, o, log) = step(state, input);
    print_step(0, &input, &s, &o, &log);
    state = s;

    let (s, o, log) = step(state, input);
    print_step(1, &input, &s, &o, &log);
    state = s;

    input.ped_button = [false; 4];

    let mut step_no = 2_u32;
    loop {
        let (s, o, log) = step(state, input);
        let phase_changed = s.phase != state.phase;
        let cleared = state.ped_demand[0] && !s.ped_demand[0];
        if phase_changed || cleared {
            print_step(step_no, &input, &s, &o, &log);
        }
        state = s;
        if cleared {
            break;
        }
        step_no = step_no.saturating_add(1);
        if step_no > 250 {
            println!("Stopped after 250 steps without clearing ped N demand");
            break;
        }
    }
}

// ----------------------------------------------------------------------
// Scenario 3: emergency_request for EW direction.
// ----------------------------------------------------------------------
fn scenario_preemption_for_ew() {
    print_header("3. Preemption request for EW");

    let (state, _o, _l) = bring_to_operational();
    let mut state = state;
    let mut input = Input::idle();
    input.emergency_request = [false, true];

    let mut step_no = 0_u32;
    loop {
        let (s, o, log) = step(state, input);
        let phase_changed = s.phase != state.phase;
        let preempt_changed = s.preempt != state.preempt;
        if phase_changed || preempt_changed {
            print_step(step_no, &input, &s, &o, &log);
        }
        state = s;
        if matches!(state.phase, Phase::GreenEw) {
            break;
        }
        step_no = step_no.saturating_add(1);
        if step_no > 200 {
            println!("Stopped after 200 steps without reaching Green_EW");
            break;
        }
    }
}

// ----------------------------------------------------------------------
// Scenario 4: lamp_fault -> Emergency, then service_reset back to Auto
// ----------------------------------------------------------------------
fn scenario_lamp_fault_to_emergency_and_reset() {
    print_header("4. Lamp fault -> Emergency, then recovery via service reset");

    let (state, _o, _l) = bring_to_operational();
    let mut state = state;
    let mut input = Input::idle();

    input.lamp_fault = true;
    let (s, o, log) = step(state, input);
    print_step(1, &input, &s, &o, &log);
    state = s;

    input.service_reset = true;
    let (s, o, log) = step(state, input);
    print_step(2, &input, &s, &o, &log);
    state = s;

    input.lamp_fault = false;
    input.service_reset = true;
    let (s, o, log) = step(state, input);
    print_step(3, &input, &s, &o, &log);
}

// ----------------------------------------------------------------------
// Scenario 5: emergency_request = [true, true].
// ----------------------------------------------------------------------
fn scenario_invalid_emergency_request_is_rejected() {
    print_header("5. Invalid emergency_request = [true, true] is rejected");

    let (state, _o, _l) = bring_to_operational();
    let mut input = Input::idle();
    input.emergency_request = [true, true];

    let (s, o, log) = step(state, input);
    print_step(1, &input, &s, &o, &log);
    assert_eq!(s.preempt, PreemptState::NoPreempt);
}


fn bring_to_operational() -> (State, Output, LogBuffer) {
    let state = State::initial();
    let input = Input::idle();
    let (state, _o, _l) = step(state, input);
    step(state, input)
}

fn print_header(title: &str) {
    println!();
    println!("--------------------------------------------------------------");
    println!(" Scenario {title}");
    println!("--------------------------------------------------------------");
}

fn print_step(step_no: u32, input: &Input, state: &State, output: &Output, log: &LogBuffer) {
    println!("[step {step_no}]");
    println!("  in : veh={:?}  ped_btn={:?}  emerg_req={:?}  lamp_fault={}  power_ok={}  op_sw={:?}  service_reset={}",
             input.vehicle_present,
             input.ped_button,
             input.emergency_request,
             input.lamp_fault,
             input.power_ok,
             input.operator_switch,
             input.service_reset
    );
    println!("  st : top={:?}  mode={:?}  phase={:?}  phase_steps={}  preempt={:?}  preempt_steps={}  ped_dem={:?}",
             state.top, state.mode, state.phase, state.phase_steps,
             state.preempt, state.preempt_steps, state.ped_demand,
    );
    println!("  out: veh={}  ped={}",
             fmt_vehicle(&output.vehicle_signal),
             fmt_ped(&output.ped_signal),
    );
    if !log.is_empty() {
        print!("  log:");
        for event in log.iter() {
            print!(" {event:?};");
        }
        println!();
    }
    if log.overflowed() {
        println!("  log: OVERFLOW");
    }
}

fn fmt_vehicle(s: &[VehicleColor; 2]) -> String {
    format!(
        "[NS={}  EW={}]",
        vehicle_color_str(s[0]),
        vehicle_color_str(s[1]),
    )
}

fn fmt_ped(s: &[PedColor; 4]) -> String {
    format!(
        "[N={} E={} S={} W={}]",
        ped_color_str(s[0]),
        ped_color_str(s[1]),
        ped_color_str(s[2]),
        ped_color_str(s[3]),
    )
}

fn vehicle_color_str(c: VehicleColor) -> &'static str {
    match c {
        VehicleColor::Red => "R",
        VehicleColor::Yellow => "Y",
        VehicleColor::Green => "G",
        VehicleColor::YellowFlashing => "F",
        VehicleColor::Off => "-",
    }
}

fn ped_color_str(c: PedColor) -> &'static str {
    match c {
        PedColor::Red => "R",
        PedColor::Green => "G",
        PedColor::GreenFlashing => "f",
        PedColor::Off => "-",
    }
}
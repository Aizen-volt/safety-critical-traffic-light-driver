#![allow(
    clippy::disallowed_types,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use tlc4::config::{MIN_ALLRED_STEPS, MIN_GREEN_STEPS, MIN_YELLOW_STEPS, PREEMPT_MAX_STEPS};
use tlc4::input::Input;
use tlc4::log::LogEvent;
use tlc4::output::{output_for, Output};
use tlc4::state::State;
use tlc4::step;
use tlc4::types::{Mode, PedColor, Phase, PreemptState, TopState, VehicleColor};

// ----------------------------------------------------------------------
// helpers
// ----------------------------------------------------------------------

fn operational() -> State {
    let (s, _, _) = step(State::initial(), Input::idle());
    let (s, _, _) = step(s, Input::idle());
    s
}

fn emergency() -> State {
    let mut input = Input::idle();
    input.lamp_fault = true;
    let (s, _, _) = step(operational(), input);
    s
}

const PHASES: [Phase; 6] = [
    Phase::GreenNs,
    Phase::YellowNs,
    Phase::AllRed1,
    Phase::GreenEw,
    Phase::YellowEw,
    Phase::AllRed2,
];

fn ns_green(o: &Output) -> bool {
    o.vehicle_signal[0] == VehicleColor::Green
}

fn ew_green(o: &Output) -> bool {
    o.vehicle_signal[1] == VehicleColor::Green
}

fn bits4(mask: u8) -> [bool; 4] {
    [
        mask & 1 != 0,
        mask & 2 != 0,
        mask & 4 != 0,
        mask & 8 != 0,
    ]
}

fn has_event(log: &tlc4::log::LogBuffer, wanted: LogEvent) -> bool {
    log.iter().any(|e| *e == wanted)
}

fn check_vehicle_transition(before: &Output, after: &Output) {
    for i in 0..2 {
        match before.vehicle_signal[i] {
            VehicleColor::Green => assert!(matches!(
                after.vehicle_signal[i],
                VehicleColor::Green | VehicleColor::Yellow
            )),
            VehicleColor::Yellow => assert!(matches!(
                after.vehicle_signal[i],
                VehicleColor::Yellow | VehicleColor::Red
            )),
            _ => {}
        }
    }
}

fn check_ped_transition(before: &Output, after: &Output) {
    for i in 0..4 {
        match before.ped_signal[i] {
            PedColor::Green => assert!(matches!(
                after.ped_signal[i],
                PedColor::Green | PedColor::GreenFlashing
            )),
            PedColor::GreenFlashing => assert!(matches!(
                after.ped_signal[i],
                PedColor::GreenFlashing | PedColor::Red
            )),
            _ => {}
        }
    }
}

// ----------------------------------------------------------------------
// SR-1 / SR-2 / SR-4.2: exhaustive check of the output mapping
// ----------------------------------------------------------------------
#[test]
fn output_mapping_invariants_hold_everywhere() {
    for top in [TopState::Off, TopState::Initializing, TopState::Operational] {
        for mode in [Mode::Auto, Mode::Emergency] {
            for phase in PHASES {
                let o = output_for(top, mode, phase);

                // SR-1: vehicle greens are mutually exclusive
                assert!(!(ns_green(&o) && ew_green(&o)));

                // SR-2.1: NS green excludes pedestrian green on N and S
                if ns_green(&o) {
                    assert_eq!(o.ped_signal[0], PedColor::Red);
                    assert_eq!(o.ped_signal[2], PedColor::Red);
                }
                // SR-2.2: EW green excludes pedestrian green on E and W
                if ew_green(&o) {
                    assert_eq!(o.ped_signal[1], PedColor::Red);
                    assert_eq!(o.ped_signal[3], PedColor::Red);
                }

                // SR-4.2: Emergency output is the defined safe state
                if top == TopState::Operational && mode == Mode::Emergency {
                    assert_eq!(o, Output::safe_state());
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// SR-3.1 / SR-3.2: no green -> red without an intermediate phase.
// Exhaustive over a covering set of states
// ----------------------------------------------------------------------
#[test]
fn no_unsafe_signal_transition_in_auto() {
    let phase_steps = [
        0u8,
        MIN_ALLRED_STEPS - 1,
        MIN_ALLRED_STEPS,
        MIN_YELLOW_STEPS - 1,
        MIN_YELLOW_STEPS,
        MIN_GREEN_STEPS - 1,
        MIN_GREEN_STEPS,
        MIN_GREEN_STEPS + 1,
    ];
    let preempts = [
        PreemptState::NoPreempt,
        PreemptState::PreemptNs,
        PreemptState::PreemptEw,
    ];
    let requests = [[false, false], [true, false], [false, true], [true, true]];

    for phase in PHASES {
        for ps in phase_steps {
            for pr in preempts {
                for demand in 0u8..16 {
                    for veh in 0u8..16 {
                        for req in requests {
                            let mut s = State::initial();
                            s.top = TopState::Operational;
                            s.mode = Mode::Auto;
                            s.phase = phase;
                            s.phase_steps = ps;
                            s.preempt = pr;
                            s.ped_demand = bits4(demand);

                            let mut input = Input::idle();
                            input.vehicle_present = bits4(veh);
                            input.emergency_request = req;

                            let before = output_for(s.top, s.mode, s.phase);
                            let (n, after, _) = step(s, input);

                            if n.top == TopState::Operational && n.mode == Mode::Auto {
                                check_vehicle_transition(&before, &after);
                                check_ped_transition(&before, &after);
                            }
                        }
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// SR-3.3: yellow and all-red minimum durations, also under preemption
// ----------------------------------------------------------------------
#[test]
fn intermediate_phase_minimum_durations() {
    let cases = [
        ([false, false], PreemptState::NoPreempt),
        ([true, false], PreemptState::PreemptNs),
        ([false, true], PreemptState::PreemptEw),
    ];

    for (req, _pr) in cases {
        // Yellow_NS holds until MIN_YELLOW_STEPS
        let mut s = operational();
        s.phase = Phase::YellowNs;
        s.phase_steps = MIN_YELLOW_STEPS - 1;
        let mut input = Input::idle();
        input.emergency_request = req;
        assert_eq!(step(s, input).0.phase, Phase::YellowNs);
        s.phase_steps = MIN_YELLOW_STEPS;
        assert_eq!(step(s, input).0.phase, Phase::AllRed1);

        // AllRed_1 holds until MIN_ALLRED_STEPS
        let mut s = operational();
        s.phase = Phase::AllRed1;
        s.phase_steps = MIN_ALLRED_STEPS - 1;
        assert_eq!(step(s, input).0.phase, Phase::AllRed1);
        s.phase_steps = MIN_ALLRED_STEPS;
        assert!(matches!(
            step(s, input).0.phase,
            Phase::GreenEw | Phase::GreenNs
        ));
    }
}

// ----------------------------------------------------------------------
// SR-4.1: every critical fault forces a defined safe state in one step
// ----------------------------------------------------------------------
#[test]
fn faults_force_safe_state() {
    // lamp fault -> Emergency (flashing yellow)
    let mut input = Input::idle();
    input.lamp_fault = true;
    let (n, o, _) = step(operational(), input);
    assert_eq!(n.mode, Mode::Emergency);
    assert_eq!(o, Output::safe_state());

    // operator switch -> Emergency
    let mut input = Input::idle();
    input.operator_switch = Mode::Emergency;
    let (n, o, _) = step(operational(), input);
    assert_eq!(n.mode, Mode::Emergency);
    assert_eq!(o, Output::safe_state());

    // preemption timeout -> Emergency
    let mut s = operational();
    s.preempt = PreemptState::PreemptNs;
    s.preempt_steps = PREEMPT_MAX_STEPS;
    let mut input = Input::idle();
    input.emergency_request = [true, false];
    let (n, o, _) = step(s, input);
    assert_eq!(n.mode, Mode::Emergency);
    assert_eq!(o, Output::safe_state());

    // power loss -> Off safe state
    let mut input = Input::idle();
    input.power_ok = false;
    let (n, o, _) = step(operational(), input);
    assert_eq!(n.top, TopState::Off);
    assert_eq!(o, Output::all_off());
}

// ----------------------------------------------------------------------
// SR-4.3: Emergency is left only when all four reset conditions hold
// ----------------------------------------------------------------------
#[test]
fn emergency_exit_requires_full_reset() {
    // no service_reset
    assert_eq!(step(emergency(), Input::idle()).0.mode, Mode::Emergency);

    // service_reset but fault still present
    let mut input = Input::idle();
    input.service_reset = true;
    input.lamp_fault = true;
    assert_eq!(step(emergency(), input).0.mode, Mode::Emergency);

    // service_reset but operator still in Emergency
    let mut input = Input::idle();
    input.service_reset = true;
    input.operator_switch = Mode::Emergency;
    assert_eq!(step(emergency(), input).0.mode, Mode::Emergency);

    // service_reset but no power -> top Off
    let mut input = Input::idle();
    input.service_reset = true;
    input.power_ok = false;
    assert_ne!(step(emergency(), input).0.mode, Mode::Auto);

    // all conditions met -> back to clean Auto
    let mut input = Input::idle();
    input.service_reset = true;
    let (n, _, _) = step(emergency(), input);
    assert_eq!(n.mode, Mode::Auto);
    assert_eq!(n.phase, Phase::GreenNs);
}

// ----------------------------------------------------------------------
// SR-5.1: each direction is reachable (no deadlock)
// ----------------------------------------------------------------------
#[test]
fn each_direction_reaches_green() {
    for (legs, idx) in [([true, false, true, false], 0usize), ([false, true, false, true], 1usize)] {
        let mut s = operational();
        let mut input = Input::idle();
        input.vehicle_present = legs;
        let mut reached = false;
        for _ in 0..1000 {
            let (n, o, _) = step(s, input);
            s = n;
            if o.vehicle_signal[idx] == VehicleColor::Green {
                reached = true;
                break;
            }
        }
        assert!(reached);
    }
}

// ----------------------------------------------------------------------
// SR-5.2: bounded waiting for the opposite direction (no starvation)
// ----------------------------------------------------------------------
#[test]
fn bounded_wait_for_opposite_direction() {
    let bound = MIN_GREEN_STEPS as usize
        + MIN_YELLOW_STEPS as usize
        + MIN_ALLRED_STEPS as usize
        + 3;

    let mut s = operational();
    s.phase = Phase::GreenNs;
    s.phase_steps = 1; // worst case: just entered Green_NS
    let mut input = Input::idle();
    input.vehicle_present = [false, true, false, true]; // EW demand held

    let mut count = 0usize;
    loop {
        let (n, o, _) = step(s, input);
        s = n;
        count += 1;
        if o.vehicle_signal[1] == VehicleColor::Green {
            break;
        }
        assert!(count <= bound, "EW green not reached within the bound");
    }
    assert!(count <= bound);
}

// ----------------------------------------------------------------------
// SR-6.1: an invalid preemption request never disturbs the region
// ----------------------------------------------------------------------
#[test]
fn invalid_request_does_not_disturb_preemption() {
    // active NS preemption is kept untouched by a [true, true] frame
    let mut s = operational();
    s.preempt = PreemptState::PreemptNs;
    s.preempt_steps = 5;
    let mut input = Input::idle();
    input.emergency_request = [true, true];
    let (n, _, log) = step(s, input);
    assert_eq!(n.preempt, PreemptState::PreemptNs);
    assert_eq!(n.preempt_steps, 5);
    assert!(has_event(&log, LogEvent::InvalidEmergencyRequest));

    // from NoPreempt the region stays NoPreempt
    let (n, _, _) = step(operational(), input);
    assert_eq!(n.preempt, PreemptState::NoPreempt);
}

// ----------------------------------------------------------------------
// SR-6.2: a stuck preemption signal times out into Emergency
// ----------------------------------------------------------------------
#[test]
fn stuck_preemption_times_out() {
    let mut s = operational();
    s.preempt = PreemptState::PreemptNs;
    s.preempt_steps = PREEMPT_MAX_STEPS;
    let mut input = Input::idle();
    input.emergency_request = [true, false];
    let (n, _, log) = step(s, input);
    assert_eq!(n.mode, Mode::Emergency);
    assert!(has_event(&log, LogEvent::PreemptTimeout));
}

// ----------------------------------------------------------------------
// Start-up self test: lamp fault during init blocks Operational
// ----------------------------------------------------------------------
#[test]
fn init_self_test_failure_blocks_startup() {
    let (s, _, _) = step(State::initial(), Input::idle());
    assert_eq!(s.top, TopState::Initializing);

    let mut input = Input::idle();
    input.lamp_fault = true;
    let (s, o, log) = step(s, input);
    assert_eq!(s.top, TopState::Initializing);
    assert_eq!(o, Output::all_off());
    assert!(has_event(&log, LogEvent::InitSelfTestFailed));
}

// ----------------------------------------------------------------------
// Randomised simulation: monitors all invariants
// ----------------------------------------------------------------------
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn bit(&mut self) -> bool {
        self.next() & 1 == 1
    }
    fn rare(&mut self) -> bool {
        self.next() % 50 == 0
    }
}

fn random_input(rng: &mut Rng) -> Input {
    let mut input = Input::idle();
    input.vehicle_present = [rng.bit(), rng.bit(), rng.bit(), rng.bit()];
    input.ped_button = [rng.bit(), rng.bit(), rng.bit(), rng.bit()];
    input.emergency_request = [rng.bit(), rng.bit()];
    input.lamp_fault = rng.rare();
    input.power_ok = !rng.rare();
    input.operator_switch = if rng.rare() { Mode::Emergency } else { Mode::Auto };
    input.service_reset = rng.bit();
    input
}

#[test]
fn simulation_preserves_all_invariants() {
    let mut rng = Rng(0x1234_5678_9abc_def1);

    for _ in 0..3000 {
        let mut s = State::initial();
        let mut prev: Option<(Output, bool)> = None;

        for _ in 0..400 {
            let input = random_input(&mut rng);
            let (n, o, _) = step(s, input);

            // SR-1
            assert!(!(ns_green(&o) && ew_green(&o)));
            // SR-2.1
            if ns_green(&o) {
                assert_eq!(o.ped_signal[0], PedColor::Red);
                assert_eq!(o.ped_signal[2], PedColor::Red);
            }
            // SR-2.2
            if ew_green(&o) {
                assert_eq!(o.ped_signal[1], PedColor::Red);
                assert_eq!(o.ped_signal[3], PedColor::Red);
            }
            // SR-4.2
            if n.top == TopState::Operational && n.mode == Mode::Emergency {
                assert_eq!(o, Output::safe_state());
            }

            let auto_op = n.top == TopState::Operational && n.mode == Mode::Auto;
            // SR-3.1 / SR-3.2 between two consecutive normal cycling steps
            if let Some((ref po, prev_auto_op)) = prev {
                if prev_auto_op && auto_op {
                    check_vehicle_transition(po, &o);
                    check_ped_transition(po, &o);
                }
            }
            prev = Some((o, auto_op));
            s = n;
        }
    }
}
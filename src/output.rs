use crate::config::{NUM_LEGS, NUM_VEHICLE_GROUPS};
use crate::types::{Mode, PedColor, Phase, TopState, VehicleColor};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Output {
    pub vehicle_signal: [VehicleColor; NUM_VEHICLE_GROUPS],
    pub ped_signal: [PedColor; NUM_LEGS],
}

impl Output {
    #[must_use]
    pub const fn all_off() -> Self {
        Self {
            vehicle_signal: [VehicleColor::Off; NUM_VEHICLE_GROUPS],
            ped_signal: [PedColor::Off; NUM_LEGS],
        }
    }

    #[must_use]
    pub const fn safe_state() -> Self {
        Self {
            vehicle_signal: [VehicleColor::YellowFlashing; NUM_VEHICLE_GROUPS],
            ped_signal: [PedColor::Off; NUM_LEGS],
        }
    }
}

#[must_use]
pub fn output_for(top: TopState, mode: Mode, phase: Phase) -> Output {
    if !matches!(top, TopState::Operational) {
        return Output::all_off();
    }

    if matches!(mode, Mode::Emergency) {
        return Output::safe_state();
    }

    let mut out = Output::all_off();
    match phase {
        Phase::GreenNs => {
            out.vehicle_signal = [VehicleColor::Green, VehicleColor::Red];
            out.ped_signal = [PedColor::Red, PedColor::Green, PedColor::Red, PedColor::Green];
        }
        Phase::YellowNs => {
            out.vehicle_signal = [VehicleColor::Yellow, VehicleColor::Red];
            out.ped_signal = [
                PedColor::Red,
                PedColor::GreenFlashing,
                PedColor::Red,
                PedColor::GreenFlashing,
            ];
        }
        Phase::AllRed1 | Phase::AllRed2 => {
            out.vehicle_signal = [VehicleColor::Red, VehicleColor::Red];
            out.ped_signal = [PedColor::Red; NUM_LEGS];
        }
        Phase::GreenEw => {
            out.vehicle_signal = [VehicleColor::Red, VehicleColor::Green];
            out.ped_signal = [PedColor::Green, PedColor::Red, PedColor::Green, PedColor::Red];
        }
        Phase::YellowEw => {
            out.vehicle_signal = [VehicleColor::Red, VehicleColor::Yellow];
            out.ped_signal = [
                PedColor::GreenFlashing,
                PedColor::Red,
                PedColor::GreenFlashing,
                PedColor::Red,
            ];
        }
    }
    out
}

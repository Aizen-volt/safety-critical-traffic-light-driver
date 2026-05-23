pub const MIN_GREEN_STEPS: u8 = 50;
pub const MIN_YELLOW_STEPS: u8 = 30;
pub const MIN_ALLRED_STEPS: u8 = 20;

pub const PREEMPT_MAX_STEPS: u16 = 600;

pub const NUM_LEGS: usize = 4;
pub const NUM_VEHICLE_GROUPS: usize = 2;

const _: () = assert!(MIN_GREEN_STEPS > 0);
const _: () = assert!(MIN_YELLOW_STEPS > 0);
const _: () = assert!(MIN_ALLRED_STEPS > 0);
const _: () = assert!(PREEMPT_MAX_STEPS > 0);
const _: () = assert!(NUM_LEGS == 4);
const _: () = assert!(NUM_VEHICLE_GROUPS == 2);
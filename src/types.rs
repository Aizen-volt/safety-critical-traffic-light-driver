#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Mode {
    Auto,
    Emergency,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TopState {
    Off,
    Initializing,
    Operational,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VehicleColor {
    Red,
    Yellow,
    Green,
    YellowFlashing,
    Off,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PedColor {
    Red,
    Green,
    GreenFlashing,
    Off,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Phase {
    GreenNs,
    YellowNs,
    /// All-red between Yellow_NS and Green_EW
    AllRed1,
    GreenEw,
    YellowEw,
    /// All-red between Yellow_EW and Green_NS
    AllRed2,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PreemptState {
    NoPreempt,
    PreemptNs,
    PreemptEw,
}
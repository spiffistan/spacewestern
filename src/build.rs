//! Build system — tool selection and placement types.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BuildTool {
    None,
    Fireplace,
    ElectricLight,
    Bench,
    StandingLamp,
    TableLamp,
    Fan,
    Compost,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FluidOverlay {
    None,
    Gases,     // all gases with distinct colors
    Smoke,     // show dye density as colored overlay
    Velocity,  // show velocity magnitude as heatmap
    Pressure,  // show pressure field
    O2,        // show O2 levels (blue=high, red=depleted)
    CO2,       // show CO2 levels (dark=none, yellow-green=high)
    Temp,      // show temperature (blue=cold, white=ambient, red=hot)
}

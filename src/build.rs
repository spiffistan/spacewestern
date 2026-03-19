//! Build system — tool selection and placement types.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BuildTool {
    None,
    Place(u8),    // block ID — replaces all material-specific variants
    Destroy,
    Roof,
    RemoveFloor,
    RemoveRoof,
    Dig,
    Door,         // special: placed on existing walls, toggles door flag
    Window,       // special: placed on existing walls, replaces with glass
    WoodBox,      // special: spawns physics body, not grid block
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
    HeatFlow,  // show velocity colored by temperature (convection patterns)
}

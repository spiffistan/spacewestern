//! Build system — tool selection and placement types.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BuildTool {
    None,
    Place(u32), // block ID — matches BT_* constants
    Destroy,
    Roof,
    RemoveFloor,
    RemoveRoof,
    Dig,
    Door,          // special: placed on existing walls, toggles door flag
    Window,        // special: placed on existing walls, replaces with glass
    WindowOpening, // special: adds glassless window opening to existing wall
    WoodBox,       // special: spawns physics body, not grid block
    GrowingZone,   // paint growing zone overlay on dirt tiles
    StorageZone,   // paint storage zone overlay — each tile stores one item stack
    DigZone,       // paint dig zone — plebs will dig terrain to target depth
    BermZone,      // paint berm zone — plebs dump dirt to raise terrain
    WaterFill,     // sandbox: continuously inject water while mouse held
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FluidOverlay {
    None,
    Gases,      // all gases with distinct colors
    Smoke,      // show dye density as colored overlay
    Velocity,   // show velocity magnitude as heatmap
    Pressure,   // show pressure field
    O2,         // show O2 levels (blue=high, red=depleted)
    CO2,        // show CO2 levels (dark=none, yellow-green=high)
    Temp,       // show temperature (blue=cold, white=ambient, red=hot)
    Power,      // show voltage in power grid (dark=none, green=normal, red=overload)
    PowerAmps,  // show current flow (brightness = current magnitude)
    PowerWatts, // show power consumption/generation (green=gen, red=consume)
    Water,      // show surface water level (blue intensity)
    WaterTable, // show underground water table depth
    Sound,      // show sound pressure field (warm/cool wave visualization)
    Terrain,    // show terrain type as colored overlay with legend
    Dust,       // show GPU dust density as heatmap
}

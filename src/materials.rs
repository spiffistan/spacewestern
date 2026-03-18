//! Material system — data-driven block properties.
//!
//! Each material's properties are uploaded as a GPU storage buffer.
//! Shaders look up properties by block type ID instead of hardcoded switches.

use bytemuck::Zeroable;

pub const NUM_MATERIALS: usize = 29;

/// GPU-side material struct. Must match the GpuMaterial layout in all WGSL shaders exactly.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuMaterial {
    // Visual (vec4)
    pub color_r: f32, pub color_g: f32, pub color_b: f32,
    pub render_style: f32, // 0=flat, 1=glass, 2=fire, 3=elight, 4=tree, 5=bench, 6=slamp, 7=tlamp, 8=fan, 9=compost

    // Physical (vec4)
    pub is_solid: f32,            // 1.0 = blocks fluid+light, 0.0 = open
    pub light_transmission: f32,  // 0=opaque, 0.4=glass, 0.5=tree, 1.0=clear
    pub fluid_obstacle: f32,      // 1.0 = wall, 0.0 = open
    pub default_height: f32,

    // Lighting (vec4)
    pub light_intensity: f32,     // 0 = not a light source
    pub light_color_r: f32,
    pub light_color_g: f32,
    pub light_color_b: f32,

    // Lighting extra (vec4)
    pub light_radius: f32,
    pub light_height: f32,        // for visibility trace (lamps above furniture)
    pub is_emissive: f32,         // 1.0 = bypasses scene lighting
    pub is_furniture: f32,        // 1.0 = receives glow, sun passes over

    // Thermal (vec4)
    pub heat_capacity: f32,
    pub conductivity: f32,
    pub solar_absorption: f32,
    pub is_flammable: f32,

    // Thermal extra (vec4)
    pub ignition_temp: f32,
    pub walkable: f32,            // 1.0 = plebs can walk here
    pub is_removable: f32,        // 1.0 = player can click to remove
    pub shows_wall_face: f32,     // 1.0 = shows oblique south face (walls only)
}

pub fn build_material_table() -> Vec<GpuMaterial> {
    let mut mats = vec![GpuMaterial::zeroed(); NUM_MATERIALS];

    // 0: Air
    { let m = &mut mats[0];
        m.color_r = 0.05; m.color_g = 0.05; m.color_b = 0.08;
        m.light_transmission = 1.0; m.walkable = 1.0;
    }
    // 1: Stone wall
    { let m = &mut mats[1];
        m.color_r = 0.52; m.color_g = 0.50; m.color_b = 0.48;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 4.0; m.conductivity = 0.002; m.solar_absorption = 0.7;
        m.shows_wall_face = 1.0;
    }
    // 2: Dirt
    { let m = &mut mats[2];
        m.color_r = 0.45; m.color_g = 0.35; m.color_b = 0.20;
        m.light_transmission = 1.0; m.walkable = 1.0;
        m.heat_capacity = 2.0; m.conductivity = 0.003; m.solar_absorption = 0.5;
    }
    // 3: Water
    { let m = &mut mats[3];
        m.color_r = 0.12; m.color_g = 0.30; m.color_b = 0.55;
        m.light_transmission = 0.8;
        m.heat_capacity = 8.0; m.conductivity = 0.01; m.solar_absorption = 0.3;
    }
    // 4: Wall (generic)
    { let m = &mut mats[4];
        m.color_r = 0.58; m.color_g = 0.56; m.color_b = 0.52;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 3.0; m.conductivity = 0.003; m.solar_absorption = 0.6;
        m.shows_wall_face = 1.0;
    }
    // 5: Glass
    { let m = &mut mats[5];
        m.color_r = 0.65; m.color_g = 0.78; m.color_b = 0.88;
        m.render_style = 1.0;
        m.light_transmission = 0.4; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 1.5; m.conductivity = 0.02; m.solar_absorption = 0.1;
        m.shows_wall_face = 1.0;
    }
    // 6: Fireplace
    { let m = &mut mats[6];
        m.color_r = 0.35; m.color_g = 0.30; m.color_b = 0.28;
        m.render_style = 2.0; m.is_emissive = 1.0;
        m.light_intensity = 0.9; m.light_color_r = 1.0; m.light_color_g = 0.55; m.light_color_b = 0.15;
        m.light_radius = 6.0; m.light_height = 1.0;
        m.walkable = 1.0; m.is_removable = 1.0;
        m.heat_capacity = 3.0; m.conductivity = 0.01; m.solar_absorption = 0.7;
    }
    // 7: Electric light (ceiling)
    { let m = &mut mats[7];
        m.color_r = 0.45; m.color_g = 0.35; m.color_b = 0.20;
        m.render_style = 3.0; m.is_emissive = 1.0;
        m.light_intensity = 0.8; m.light_color_r = 0.95; m.light_color_g = 0.92; m.light_color_b = 0.85;
        m.light_radius = 6.0;
        m.walkable = 1.0; m.is_removable = 1.0;
    }
    // 8: Tree
    { let m = &mut mats[8];
        m.color_r = 0.18; m.color_g = 0.35; m.color_b = 0.12;
        m.render_style = 4.0;
        m.light_transmission = 0.5;
        m.heat_capacity = 2.0; m.conductivity = 0.001; m.solar_absorption = 0.3;
    }
    // 9: Bench
    { let m = &mut mats[9];
        m.color_r = 0.55; m.color_g = 0.38; m.color_b = 0.18;
        m.render_style = 5.0;
        m.light_transmission = 1.0; m.is_furniture = 1.0; m.walkable = 1.0;
        m.heat_capacity = 2.0; m.conductivity = 0.003; m.solar_absorption = 0.4;
    }
    // 10: Standing lamp
    { let m = &mut mats[10];
        m.color_r = 0.45; m.color_g = 0.35; m.color_b = 0.20;
        m.render_style = 6.0; m.is_emissive = 1.0;
        m.light_intensity = 1.0; m.light_color_r = 0.95; m.light_color_g = 0.85; m.light_color_b = 0.60;
        m.light_radius = 6.0; m.light_height = 2.0;
        m.walkable = 1.0; m.is_removable = 1.0;
    }
    // 11: Table lamp
    { let m = &mut mats[11];
        m.color_r = 0.55; m.color_g = 0.38; m.color_b = 0.18;
        m.render_style = 7.0;
        m.light_intensity = 0.35; m.light_color_r = 0.95; m.light_color_g = 0.80; m.light_color_b = 0.50;
        m.light_radius = 4.0; m.light_height = 1.5;
        m.is_furniture = 1.0; m.is_removable = 1.0;
    }
    // 12: Fan
    { let m = &mut mats[12];
        m.color_r = 0.60; m.color_g = 0.60; m.color_b = 0.62;
        m.render_style = 8.0;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.is_removable = 1.0;
        m.heat_capacity = 1.0; m.conductivity = 0.05; m.solar_absorption = 0.8;
        m.shows_wall_face = 1.0;
    }
    // 13: Compost
    { let m = &mut mats[13];
        m.color_r = 0.30; m.color_g = 0.25; m.color_b = 0.15;
        m.render_style = 9.0;
        m.walkable = 1.0; m.is_removable = 1.0;
        m.heat_capacity = 2.0; m.conductivity = 0.005; m.solar_absorption = 0.5;
    }

    // 14: Insulated wall (perfectly insulating — zero conductivity)
    { let m = &mut mats[14];
        m.color_r = 0.90; m.color_g = 0.90; m.color_b = 0.92;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 10.0; m.conductivity = 0.0; m.solar_absorption = 0.0;
        m.shows_wall_face = 1.0;
    }

    // 15: Pipe
    { let m = &mut mats[15];
        m.color_r = 0.50; m.color_g = 0.52; m.color_b = 0.55;
        m.render_style = 10.0;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0;
        m.is_removable = 1.0;
        m.heat_capacity = 1.0; m.conductivity = 0.03; m.solar_absorption = 0.5;
    }
    // 16: Pump
    { let m = &mut mats[16];
        m.color_r = 0.45; m.color_g = 0.55; m.color_b = 0.45;
        m.render_style = 11.0;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0;
        m.is_removable = 1.0;
        m.heat_capacity = 1.5; m.conductivity = 0.02; m.solar_absorption = 0.4;
    }
    // 17: Tank
    { let m = &mut mats[17];
        m.color_r = 0.55; m.color_g = 0.55; m.color_b = 0.60;
        m.render_style = 12.0;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0;
        m.is_removable = 1.0;
        m.heat_capacity = 3.0; m.conductivity = 0.01; m.solar_absorption = 0.3;
    }
    // 18: Valve
    { let m = &mut mats[18];
        m.color_r = 0.60; m.color_g = 0.45; m.color_b = 0.40;
        m.render_style = 13.0;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0;
        m.is_removable = 1.0;
        m.heat_capacity = 1.0; m.conductivity = 0.03; m.solar_absorption = 0.5;
    }
    // 19: Outlet
    { let m = &mut mats[19];
        m.color_r = 0.50; m.color_g = 0.52; m.color_b = 0.55;
        m.render_style = 14.0;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0;
        m.is_removable = 1.0;
        m.heat_capacity = 1.0; m.conductivity = 0.03; m.solar_absorption = 0.5;
    }
    // 20: Inlet (reads gas from environment into pipe network)
    { let m = &mut mats[20];
        m.color_r = 0.55; m.color_g = 0.50; m.color_b = 0.45;
        m.render_style = 15.0;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0;
        m.is_removable = 1.0;
        m.heat_capacity = 1.0; m.conductivity = 0.03; m.solar_absorption = 0.5;
    }

    // 21: Wood wall — warm, flammable, moderate insulation
    { let m = &mut mats[21];
        m.color_r = 0.55; m.color_g = 0.38; m.color_b = 0.18;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 2.0; m.conductivity = 0.003; m.solar_absorption = 0.4;
        m.is_flammable = 1.0; m.ignition_temp = 250.0;
        m.shows_wall_face = 1.0;
    }
    // 22: Steel wall — strong, high conductivity, fireproof
    { let m = &mut mats[22];
        m.color_r = 0.60; m.color_g = 0.62; m.color_b = 0.65;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 1.0; m.conductivity = 0.08; m.solar_absorption = 0.9;
        m.shows_wall_face = 1.0;
    }
    // 23: Sandstone wall — warm color, moderate properties
    { let m = &mut mats[23];
        m.color_r = 0.72; m.color_g = 0.60; m.color_b = 0.42;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 3.5; m.conductivity = 0.004; m.solar_absorption = 0.6;
        m.shows_wall_face = 1.0;
    }
    // 24: Granite wall — very strong, dense, slow to heat
    { let m = &mut mats[24];
        m.color_r = 0.42; m.color_g = 0.40; m.color_b = 0.45;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 5.0; m.conductivity = 0.005; m.solar_absorption = 0.8;
        m.shows_wall_face = 1.0;
    }
    // 25: Limestone wall — light color, porous, moderate insulation
    { let m = &mut mats[25];
        m.color_r = 0.82; m.color_g = 0.78; m.color_b = 0.70;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 3.0; m.conductivity = 0.003; m.solar_absorption = 0.5;
        m.shows_wall_face = 1.0;
    }

    // 26: Wood floor — warm brown planks, good insulation
    { let m = &mut mats[26];
        m.color_r = 0.55; m.color_g = 0.42; m.color_b = 0.22;
        m.light_transmission = 1.0; m.walkable = 1.0; m.is_removable = 1.0;
        m.heat_capacity = 2.5; m.conductivity = 0.002; m.solar_absorption = 0.4;
    }
    // 27: Stone floor — gray tiles, cold, durable
    { let m = &mut mats[27];
        m.color_r = 0.50; m.color_g = 0.48; m.color_b = 0.45;
        m.light_transmission = 1.0; m.walkable = 1.0; m.is_removable = 1.0;
        m.heat_capacity = 4.0; m.conductivity = 0.006; m.solar_absorption = 0.5;
    }
    // 28: Concrete floor — flat gray, modern
    { let m = &mut mats[28];
        m.color_r = 0.58; m.color_g = 0.57; m.color_b = 0.55;
        m.light_transmission = 1.0; m.walkable = 1.0; m.is_removable = 1.0;
        m.heat_capacity = 3.5; m.conductivity = 0.005; m.solar_absorption = 0.5;
    }

    mats
}

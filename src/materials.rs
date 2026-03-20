//! Material system — data-driven block properties.
//!
//! Each material's properties are uploaded as a GPU storage buffer.
//! Shaders look up properties by block type ID instead of hardcoded switches.

use bytemuck::Zeroable;

pub const NUM_MATERIALS: usize = 46;

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
    // 1: Stone wall — high thermal mass, moderate conductivity (stores heat well)
    // Real stone: ~800 J/(kg·K), ~1.5 W/(m·K). Absorbs sun heat slowly, releases it at night.
    { let m = &mut mats[1];
        m.color_r = 0.52; m.color_g = 0.50; m.color_b = 0.48;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 8.0; m.conductivity = 0.012; m.solar_absorption = 0.7;
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
    // 4: Wall (generic) — moderate thermal mass
    { let m = &mut mats[4];
        m.color_r = 0.58; m.color_g = 0.56; m.color_b = 0.52;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 6.0; m.conductivity = 0.008; m.solar_absorption = 0.6;
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

    // 21: Wood wall — low thermal mass, good insulator (air pockets), flammable
    // Real wood: ~1700 J/(kg·K), ~0.12 W/(m·K). Poor heat storage, great insulation.
    { let m = &mut mats[21];
        m.color_r = 0.55; m.color_g = 0.38; m.color_b = 0.18;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 3.0; m.conductivity = 0.003; m.solar_absorption = 0.4;
        m.is_flammable = 1.0; m.ignition_temp = 250.0;
        m.shows_wall_face = 1.0;
    }
    // 22: Steel wall — low thermal mass, very high conductivity, fireproof
    // Real steel: ~500 J/(kg·K), ~50 W/(m·K). Heats and cools fast, terrible insulator.
    { let m = &mut mats[22];
        m.color_r = 0.60; m.color_g = 0.62; m.color_b = 0.65;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 2.0; m.conductivity = 0.06; m.solar_absorption = 0.9;
        m.shows_wall_face = 1.0;
    }
    // 23: Sandstone wall — good thermal mass, low conductivity (porous)
    // Real sandstone: ~920 J/(kg·K), ~1.7 W/(m·K). Good balance of storage and insulation.
    { let m = &mut mats[23];
        m.color_r = 0.72; m.color_g = 0.60; m.color_b = 0.42;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 7.0; m.conductivity = 0.010; m.solar_absorption = 0.6;
        m.shows_wall_face = 1.0;
    }
    // 24: Granite wall — very high thermal mass, moderate conductivity (dense crystalline)
    // Real granite: ~790 J/(kg·K) but very dense (~2700 kg/m³), ~2.5 W/(m·K). Massive heat battery.
    { let m = &mut mats[24];
        m.color_r = 0.42; m.color_g = 0.40; m.color_b = 0.45;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 10.0; m.conductivity = 0.015; m.solar_absorption = 0.8;
        m.shows_wall_face = 1.0;
    }
    // 25: Limestone wall — moderate thermal mass, porous (good insulator)
    // Real limestone: ~840 J/(kg·K), ~1.3 W/(m·K). Light color reflects more sun.
    { let m = &mut mats[25];
        m.color_r = 0.82; m.color_g = 0.78; m.color_b = 0.70;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 6.0; m.conductivity = 0.008; m.solar_absorption = 0.4;
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

    // 29: Cannon — directional, fires cannonballs
    { let m = &mut mats[29];
        m.color_r = 0.30; m.color_g = 0.28; m.color_b = 0.25;
        m.render_style = 16.0;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 2.0;
        m.is_removable = 1.0;
        m.heat_capacity = 3.0; m.conductivity = 0.04; m.solar_absorption = 0.7;
    }

    // 30: Bed — furniture, walkable, plebs sleep here
    { let m = &mut mats[30];
        m.color_r = 0.35; m.color_g = 0.25; m.color_b = 0.50; // purple-ish bedding
        m.render_style = 17.0;
        m.light_transmission = 1.0; m.is_furniture = 1.0; m.walkable = 1.0;
        m.is_removable = 1.0;
        m.heat_capacity = 2.0; m.conductivity = 0.002; m.solar_absorption = 0.3;
    }

    // 31: Berry bush — harvestable plant
    { let m = &mut mats[31];
        m.color_r = 0.20; m.color_g = 0.40; m.color_b = 0.15;
        m.render_style = 18.0;
        m.light_transmission = 0.6;
        m.walkable = 1.0; // can walk through (low bush)
        m.heat_capacity = 1.5; m.conductivity = 0.001; m.solar_absorption = 0.3;
    }

    // 32: Dug ground — excavated tile, can fill with water at depth >= 2
    { let m = &mut mats[32];
        m.color_r = 0.35; m.color_g = 0.28; m.color_b = 0.15; // exposed earth
        m.render_style = 19.0;
        m.light_transmission = 1.0; m.walkable = 1.0;
        m.heat_capacity = 2.0; m.conductivity = 0.003; m.solar_absorption = 0.5;
    }

    // 33: Storage crate — holds resources
    { let m = &mut mats[33];
        m.color_r = 0.50; m.color_g = 0.38; m.color_b = 0.20;
        m.render_style = 20.0;
        m.light_transmission = 1.0; m.is_furniture = 1.0;
        m.is_removable = 1.0;
        m.heat_capacity = 2.0; m.conductivity = 0.002; m.solar_absorption = 0.4;
    }

    // 35: Mud wall — organic, rounded, good insulation, moderate thermal mass
    { let m = &mut mats[35];
        m.color_r = 0.52; m.color_g = 0.40; m.color_b = 0.25;
        m.is_solid = 1.0; m.fluid_obstacle = 1.0; m.default_height = 3.0;
        m.heat_capacity = 5.0; m.conductivity = 0.004; m.solar_absorption = 0.5;
        m.shows_wall_face = 1.0;
    }

    // 34: Rock — natural stone that can be picked up and hauled
    { let m = &mut mats[34];
        m.color_r = 0.50; m.color_g = 0.48; m.color_b = 0.44;
        m.render_style = 21.0;
        m.walkable = 1.0; // can walk over
        m.is_removable = 1.0;
        m.light_transmission = 1.0;
        m.heat_capacity = 3.0; m.conductivity = 0.02; m.solar_absorption = 0.6;
    }

    // 36: Wire
    { let m = &mut mats[36];
        m.color_r = 0.45; m.color_g = 0.42; m.color_b = 0.35;
        m.render_style = 22.0;
        m.walkable = 1.0; m.is_removable = 1.0; m.light_transmission = 1.0;
        m.heat_capacity = 0.5; m.conductivity = 0.04; m.solar_absorption = 0.3;
    }
    // 37: Solar Panel
    { let m = &mut mats[37];
        m.color_r = 0.15; m.color_g = 0.18; m.color_b = 0.35;
        m.render_style = 23.0;
        m.is_removable = 1.0; m.light_transmission = 0.3;
        m.heat_capacity = 1.0; m.conductivity = 0.02; m.solar_absorption = 0.9;
    }
    // 38: Battery (small)
    { let m = &mut mats[38];
        m.color_r = 0.35; m.color_g = 0.45; m.color_b = 0.30;
        m.render_style = 24.0;
        m.is_removable = 1.0; m.is_furniture = 1.0; m.light_transmission = 1.0;
        m.heat_capacity = 2.0; m.conductivity = 0.01; m.solar_absorption = 0.3;
    }
    // 39: Battery (medium, 2 tiles)
    { let m = &mut mats[39];
        m.color_r = 0.30; m.color_g = 0.42; m.color_b = 0.28;
        m.render_style = 25.0;
        m.is_removable = 1.0; m.is_furniture = 1.0; m.light_transmission = 1.0;
        m.heat_capacity = 3.0; m.conductivity = 0.01; m.solar_absorption = 0.3;
    }
    // 40: Battery (large, 2x2)
    { let m = &mut mats[40];
        m.color_r = 0.25; m.color_g = 0.38; m.color_b = 0.25;
        m.render_style = 26.0;
        m.is_removable = 1.0; m.is_furniture = 1.0; m.light_transmission = 1.0;
        m.heat_capacity = 5.0; m.conductivity = 0.01; m.solar_absorption = 0.3;
    }

    // 42: Switch
    { let m = &mut mats[42];
        m.color_r = 0.50; m.color_g = 0.48; m.color_b = 0.42;
        m.render_style = 28.0;
        m.walkable = 1.0; m.is_removable = 1.0; m.light_transmission = 1.0;
    }
    // 43: Dimmer
    { let m = &mut mats[43];
        m.color_r = 0.48; m.color_g = 0.45; m.color_b = 0.40;
        m.render_style = 29.0;
        m.walkable = 1.0; m.is_removable = 1.0; m.light_transmission = 1.0;
    }

    // 41: Wind Turbine
    { let m = &mut mats[41];
        m.color_r = 0.60; m.color_g = 0.62; m.color_b = 0.65;
        m.render_style = 27.0;
        m.is_removable = 1.0; m.light_transmission = 0.5;
        m.heat_capacity = 1.0; m.conductivity = 0.03; m.solar_absorption = 0.5;
    }

    mats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_count() {
        let mats = build_material_table();
        assert_eq!(mats.len(), NUM_MATERIALS);
    }

    #[test]
    fn test_solid_blocks_have_height() {
        let mats = build_material_table();
        for (i, m) in mats.iter().enumerate() {
            if m.is_solid > 0.5 && m.default_height < 0.5 {
                // Pipe components and similar can be solid without default height
                // but walls should have height
                if matches!(i, 1 | 4 | 14 | 21..=25) {
                    panic!("Wall material {} has is_solid but no default_height", i);
                }
            }
        }
    }

    #[test]
    fn test_light_sources_have_color() {
        let mats = build_material_table();
        for (i, m) in mats.iter().enumerate() {
            if m.light_intensity > 0.0 {
                let color_sum = m.light_color_r + m.light_color_g + m.light_color_b;
                assert!(color_sum > 0.1, "Light source {} has intensity but no color", i);
                assert!(m.light_radius > 0.0, "Light source {} has intensity but no radius", i);
            }
        }
    }

    #[test]
    fn test_walkable_items_not_solid() {
        let mats = build_material_table();
        for (i, m) in mats.iter().enumerate() {
            if m.walkable > 0.5 && m.is_solid > 0.5 {
                panic!("Material {} is both walkable and solid", i);
            }
        }
    }

    #[test]
    fn test_insulated_wall_zero_conductivity() {
        let mats = build_material_table();
        let insulated = &mats[14];
        assert_eq!(insulated.conductivity, 0.0);
        assert_eq!(insulated.solar_absorption, 0.0);
        assert!(insulated.heat_capacity > 5.0); // high thermal mass
    }

    #[test]
    fn test_flammable_materials_have_ignition_temp() {
        let mats = build_material_table();
        for (i, m) in mats.iter().enumerate() {
            if m.is_flammable > 0.5 {
                assert!(m.ignition_temp > 0.0,
                    "Flammable material {} has no ignition temperature", i);
            }
        }
    }
}

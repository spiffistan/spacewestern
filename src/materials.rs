//! Material system — GPU-side material struct and table generation.
//!
//! The material table is generated from `blocks.toml` via `BlockRegistry`.
//! This module owns the `GpuMaterial` struct definition (must match all WGSL shaders)
//! and re-exports the table builder for convenience.


pub const NUM_MATERIALS: usize = 57;

/// GPU-side material struct. Must match the GpuMaterial layout in all WGSL shaders exactly.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuMaterial {
    // Visual (vec4)
    pub color_r: f32, pub color_g: f32, pub color_b: f32,
    pub render_style: f32,

    // Physical (vec4)
    pub is_solid: f32,
    pub light_transmission: f32,
    pub fluid_obstacle: f32,
    pub default_height: f32,

    // Lighting (vec4)
    pub light_intensity: f32,
    pub light_color_r: f32,
    pub light_color_g: f32,
    pub light_color_b: f32,

    // Lighting extra (vec4)
    pub light_radius: f32,
    pub light_height: f32,
    pub is_emissive: f32,
    pub is_furniture: f32,

    // Thermal (vec4)
    pub heat_capacity: f32,
    pub conductivity: f32,
    pub solar_absorption: f32,
    pub is_flammable: f32,

    // Thermal extra (vec4)
    pub ignition_temp: f32,
    pub walkable: f32,
    pub is_removable: f32,
    pub shows_wall_face: f32,
}

/// Build the GPU material table from blocks.toml (single source of truth).
pub fn build_material_table() -> Vec<GpuMaterial> {
    crate::block_defs::BlockRegistry::load().build_gpu_materials()
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
                if matches!(i, 1 | 4 | 14 | 21..=25 | 35) {
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

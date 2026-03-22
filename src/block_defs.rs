//! Data-driven block definitions loaded from blocks.toml.
//! Replaces hardcoded material tables, build tool enums, and UI menu entries.

use serde::Deserialize;
use crate::materials::{GpuMaterial, NUM_MATERIALS};
use bytemuck::Zeroable;

#[derive(Deserialize)]
struct BlockFile {
    block: Vec<BlockDef>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct BlockDef {
    pub id: u8,
    pub name: String,
    pub color: [f32; 3],

    #[serde(default)] pub render_style: f32,
    #[serde(default)] pub is_solid: bool,
    #[serde(default)] pub light_transmission: f32,
    #[serde(default)] pub fluid_obstacle: bool,
    #[serde(default)] pub default_height: f32,
    #[serde(default)] pub is_emissive: bool,
    #[serde(default)] pub is_furniture: bool,
    #[serde(default)] pub walkable: bool,
    #[serde(default)] pub is_removable: bool,
    #[serde(default)] pub shows_wall_face: bool,
    #[serde(default)] pub is_flammable: bool,
    #[serde(default)] pub ignition_temp: f32,
    #[serde(default)] pub is_wall: bool,
    #[serde(default)] pub is_plant: bool,     // plants: no shadow casting, harvestable
    #[serde(default)] pub is_harvestable: bool, // can be harvested via work queue

    #[serde(default)] pub light: Option<LightDef>,
    #[serde(default)] pub thermal: Option<ThermalDef>,
    #[serde(default)] pub placement: Option<PlacementDef>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LightDef {
    pub intensity: f32,
    pub color: [f32; 3],
    pub radius: f32,
    #[serde(default)] pub height: f32,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct ThermalDef {
    #[serde(default)] pub heat_capacity: f32,
    #[serde(default)] pub conductivity: f32,
    #[serde(default)] pub solar_absorption: f32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PlacementDef {
    pub category: String,
    pub icon: String,
    pub label: String,
    #[serde(default)] pub click: ClickMode,
    #[serde(default)] pub place_height: u8,
    #[serde(default)] pub drag: Option<DragShape>,
    #[serde(default)] pub rotatable: bool,
    #[serde(default)] pub stays_selected: bool,
    #[serde(default)] pub extra_flags: u8,
}

#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClickMode {
    #[default]
    Simple,
    None,
    OnWall,
    OnFurniture,
    MultiTile,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DragShape {
    HollowRect,
    FilledRect,
    Line,
    DiagonalLine,
    None,
}

/// Runtime registry of all block definitions, built once at startup.
/// Cached via OnceLock — TOML is parsed only once, not per-frame.
pub struct BlockRegistry {
    defs: Vec<Option<BlockDef>>,          // indexed by block ID (0..NUM_MATERIALS)
    pub wall_ids: Vec<u8>,               // all IDs where is_wall=true
    pub placeable: Vec<(u8, PlacementDef)>, // (block_id, placement) for build menu
}

static REGISTRY_CACHE: std::sync::OnceLock<BlockRegistry> = std::sync::OnceLock::new();

impl BlockRegistry {
    /// Get the cached registry (parsed once, reused forever).
    pub fn cached() -> &'static BlockRegistry {
        REGISTRY_CACHE.get_or_init(Self::load)
    }

    /// Parse the registry from blocks.toml. Prefer `cached()` for runtime use.
    pub fn load() -> Self {
        let toml_str = include_str!("blocks.toml");
        let file: BlockFile = toml::from_str(toml_str).expect("Failed to parse blocks.toml");

        let mut defs: Vec<Option<BlockDef>> = (0..NUM_MATERIALS).map(|_| None).collect();
        let mut wall_ids = Vec::new();
        let mut placeable = Vec::new();

        for def in file.block {
            let id = def.id as usize;
            assert!(id < NUM_MATERIALS, "Block ID {} exceeds NUM_MATERIALS ({})", id, NUM_MATERIALS);
            if def.is_wall {
                wall_ids.push(def.id);
            }
            if let Some(ref p) = def.placement {
                placeable.push((def.id, p.clone()));
            }
            defs[id] = Some(def);
        }

        BlockRegistry { defs, wall_ids, placeable }
    }

    pub fn get(&self, id: u8) -> Option<&BlockDef> {
        self.defs.get(id as usize).and_then(|d| d.as_ref())
    }

    pub fn name(&self, id: u8) -> &str {
        self.get(id).map(|d| d.name.as_str()).unwrap_or("Unknown")
    }

    pub fn is_wall(&self, id: u8) -> bool {
        self.wall_ids.contains(&id)
    }

    pub fn tools_in_category<'a>(&'a self, cat: &str) -> Vec<&'a (u8, PlacementDef)> {
        self.placeable.iter().filter(|(_, p)| p.category == cat).collect()
    }

    /// Build GPU material table from block definitions.
    pub fn build_gpu_materials(&self) -> Vec<GpuMaterial> {
        let mut mats = vec![GpuMaterial::zeroed(); NUM_MATERIALS];

        for (i, slot) in self.defs.iter().enumerate() {
            let Some(def) = slot else { continue };
            let m = &mut mats[i];

            m.color_r = def.color[0];
            m.color_g = def.color[1];
            m.color_b = def.color[2];
            m.render_style = def.render_style;

            m.is_solid = if def.is_solid { 1.0 } else { 0.0 };
            m.light_transmission = def.light_transmission;
            m.fluid_obstacle = if def.fluid_obstacle { 1.0 } else { 0.0 };
            m.default_height = def.default_height;

            m.is_emissive = if def.is_emissive { 1.0 } else { 0.0 };
            m.is_furniture = if def.is_furniture { 1.0 } else { 0.0 };
            m.walkable = if def.walkable { 1.0 } else { 0.0 };
            m.is_removable = if def.is_removable { 1.0 } else { 0.0 };
            m.shows_wall_face = if def.shows_wall_face { 1.0 } else { 0.0 };
            m.is_flammable = if def.is_flammable { 1.0 } else { 0.0 };
            m.ignition_temp = def.ignition_temp;

            if let Some(ref light) = def.light {
                m.light_intensity = light.intensity;
                m.light_color_r = light.color[0];
                m.light_color_g = light.color[1];
                m.light_color_b = light.color[2];
                m.light_radius = light.radius;
                m.light_height = light.height;
            }

            if let Some(ref th) = def.thermal {
                m.heat_capacity = th.heat_capacity;
                m.conductivity = th.conductivity;
                m.solar_absorption = th.solar_absorption;
            }
        }

        mats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_registry() {
        let reg = BlockRegistry::load();
        assert!(reg.get(0).is_some(), "Air should exist");
        assert!(reg.get(1).is_some(), "Stone Wall should exist");
        assert_eq!(reg.name(1), "Stone Wall");
        assert!(reg.is_wall(1));
        assert!(!reg.is_wall(0));
        assert!(!reg.placeable.is_empty());
    }

    #[test]
    fn test_wall_ids() {
        let reg = BlockRegistry::load();
        assert!(reg.is_wall(1));  // Stone
        assert!(reg.is_wall(4));  // Wall
        assert!(reg.is_wall(5));  // Glass
        assert!(reg.is_wall(14)); // Insulated
        assert!(reg.is_wall(21)); // Wood
        assert!(reg.is_wall(35)); // Mud
        assert!(!reg.is_wall(6)); // Fireplace
        assert!(!reg.is_wall(2)); // Dirt
    }

    #[test]
    fn test_categories() {
        let reg = BlockRegistry::load();
        let walls = reg.tools_in_category("Walls");
        assert!(walls.len() >= 5, "Should have at least 5 wall types");
        let floors = reg.tools_in_category("Floor");
        assert!(floors.len() >= 3, "Should have at least 3 floor types");
    }
}

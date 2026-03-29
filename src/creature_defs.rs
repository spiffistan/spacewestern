//! Data-driven creature definitions loaded from creatures.toml.

use serde::Deserialize;

pub const CREATURE_DUSKWEAVER: u8 = 0;
pub const CREATURE_HOLLOWCALL: u8 = 1;

const MAX_CREATURE_TYPES: usize = 16;

#[derive(Deserialize)]
struct CreatureFile {
    creature: Vec<CreatureDef>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CreatureDef {
    pub id: u8,
    pub name: String,
    #[serde(default = "default_health")]
    pub health: f32,
    #[serde(default)]
    pub speed: f32,
    #[serde(default)]
    pub damage: f32,
    #[serde(default)]
    pub nocturnal: bool,
    #[serde(default = "default_one")]
    pub pack_min: u8,
    #[serde(default = "default_one")]
    pub pack_max: u8,
    #[serde(default)]
    pub flee_light_radius: f32,
    #[serde(default)]
    pub flee_group_size: u8,
    #[serde(default)]
    pub body_radius: f32,
    #[serde(default)]
    pub color: [f32; 3],
    #[serde(default)]
    pub eye_color: [f32; 3],
    #[serde(default)]
    pub sound_amplitude_db: f32,
    #[serde(default)]
    pub sound_frequency: f32,
    #[serde(default)]
    pub sound_pattern: u32,
    #[serde(default = "default_interval")]
    pub sound_interval: f32,
}

fn default_health() -> f32 {
    10.0
}
fn default_one() -> u8 {
    1
}
fn default_interval() -> f32 {
    5.0
}

pub struct CreatureRegistry {
    defs: Vec<Option<CreatureDef>>,
}

static CREATURE_REGISTRY_CACHE: std::sync::OnceLock<CreatureRegistry> = std::sync::OnceLock::new();

impl CreatureRegistry {
    pub fn cached() -> &'static CreatureRegistry {
        CREATURE_REGISTRY_CACHE.get_or_init(Self::load)
    }

    pub fn load() -> Self {
        let toml_str = include_str!("creatures.toml");
        let file: CreatureFile = toml::from_str(toml_str).expect("Failed to parse creatures.toml");
        let mut defs: Vec<Option<CreatureDef>> = (0..MAX_CREATURE_TYPES).map(|_| None).collect();
        for def in file.creature {
            let id = def.id as usize;
            assert!(
                id < MAX_CREATURE_TYPES,
                "Creature ID {} exceeds MAX_CREATURE_TYPES ({})",
                id,
                MAX_CREATURE_TYPES
            );
            defs[id] = Some(def);
        }
        CreatureRegistry { defs }
    }

    pub fn get(&self, id: u8) -> Option<&CreatureDef> {
        self.defs.get(id as usize).and_then(|d| d.as_ref())
    }

    pub fn name(&self, id: u8) -> &str {
        self.get(id).map(|d| d.name.as_str()).unwrap_or("Unknown")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_creature_registry() {
        let reg = CreatureRegistry::load();
        let dw = reg.get(CREATURE_DUSKWEAVER).unwrap();
        assert_eq!(dw.name, "Duskweaver");
        assert_eq!(dw.pack_min, 3);
        assert_eq!(dw.pack_max, 7);
        assert!(dw.nocturnal);
        assert!(dw.speed > 0.0);

        let hc = reg.get(CREATURE_HOLLOWCALL).unwrap();
        assert_eq!(hc.name, "Hollowcall");
        assert_eq!(hc.speed, 0.0);
        assert_eq!(hc.body_radius, 0.0);
    }
}

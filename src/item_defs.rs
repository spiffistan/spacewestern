//! Data-driven item definitions loaded from items.toml.

use serde::Deserialize;

// Item ID constants (must match items.toml)
pub const ITEM_BERRIES: u16 = 0;
pub const ITEM_WOOD: u16 = 1;
pub const ITEM_ROCK: u16 = 2;
pub const ITEM_FIBER: u16 = 3;
pub const ITEM_CLAY: u16 = 4;
pub const ITEM_SCRAP_WOOD: u16 = 5;
pub const ITEM_ROPE: u16 = 10;
pub const ITEM_WOODEN_BUCKET: u16 = 11;
pub const ITEM_CLAY_JUG: u16 = 12;
pub const ITEM_UNFIRED_JUG: u16 = 13;
pub const ITEM_STONE_AXE: u16 = 20;
pub const ITEM_WOODEN_SHOVEL: u16 = 21;
pub const ITEM_STONE_PICK: u16 = 22;
pub const ITEM_PLANK: u16 = 23;

#[derive(Deserialize)]
struct ItemFile {
    item: Vec<ItemDef>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ItemDef {
    pub id: u16,
    pub name: String,
    pub icon: String,
    #[serde(default)]
    pub category: String,
    #[serde(default = "default_stack_max")]
    pub stack_max: u16,
    #[serde(default)]
    pub nutrition: f32,
    #[serde(default)]
    pub liquid_capacity: u16,
}

fn default_stack_max() -> u16 {
    1
}

/// An item stack: one slot holding some quantity of an item type,
/// optionally containing liquid (for containers).
#[derive(Clone, Debug, PartialEq)]
pub struct ItemStack {
    pub item_id: u16,
    pub count: u16,
    pub liquid: Option<(LiquidType, u16)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LiquidType {
    Water,
}

impl ItemStack {
    pub fn new(item_id: u16, count: u16) -> Self {
        Self {
            item_id,
            count,
            liquid: None,
        }
    }

    pub fn label(&self) -> String {
        let reg = ItemRegistry::cached();
        let name = reg.name(self.item_id);
        if self.count == 1 {
            let mut s = name.to_string();
            if let Some((liq, amt)) = self.liquid {
                let liq_name = match liq {
                    LiquidType::Water => "water",
                };
                s += &format!(
                    " ({}/{})",
                    amt,
                    reg.get(self.item_id)
                        .map(|d| d.liquid_capacity)
                        .unwrap_or(0)
                );
                s += &format!(" {}", liq_name);
            }
            s
        } else {
            format!("{}x {}", self.count, name)
        }
    }

    pub fn icon(&self) -> &str {
        let reg = ItemRegistry::cached();
        reg.get(self.item_id)
            .map(|d| d.icon.as_str())
            .unwrap_or("?")
    }

    pub fn is_container(&self) -> bool {
        let reg = ItemRegistry::cached();
        reg.get(self.item_id)
            .map(|d| d.liquid_capacity > 0)
            .unwrap_or(false)
    }

    pub fn liquid_capacity(&self) -> u16 {
        let reg = ItemRegistry::cached();
        reg.get(self.item_id)
            .map(|d| d.liquid_capacity)
            .unwrap_or(0)
    }
}

const MAX_ITEMS: usize = 64;

pub struct ItemRegistry {
    defs: Vec<Option<ItemDef>>,
}

static ITEM_REGISTRY_CACHE: std::sync::OnceLock<ItemRegistry> = std::sync::OnceLock::new();

impl ItemRegistry {
    pub fn cached() -> &'static ItemRegistry {
        ITEM_REGISTRY_CACHE.get_or_init(Self::load)
    }

    pub fn load() -> Self {
        let toml_str = include_str!("items.toml");
        let file: ItemFile = toml::from_str(toml_str).expect("Failed to parse items.toml");
        let mut defs: Vec<Option<ItemDef>> = (0..MAX_ITEMS).map(|_| None).collect();
        for def in file.item {
            let id = def.id as usize;
            assert!(
                id < MAX_ITEMS,
                "Item ID {} exceeds MAX_ITEMS ({})",
                id,
                MAX_ITEMS
            );
            defs[id] = Some(def);
        }
        ItemRegistry { defs }
    }

    pub fn get(&self, id: u16) -> Option<&ItemDef> {
        self.defs.get(id as usize).and_then(|d| d.as_ref())
    }

    pub fn name(&self, id: u16) -> &str {
        self.get(id).map(|d| d.name.as_str()).unwrap_or("Unknown")
    }

    pub fn stack_max(&self, id: u16) -> u16 {
        self.get(id).map(|d| d.stack_max).unwrap_or(1)
    }

    pub fn nutrition(&self, id: u16) -> f32 {
        self.get(id).map(|d| d.nutrition).unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_item_registry() {
        let reg = ItemRegistry::load();
        assert!(reg.get(ITEM_BERRIES).is_some());
        assert_eq!(reg.name(ITEM_BERRIES), "Berries");
        assert_eq!(reg.stack_max(ITEM_BERRIES), 20);
        assert!(reg.nutrition(ITEM_BERRIES) > 0.0);
        assert_eq!(reg.name(ITEM_WOOD), "Wood");
        assert_eq!(reg.name(ITEM_ROCK), "Rock");
    }

    #[test]
    fn test_item_stack() {
        let stack = ItemStack::new(ITEM_BERRIES, 5);
        assert_eq!(stack.count, 5);
        assert!(!stack.is_container());

        let bucket = ItemStack::new(ITEM_WOODEN_BUCKET, 1);
        assert!(bucket.is_container());
        assert_eq!(bucket.liquid_capacity(), 5);
    }

    #[test]
    fn test_containers() {
        let reg = ItemRegistry::load();
        let bucket = reg.get(ITEM_WOODEN_BUCKET).unwrap();
        assert_eq!(bucket.liquid_capacity, 5);
        let jug = reg.get(ITEM_CLAY_JUG).unwrap();
        assert_eq!(jug.liquid_capacity, 3);
    }
}

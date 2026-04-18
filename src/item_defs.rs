//! Data-driven item definitions loaded from items.toml.

use serde::Deserialize;

// Item ID constants (must match items.toml)
// Food — raw 0–99, cooked 100–199
pub const ITEM_BERRIES: u16 = 0;
pub const ITEM_RAW_MEAT: u16 = 1;
pub const ITEM_RAW_FISH: u16 = 2;
pub const ITEM_COOKED_MEAT: u16 = 100;
pub const ITEM_COOKED_FISH: u16 = 101;
// Raw materials 200–299
pub const ITEM_WOOD: u16 = 200;
pub const ITEM_ROCK: u16 = 201;
pub const ITEM_FIBER: u16 = 202;
pub const ITEM_CLAY: u16 = 203;
pub const ITEM_SCRAP_WOOD: u16 = 204; // sticks/branches
pub const ITEM_LOG: u16 = 205;
pub const ITEM_REED_STALK: u16 = 206;
pub const ITEM_THORNS: u16 = 207;
pub const ITEM_SALT: u16 = 208;
pub const ITEM_NECTAR: u16 = 3; // food category (raw 0-99)
pub const ITEM_DRIED_PETALS: u16 = 209;
pub const ITEM_CHARCOAL: u16 = 210;
// Intermediate / crafted materials 300–399
pub const ITEM_ROPE: u16 = 300;
pub const ITEM_PLANK: u16 = 301;
pub const ITEM_UNFIRED_JUG: u16 = 302;
// Containers 400–499
pub const ITEM_WOODEN_BUCKET: u16 = 400;
pub const ITEM_CLAY_JUG: u16 = 401;
// Tools 500–599
// Stone tier (Tier 1: knapped stone + stick handle)
pub const ITEM_STONE_AXE: u16 = 500;
pub const ITEM_STONE_PICK: u16 = 501;
pub const ITEM_WOODEN_SHOVEL: u16 = 502;
pub const ITEM_KNIFE: u16 = 503; // stone knife (legacy name kept for compat)
// Primitive tier (Tier 0-1: hands + found materials)
pub const ITEM_PRIMITIVE_HAMMERSTONE: u16 = 504;
pub const ITEM_PRIMITIVE_STONE_BLADE: u16 = 505;
pub const ITEM_PRIMITIVE_DIGGING_STICK: u16 = 509;
// Flint tier (Tier 2: requires flint from chalk/limestone)
pub const ITEM_FLINT_BLADE: u16 = 506;
pub const ITEM_FLINT_PICK: u16 = 507;
pub const ITEM_FLINT_AXE: u16 = 508;
// Weapons 600–699
pub const ITEM_PISTOL: u16 = 600;
// Ammo 800–899
pub const ITEM_PISTOL_ROUNDS: u16 = 800;
// Equipment / utility 700–799
pub const ITEM_FIBER_BELT: u16 = 700;
pub const ITEM_FISHING_LINE: u16 = 701;
pub const ITEM_SNARE: u16 = 702;

// Weapon type constants (matches shader)
pub const WEAPON_NONE: u8 = 0;
pub const WEAPON_AXE: u8 = 1;
pub const WEAPON_PICK: u8 = 2;
pub const WEAPON_SHOVEL: u8 = 3;
pub const WEAPON_PISTOL: u8 = 4;
pub const WEAPON_KNIFE: u8 = 5;

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
    #[serde(default)]
    pub melee_damage: f32,
    #[serde(default)]
    pub melee_speed: f32,
    #[serde(default)]
    pub melee_range: f32,
    #[serde(default)]
    pub melee_knockback: f32,
    #[serde(default)]
    pub melee_bleed: f32,
    #[serde(default)]
    pub weapon_type: u8,
    #[serde(default)]
    pub is_ranged: bool,
    #[serde(default)]
    pub ranged_spread: f32,
    #[serde(default)]
    pub ranged_aim_speed: f32,
    #[serde(default)]
    pub magazine_size: u8,
    #[serde(default)]
    pub reload_time: f32,
    /// Tool type for activity gating: "knife", "axe", "pick", "shovel"
    #[serde(default)]
    pub tool_type: String,
    /// If true, this item is a belt (wearable equipment layer)
    #[serde(default)]
    pub is_belt: bool,
    /// Number of belt slots (only meaningful when is_belt = true)
    #[serde(default)]
    pub belt_slots: u8,
    /// Time in game-seconds before this item spoils (0 = never spoils)
    #[serde(default)]
    pub spoil_time: f32,
    /// Chance of nausea when eating (0.0–1.0, e.g. 0.15 = 15%)
    #[serde(default)]
    pub sickness_chance: f32,
    /// Ammo type this weapon uses or this ammo provides (e.g. "9mm")
    #[serde(default)]
    pub ammo_type: String,
    /// Maximum durability (uses) for tools. 0 = indestructible.
    #[serde(default)]
    pub max_durability: u16,
}

impl ItemDef {
    pub fn is_melee_weapon(&self) -> bool {
        self.melee_damage > 0.0
    }

    pub fn is_ranged_weapon(&self) -> bool {
        self.is_ranged
    }

    /// True if this item is a tool or weapon that belongs on a belt
    pub fn is_belt_item(&self) -> bool {
        self.is_melee_weapon() || self.is_ranged_weapon() || !self.tool_type.is_empty()
    }

    /// Check if this item has a specific tool type
    pub fn has_tool_type(&self, t: &str) -> bool {
        self.tool_type == t
    }
}

fn default_stack_max() -> u16 {
    1
}

/// An item stack: one slot holding some quantity of an item type,
/// optionally containing liquid (for containers).
#[derive(Clone, Debug)]
pub struct ItemStack {
    pub item_id: u16,
    pub count: u16,
    pub liquid: Option<(LiquidType, u16)>,
    /// Freshness: 1.0 = fresh, 0.0 = spoiled. Only meaningful for items with spoil_time > 0.
    pub freshness: f32,
    /// Durability: remaining uses. 0 = broken. Only meaningful for tools with max_durability > 0.
    pub durability: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LiquidType {
    Water,
}

impl PartialEq for ItemStack {
    fn eq(&self, other: &Self) -> bool {
        self.item_id == other.item_id && self.count == other.count && self.liquid == other.liquid
    }
}

impl ItemStack {
    pub fn new(item_id: u16, count: u16) -> Self {
        // Auto-set durability from item def
        let dur = ItemRegistry::cached()
            .get(item_id)
            .map(|d| d.max_durability)
            .unwrap_or(0);
        Self {
            item_id,
            count,
            liquid: None,
            freshness: 1.0,
            durability: dur,
        }
    }

    pub fn label(&self) -> String {
        let reg = ItemRegistry::cached();
        let name = reg.name(self.item_id);
        let mut s = if self.count == 1 {
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
        };
        // Freshness indicator for perishable items
        let spoilable = reg.get(self.item_id).is_some_and(|d| d.spoil_time > 0.0);
        if spoilable && self.freshness < 0.99 {
            if self.freshness < 0.25 {
                s += " (rotting)";
            } else if self.freshness < 0.5 {
                s += " (stale)";
            } else if self.freshness < 0.75 {
                s += " (aging)";
            }
        }
        s
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

const MAX_ITEMS: usize = 900;

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

//! Resource types, inventories, and storage crate contents.

/// Maximum items a single storage crate can hold.
pub const CRATE_MAX_ITEMS: u32 = 10;

/// Inventory of a storage crate.
#[derive(Clone, Debug, Default)]
pub struct CrateInventory {
    pub rocks: u32,
    pub berries: u32,
}

impl CrateInventory {
    pub fn total(&self) -> u32 { self.rocks + self.berries }
    pub fn space(&self) -> u32 { CRATE_MAX_ITEMS.saturating_sub(self.total()) }
}

/// What a pleb is currently carrying in their hands.
#[derive(Clone, Debug, Default)]
pub struct PlebInventory {
    pub berries: u32,
    pub rocks: u32,
    pub carrying: Option<&'static str>,
}

/// Item type for ground items.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemKind {
    Berries(u32),
    Rocks(u32),
    Wood(u32),
}

impl ItemKind {
    pub fn label(&self) -> String {
        match self {
            ItemKind::Berries(n) => format!("{} berries", n),
            ItemKind::Rocks(n) => format!("{} rocks", n),
            ItemKind::Wood(n) => format!("{} wood", n),
        }
    }
    pub fn count(&self) -> u32 {
        match self { ItemKind::Berries(n) | ItemKind::Rocks(n) | ItemKind::Wood(n) => *n }
    }
}

/// An item sitting on the ground, waiting to be hauled.
#[derive(Clone, Debug)]
pub struct GroundItem {
    pub x: f32,
    pub y: f32,
    pub kind: ItemKind,
}

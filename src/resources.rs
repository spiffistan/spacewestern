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

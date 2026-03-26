//! Resource types, inventories, and storage crate contents.
//! Uses the data-driven item system from item_defs.

use crate::item_defs::*;

/// Maximum items a single storage crate can hold.
pub const CRATE_MAX_ITEMS: u32 = 10;

/// Inventory of a storage crate — holds mixed item stacks.
#[derive(Clone, Debug, Default)]
pub struct CrateInventory {
    pub stacks: Vec<ItemStack>,
}

impl CrateInventory {
    pub fn total(&self) -> u32 {
        self.stacks.iter().map(|s| s.count as u32).sum()
    }

    pub fn space(&self) -> u32 {
        CRATE_MAX_ITEMS.saturating_sub(self.total())
    }

    /// Count of a specific item type.
    pub fn count_of(&self, item_id: u16) -> u32 {
        self.stacks
            .iter()
            .filter(|s| s.item_id == item_id)
            .map(|s| s.count as u32)
            .sum()
    }

    /// Add items, respecting capacity. Returns how many were actually added.
    #[allow(dead_code)]
    pub fn add(&mut self, item_id: u16, count: u16) -> u16 {
        let can_add = (self.space() as u16).min(count);
        if can_add == 0 {
            return 0;
        }
        // Don't merge containers (they have unique liquid state)
        let is_container = ItemRegistry::cached()
            .get(item_id)
            .map(|d| d.liquid_capacity > 0)
            .unwrap_or(false);
        if !is_container {
            if let Some(stack) = self.stacks.iter_mut().find(|s| s.item_id == item_id) {
                stack.count += can_add;
                return can_add;
            }
        }
        self.stacks.push(ItemStack::new(item_id, can_add));
        can_add
    }

    /// Store a full ItemStack (preserves liquid contents for containers).
    /// Returns true if stored successfully.
    pub fn add_stack(&mut self, stack: ItemStack) -> bool {
        if self.space() < stack.count as u32 {
            return false;
        }
        let is_container = stack.is_container();
        if !is_container {
            if let Some(existing) = self.stacks.iter_mut().find(|s| s.item_id == stack.item_id) {
                existing.count += stack.count;
                return true;
            }
        }
        self.stacks.push(stack);
        true
    }

    /// Remove items. Returns how many were actually removed.
    pub fn remove(&mut self, item_id: u16, count: u16) -> u16 {
        if let Some(stack) = self.stacks.iter_mut().find(|s| s.item_id == item_id) {
            let take = stack.count.min(count);
            stack.count -= take;
            if stack.count == 0 {
                self.stacks.retain(|s| s.count > 0);
            }
            take
        } else {
            0
        }
    }
}

/// What a pleb is currently carrying.
#[derive(Clone, Debug, Default)]
pub struct PlebInventory {
    pub stacks: Vec<ItemStack>,
}

impl PlebInventory {
    /// Count of a specific item type in inventory.
    pub fn count_of(&self, item_id: u16) -> u32 {
        self.stacks
            .iter()
            .filter(|s| s.item_id == item_id)
            .map(|s| s.count as u32)
            .sum()
    }

    /// Add items to inventory (merges into existing stack or creates new).
    pub fn add(&mut self, item_id: u16, count: u16) {
        if count == 0 {
            return;
        }
        let is_container = ItemRegistry::cached()
            .get(item_id)
            .map(|d| d.liquid_capacity > 0)
            .unwrap_or(false);
        if !is_container {
            if let Some(stack) = self.stacks.iter_mut().find(|s| s.item_id == item_id) {
                stack.count += count;
                return;
            }
        }
        self.stacks.push(ItemStack::new(item_id, count));
    }

    /// Add a full ItemStack (preserves liquid for containers).
    pub fn add_stack(&mut self, stack: ItemStack) {
        if stack.count == 0 {
            return;
        }
        if !stack.is_container() {
            if let Some(existing) = self.stacks.iter_mut().find(|s| s.item_id == stack.item_id) {
                existing.count += stack.count;
                return;
            }
        }
        self.stacks.push(stack);
    }

    /// Remove items from inventory. Returns how many were actually removed.
    pub fn remove(&mut self, item_id: u16, count: u16) -> u16 {
        if let Some(stack) = self.stacks.iter_mut().find(|s| s.item_id == item_id) {
            let take = stack.count.min(count);
            stack.count -= take;
            if stack.count == 0 {
                self.stacks.retain(|s| s.count > 0);
            }
            take
        } else {
            0
        }
    }

    /// Is the pleb carrying anything?
    pub fn is_carrying(&self) -> bool {
        !self.stacks.is_empty()
    }

    /// What item type is the pleb primarily carrying? (first non-empty stack)
    #[allow(dead_code)]
    pub fn carrying_type(&self) -> Option<u16> {
        self.stacks.first().map(|s| s.item_id)
    }

    /// Label for what's being carried.
    pub fn carrying_label(&self) -> String {
        if let Some(stack) = self.stacks.first() {
            stack.label()
        } else {
            "Nothing".to_string()
        }
    }

    pub fn wood(&self) -> u32 {
        self.count_of(ITEM_WOOD)
    }
}

/// An item sitting on the ground, waiting to be hauled.
#[derive(Clone, Debug)]
pub struct GroundItem {
    pub x: f32,
    pub y: f32,
    pub stack: ItemStack,
}

impl GroundItem {
    pub fn new(x: f32, y: f32, item_id: u16, count: u16) -> Self {
        Self {
            x,
            y,
            stack: ItemStack::new(item_id, count),
        }
    }
}

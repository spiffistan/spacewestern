//! Rayworld library — re-exports modules for integration tests.

/// Check if a value matches any of the given block type constants.
#[macro_export]
macro_rules! bt_is {
    ($val:expr, $($bt:expr),+ $(,)?) => {
        $( $val == $bt )||+
    }
}

pub mod audio;
pub mod block_defs;
pub mod build;
pub mod camera;
pub mod cards;
pub mod comms;
pub mod creature_defs;
pub mod creatures;
pub mod dust;
pub mod fluid;
pub mod grid;
pub mod item_defs;
pub mod materials;
pub mod morale;
pub mod needs;
pub mod physics;
pub mod pipes;
pub mod pleb;
pub mod recipe_defs;
pub mod resources;
pub mod rooms;
pub mod sprites;
pub mod terrain;
pub mod theme;
pub mod weather;
pub mod zones;

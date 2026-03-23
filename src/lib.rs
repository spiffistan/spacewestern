//! Rayworld library — re-exports modules for integration tests.

/// Check if a value matches any of the given block type constants.
#[macro_export]
macro_rules! bt_is {
    ($val:expr, $($bt:expr),+ $(,)?) => {
        $( $val == $bt )||+
    }
}

pub mod materials;
pub mod grid;
pub mod sprites;
pub mod block_defs;
pub mod pleb;
pub mod needs;
pub mod build;
pub mod camera;
pub mod fluid;
pub mod pipes;
pub mod physics;
pub mod zones;
pub mod weather;
pub mod resources;
pub mod item_defs;
pub mod recipe_defs;

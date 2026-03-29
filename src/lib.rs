//! Rayworld library — re-exports modules for integration tests.

/// Check if a value matches any of the given block type constants.
#[macro_export]
macro_rules! bt_is {
    ($val:expr, $($bt:expr),+ $(,)?) => {
        $( $val == $bt )||+
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod audio;
pub mod block_defs;
pub mod build;
pub mod camera;
pub mod creature_defs;
pub mod creatures;
pub mod fluid;
pub mod grid;
pub mod item_defs;
pub mod materials;
pub mod needs;
pub mod physics;
pub mod pipes;
pub mod pleb;
pub mod recipe_defs;
pub mod resources;
pub mod sprites;
pub mod weather;
pub mod zones;

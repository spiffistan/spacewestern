//! Terrain generation module for the voxel world.
//!
//! This module handles procedural terrain generation for a flat world
//! with surface detail support (voxels down to 1/10 size on the surface).

use std::collections::HashMap;

/// World dimensions
pub const WORLD_SIZE_X: u32 = 1000;
pub const WORLD_SIZE_Y: u32 = 20;  // Depth (vertical)
pub const WORLD_SIZE_Z: u32 = 1000;

/// Base voxel size
pub const VOXEL_SIZE: f32 = 1.0;

/// Detail level for surface voxels (1/10 size)
pub const DETAIL_SCALE: f32 = 0.1;
pub const DETAIL_SUBDIVISIONS: u32 = 10;

/// Voxel types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VoxelType {
    Air = 0,
    Grass = 1,
    Dirt = 2,
    Stone = 3,
    Sand = 4,
    Water = 5,
    Gravel = 6,
}

impl VoxelType {
    pub fn is_solid(&self) -> bool {
        !matches!(self, VoxelType::Air | VoxelType::Water)
    }
}

/// A detail voxel represents a 1/10 size voxel within a surface cell
#[derive(Clone, Copy, Debug)]
pub struct DetailVoxel {
    pub voxel_type: VoxelType,
    /// Height offset within the detail cell (0.0 to 1.0)
    pub height_offset: f32,
}

/// Terrain configuration
#[derive(Clone, Debug)]
pub struct TerrainConfig {
    /// Base terrain height (in voxels from bottom)
    pub base_height: u32,
    /// Maximum height variation
    pub height_variation: f32,
    /// Noise frequency for terrain undulation
    pub noise_frequency: f32,
    /// Whether to generate surface detail
    pub enable_surface_detail: bool,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            base_height: 10,
            height_variation: 3.0,
            noise_frequency: 0.05,
            enable_surface_detail: true,
        }
    }
}

/// Terrain generator
pub struct TerrainGenerator {
    config: TerrainConfig,
    /// Cache for surface detail voxels (keyed by world grid position)
    detail_cache: HashMap<(i32, i32, i32), Vec<DetailVoxel>>,
}

impl TerrainGenerator {
    pub fn new(config: TerrainConfig) -> Self {
        Self {
            config,
            detail_cache: HashMap::new(),
        }
    }

    /// Simple hash function for procedural generation
    fn hash(x: f32, z: f32) -> f32 {
        let p = (x * 0.3183099 + 0.1, z * 0.3183099 + 0.1);
        let p = (p.0.fract() * 17.0, p.1.fract() * 17.0);
        (p.0 * p.1 * (p.0 + p.1)).fract()
    }

    /// Generate height at a given world position
    pub fn get_height(&self, x: f32, z: f32) -> f32 {
        let base = self.config.base_height as f32;
        let freq = self.config.noise_frequency;
        let var = self.config.height_variation;

        // Simple layered noise for gentle hills
        let mut height = base;
        height += (x * freq).sin() * var * 0.5;
        height += (z * freq).cos() * var * 0.5;
        height += ((x + z) * freq * 0.7).sin() * var * 0.3;

        height
    }

    /// Get the voxel type at a grid position
    pub fn get_voxel(&self, x: i32, y: i32, z: i32) -> VoxelType {
        // Bounds check
        if x < 0 || x >= WORLD_SIZE_X as i32 ||
           y < 0 || y >= WORLD_SIZE_Y as i32 ||
           z < 0 || z >= WORLD_SIZE_Z as i32 {
            return VoxelType::Air;
        }

        let height = self.get_height(x as f32, z as f32);
        let terrain_y = height as i32;

        if y > terrain_y {
            VoxelType::Air
        } else if y == terrain_y {
            VoxelType::Grass
        } else if y >= terrain_y - 3 {
            VoxelType::Dirt
        } else {
            VoxelType::Stone
        }
    }

    /// Get surface detail at a position (for 1/10 size voxels)
    /// Returns the height offset (0.0 to 1.0) for smooth terrain
    pub fn get_surface_detail(&self, x: f32, z: f32) -> f32 {
        if !self.config.enable_surface_detail {
            return 0.0;
        }

        let height = self.get_height(x, z);
        height.fract()
    }

    /// Check if a position is at the surface (for detail rendering)
    pub fn is_surface(&self, x: i32, y: i32, z: i32) -> bool {
        let height = self.get_height(x as f32, z as f32);
        let terrain_y = height as i32;
        y == terrain_y
    }

    /// Generate terrain data for GPU upload
    /// Returns a flat array of voxel types for the entire world
    pub fn generate_world_data(&self) -> Vec<u8> {
        let size = (WORLD_SIZE_X * WORLD_SIZE_Y * WORLD_SIZE_Z) as usize;
        let mut data = vec![0u8; size];

        for z in 0..WORLD_SIZE_Z {
            for y in 0..WORLD_SIZE_Y {
                for x in 0..WORLD_SIZE_X {
                    let idx = (z * WORLD_SIZE_Y * WORLD_SIZE_X + y * WORLD_SIZE_X + x) as usize;
                    data[idx] = self.get_voxel(x as i32, y as i32, z as i32) as u8;
                }
            }
        }

        data
    }

    /// Generate heightmap for GPU (stores height at each x,z position)
    pub fn generate_heightmap(&self) -> Vec<f32> {
        let size = (WORLD_SIZE_X * WORLD_SIZE_Z) as usize;
        let mut heights = vec![0.0f32; size];

        for z in 0..WORLD_SIZE_Z {
            for x in 0..WORLD_SIZE_X {
                let idx = (z * WORLD_SIZE_X + x) as usize;
                heights[idx] = self.get_height(x as f32, z as f32);
            }
        }

        heights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_generation() {
        let gen = TerrainGenerator::new(TerrainConfig::default());

        // Check that surface is grass
        let height = gen.get_height(50.0, 50.0);
        let voxel = gen.get_voxel(50, height as i32, 50);
        assert_eq!(voxel, VoxelType::Grass);

        // Check below surface is dirt
        let voxel = gen.get_voxel(50, height as i32 - 1, 50);
        assert_eq!(voxel, VoxelType::Dirt);

        // Check deep is stone
        let voxel = gen.get_voxel(50, 0, 50);
        assert_eq!(voxel, VoxelType::Stone);

        // Check above surface is air
        let voxel = gen.get_voxel(50, height as i32 + 1, 50);
        assert_eq!(voxel, VoxelType::Air);
    }
}

//! Terrain generation module for the voxel world.
//!
//! This module handles procedural terrain generation for a flat world
//! with surface detail support (voxels down to 1/10 size on the surface).

use std::collections::HashMap;
use std::collections::HashSet;

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

    /// Cast a ray and find the first voxel it hits
    /// Returns (grid_x, grid_y, grid_z, voxel_type) or None if no hit
    pub fn raycast(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_dist: f32,
        removed_voxels: &HashSet<(i32, i32, i32)>,
        placed_voxels: &HashMap<(i32, i32, i32), u8>,
    ) -> Option<(i32, i32, i32, u8)> {
        // World bounds (centered at origin, Y starts at 0)
        let world_min = [
            -(WORLD_SIZE_X as f32) / 2.0,
            0.0,
            -(WORLD_SIZE_Z as f32) / 2.0,
        ];
        let world_max = [
            (WORLD_SIZE_X as f32) / 2.0,
            WORLD_SIZE_Y as f32,
            (WORLD_SIZE_Z as f32) / 2.0,
        ];

        // Intersect world bounds
        let inv_dir = [
            if direction[0] != 0.0 { 1.0 / direction[0] } else { 1e30 },
            if direction[1] != 0.0 { 1.0 / direction[1] } else { 1e30 },
            if direction[2] != 0.0 { 1.0 / direction[2] } else { 1e30 },
        ];

        let t0 = [
            (world_min[0] - origin[0]) * inv_dir[0],
            (world_min[1] - origin[1]) * inv_dir[1],
            (world_min[2] - origin[2]) * inv_dir[2],
        ];
        let t1 = [
            (world_max[0] - origin[0]) * inv_dir[0],
            (world_max[1] - origin[1]) * inv_dir[1],
            (world_max[2] - origin[2]) * inv_dir[2],
        ];

        let tmin = [t0[0].min(t1[0]), t0[1].min(t1[1]), t0[2].min(t1[2])];
        let tmax = [t0[0].max(t1[0]), t0[1].max(t1[1]), t0[2].max(t1[2])];

        let t_enter = tmin[0].max(tmin[1]).max(tmin[2]).max(0.001);
        let t_exit = tmax[0].min(tmax[1]).min(tmax[2]);

        if t_enter >= t_exit {
            return None;
        }

        // Entry point
        let entry = [
            origin[0] + direction[0] * t_enter,
            origin[1] + direction[1] * t_enter,
            origin[2] + direction[2] * t_enter,
        ];

        // Transform to grid space
        let ray_pos = [
            entry[0] - world_min[0],
            entry[1].clamp(0.001, WORLD_SIZE_Y as f32 - 0.001),
            entry[2] - world_min[2],
        ];

        let mut map_pos = [
            ray_pos[0].floor() as i32,
            ray_pos[1].floor() as i32,
            ray_pos[2].floor() as i32,
        ];

        let ray_step = [
            if direction[0] >= 0.0 { 1 } else { -1 },
            if direction[1] >= 0.0 { 1 } else { -1 },
            if direction[2] >= 0.0 { 1 } else { -1 },
        ];

        let delta_dist = [
            (1.0 / direction[0]).abs().min(1e30),
            (1.0 / direction[1]).abs().min(1e30),
            (1.0 / direction[2]).abs().min(1e30),
        ];

        let mut side_dist = [
            if direction[0] < 0.0 {
                (ray_pos[0] - map_pos[0] as f32) * delta_dist[0]
            } else {
                (map_pos[0] as f32 + 1.0 - ray_pos[0]) * delta_dist[0]
            },
            if direction[1] < 0.0 {
                (ray_pos[1] - map_pos[1] as f32) * delta_dist[1]
            } else {
                (map_pos[1] as f32 + 1.0 - ray_pos[1]) * delta_dist[1]
            },
            if direction[2] < 0.0 {
                (ray_pos[2] - map_pos[2] as f32) * delta_dist[2]
            } else {
                (map_pos[2] as f32 + 1.0 - ray_pos[2]) * delta_dist[2]
            },
        ];

        for _ in 0..256 {
            // Bounds check
            if map_pos[0] < 0 || map_pos[0] >= WORLD_SIZE_X as i32 ||
               map_pos[1] < 0 || map_pos[1] >= WORLD_SIZE_Y as i32 ||
               map_pos[2] < 0 || map_pos[2] >= WORLD_SIZE_Z as i32 {
                break;
            }

            let pos_key = (map_pos[0], map_pos[1], map_pos[2]);

            // Check placed voxels first
            if let Some(&voxel_type) = placed_voxels.get(&pos_key) {
                if voxel_type > 0 {
                    return Some((map_pos[0], map_pos[1], map_pos[2], voxel_type));
                }
            } else if !removed_voxels.contains(&pos_key) {
                // Check procedural terrain
                let voxel = self.get_voxel(map_pos[0], map_pos[1], map_pos[2]);
                if voxel.is_solid() {
                    return Some((map_pos[0], map_pos[1], map_pos[2], voxel as u8));
                }
            }

            // DDA step
            if side_dist[0] < side_dist[1] {
                if side_dist[0] < side_dist[2] {
                    side_dist[0] += delta_dist[0];
                    map_pos[0] += ray_step[0];
                } else {
                    side_dist[2] += delta_dist[2];
                    map_pos[2] += ray_step[2];
                }
            } else {
                if side_dist[1] < side_dist[2] {
                    side_dist[1] += delta_dist[1];
                    map_pos[1] += ray_step[1];
                } else {
                    side_dist[2] += delta_dist[2];
                    map_pos[2] += ray_step[2];
                }
            }
        }

        None
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

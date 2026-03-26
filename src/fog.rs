//! Fog of war — recursive shadowcasting visibility + exploration memory.
//!
//! Uses 8-octant recursive shadowcasting (standard roguelike algorithm).
//! Walls block vision, glass/trees partially transmit.
//! At night, outdoor vision requires a torch/headlight.

use crate::grid::*;
use crate::pleb::Pleb;

pub const DEFAULT_VISION_RADIUS: i32 = 25;
const TORCH_VISION_RADIUS: i32 = 8;
const SUN_THRESHOLD: f32 = 0.1; // below this, it's "night" for fog purposes

/// Fog value constants for the GPU texture (R8Unorm: 0-255 → 0.0-1.0)
const FOG_SHROUDED: u8 = 0; // 0.0 — completely black
const FOG_EXPLORED: u8 = 76; // ~0.3 — dimmed, desaturated
const FOG_VISIBLE: u8 = 255; // 1.0 — full rendering

/// Returns true if the tile at (x, y) blocks line of sight.
/// For thin walls, only blocks if the sightline crosses a walled edge.
/// `from_x, from_y` is the tile the sightline is coming from (toward viewer).
fn blocks_vision(
    grid: &[u32],
    wall_data: &[u16],
    x: i32,
    y: i32,
    from_x: i32,
    from_y: i32,
) -> bool {
    if x < 0 || y < 0 || x >= GRID_W as i32 || y >= GRID_H as i32 {
        return true; // out of bounds blocks
    }
    let idx = (y as u32 * GRID_W + x as u32) as usize;
    let block = grid[idx];
    let bt = block_type_rs(block);
    let bh = block_height_rs(block);

    if bh == 0 {
        return false;
    } // no height = no block

    // Doors: open doors don't block
    let flags = block_flags_rs(block);
    let is_door = flags & 1 != 0;
    let is_open = flags & 4 != 0;
    if is_door && is_open {
        return false;
    }

    // Glass and trees: don't block (can see through)
    if bt_is!(bt, BT_GLASS, BT_TREE, BT_BERRY_BUSH, BT_CROP) {
        return false;
    }

    // Thin walls: only block if the sightline crosses a walled edge
    if is_wall_block(bt) && thin_wall_is_walkable(block) {
        // Check if edge between from→here is walled
        return edge_blocked_wd(grid, wall_data, from_x, from_y, x, y);
    }

    // Everything else with height blocks
    true
}

/// Recursive shadowcasting for one octant.
/// Uses the "Restrictive Precise Angle Shadowcasting" variant.
fn cast_light(
    grid: &[u32],
    wall_data: &[u16],
    visibility: &mut [u8],
    cx: i32,
    cy: i32,
    radius: i32,
    row: i32,
    start_slope: f32,
    end_slope: f32,
    xx: i32,
    xy: i32,
    yx: i32,
    yy: i32,
) {
    if start_slope < end_slope {
        return;
    }

    let mut start = start_slope;
    let radius_sq = radius * radius;

    for j in row..=radius {
        let mut blocked = false;
        let mut new_start = 0.0f32;

        let dy = -j;
        let mut dx = dy; // scan from left to right

        while dx <= 0 {
            // Transform to map coordinates
            let mx = cx + dx * xx + dy * xy;
            let my = cy + dx * yx + dy * yy;

            // Slope of the left and right edges of this cell
            let l_slope = (dx as f32 - 0.5) / (dy as f32 + 0.5);
            let r_slope = (dx as f32 + 0.5) / (dy as f32 - 0.5);

            if start < r_slope {
                dx += 1;
                continue;
            }
            if end_slope > l_slope {
                break;
            }

            // Within radius? Mark visible
            if dx * dx + dy * dy <= radius_sq {
                if mx >= 0 && my >= 0 && mx < GRID_W as i32 && my < GRID_H as i32 {
                    visibility[(my as u32 * GRID_W + mx as u32) as usize] = 255;
                }
            }

            // "From" cell: one step closer to origin along the row direction
            let from_x = cx + dx * xx + (dy + 1) * xy;
            let from_y = cy + dx * yx + (dy + 1) * yy;

            if blocked {
                // Previous cell was a wall
                if blocks_vision(grid, wall_data, mx, my, from_x, from_y) {
                    // Still a wall — adjust start slope
                    new_start = r_slope;
                } else {
                    // Wall ended — start a new scan
                    blocked = false;
                    start = new_start;
                }
            } else if blocks_vision(grid, wall_data, mx, my, from_x, from_y) && j < radius {
                // Hit a wall — recurse with narrowed range, then mark blocked
                blocked = true;
                cast_light(
                    grid,
                    wall_data,
                    visibility,
                    cx,
                    cy,
                    radius,
                    j + 1,
                    start,
                    l_slope,
                    xx,
                    xy,
                    yx,
                    yy,
                );
                new_start = r_slope;
            }

            dx += 1;
        }

        if blocked {
            break;
        }
    }
}

/// Compute full 360° shadowcasting visibility from a single point.
fn compute_fov(
    grid: &[u32],
    wall_data: &[u16],
    visibility: &mut [u8],
    cx: i32,
    cy: i32,
    radius: i32,
) {
    // Mark origin visible
    if cx >= 0 && cy >= 0 && cx < GRID_W as i32 && cy < GRID_H as i32 {
        visibility[(cy as u32 * GRID_W + cx as u32) as usize] = 255;
    }

    // 8 octant multipliers — maps (dx, dy) in octant-local space to world offsets.
    // Each octant scans a 45° wedge.
    const MULTIPLIERS: [[i32; 4]; 8] = [
        [1, 0, 0, 1],
        [0, 1, 1, 0],
        [0, -1, 1, 0],
        [-1, 0, 0, 1],
        [-1, 0, 0, -1],
        [0, -1, -1, 0],
        [0, 1, -1, 0],
        [1, 0, 0, -1],
    ];

    for m in &MULTIPLIERS {
        cast_light(
            grid, wall_data, visibility, cx, cy, radius, 1, 1.0, 0.0, m[0], m[1], m[2], m[3],
        );
    }
}

/// Update fog of war state. Returns true if fog texture changed (needs re-upload).
///
/// `sun_intensity`: current sun brightness (0-1). Below SUN_THRESHOLD, outdoor
/// vision is limited to torch/headlight range.
pub fn update_fog(
    grid: &[u32],
    wall_data: &[u16],
    plebs: &[Pleb],
    sun_intensity: f32,
    vision_radius: i32,
    fog_visibility: &mut [u8],
    fog_explored: &mut [u8],
    fog_texture_data: &mut [u8],
    prev_tiles: &mut Vec<(i32, i32)>,
) -> bool {
    let grid_size = (GRID_W * GRID_H) as usize;

    // Check if any colonist changed tiles
    let current_tiles: Vec<(i32, i32)> = plebs
        .iter()
        .filter(|p| !p.is_enemy)
        .map(|p| (p.x.floor() as i32, p.y.floor() as i32))
        .collect();

    if current_tiles == *prev_tiles && !prev_tiles.is_empty() {
        return false; // no change
    }
    *prev_tiles = current_tiles.clone();

    // Clear visibility (recomputed from scratch)
    fog_visibility.iter_mut().for_each(|v| *v = 0);

    let is_night = sun_intensity < SUN_THRESHOLD;

    // Compute visibility for each colonist
    for pleb in plebs.iter().filter(|p| !p.is_enemy) {
        let px = pleb.x.floor() as i32;
        let py = pleb.y.floor() as i32;

        // Full vision radius with shadowcasting
        let radius = if is_night && !pleb.torch_on && !pleb.headlight_on {
            // Nighttime without light: very limited vision
            3
        } else if is_night {
            // Nighttime with torch/headlight: reduced radius
            TORCH_VISION_RADIUS.min(vision_radius)
        } else {
            // Daytime: full vision
            vision_radius
        };

        compute_fov(grid, wall_data, fog_visibility, px, py, radius);
    }

    // Update explored (union: once explored, always explored)
    for i in 0..grid_size {
        if fog_visibility[i] > 0 {
            fog_explored[i] = 255;
        }
    }

    // Compose fog texture data for GPU
    for i in 0..grid_size {
        fog_texture_data[i] = if fog_visibility[i] > 0 {
            FOG_VISIBLE
        } else if fog_explored[i] > 0 {
            FOG_EXPLORED
        } else {
            FOG_SHROUDED
        };
    }

    true // changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::make_block;

    fn empty_grid() -> Vec<u32> {
        vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize]
    }

    #[test]
    fn test_origin_is_visible() {
        let grid = empty_grid();
        let mut vis = vec![0u8; (GRID_W * GRID_H) as usize];
        compute_fov(&grid, &[], &mut vis, 128, 128, 10);
        assert_eq!(vis[(128 * GRID_W + 128) as usize], 255);
    }

    #[test]
    fn test_open_area_visible() {
        let grid = empty_grid();
        let mut vis = vec![0u8; (GRID_W * GRID_H) as usize];
        compute_fov(&grid, &[], &mut vis, 128, 128, 5);
        // Adjacent tiles should be visible
        assert_eq!(vis[(128 * GRID_W + 129) as usize], 255);
        assert_eq!(vis[(129 * GRID_W + 128) as usize], 255);
        assert_eq!(vis[(127 * GRID_W + 128) as usize], 255);
    }

    #[test]
    fn test_wall_blocks_vision() {
        let mut grid = empty_grid();
        // Place a wall at (130, 128)
        let wall_idx = (128 * GRID_W + 130) as usize;
        grid[wall_idx] = make_block(BT_WALL as u8, 3, 0);

        let mut vis = vec![0u8; (GRID_W * GRID_H) as usize];
        compute_fov(&grid, &[], &mut vis, 128, 128, 10);

        // Tile behind wall (131, 128) should NOT be visible
        assert_eq!(
            vis[(128 * GRID_W + 131) as usize],
            0,
            "tile behind wall should be hidden"
        );
        // Tile before wall (129, 128) should be visible
        assert_eq!(
            vis[(128 * GRID_W + 129) as usize],
            255,
            "tile before wall should be visible"
        );
    }

    #[test]
    fn test_glass_does_not_block() {
        let mut grid = empty_grid();
        let glass_idx = (128 * GRID_W + 130) as usize;
        grid[glass_idx] = make_block(BT_GLASS as u8, 3, 0);

        let mut vis = vec![0u8; (GRID_W * GRID_H) as usize];
        compute_fov(&grid, &[], &mut vis, 128, 128, 10);

        // Tile behind glass should be visible
        assert_eq!(
            vis[(128 * GRID_W + 131) as usize],
            255,
            "should see through glass"
        );
    }

    #[test]
    fn test_radius_limit() {
        let grid = empty_grid();
        let mut vis = vec![0u8; (GRID_W * GRID_H) as usize];
        compute_fov(&grid, &[], &mut vis, 128, 128, 5);

        // Tile at distance 6 should NOT be visible
        assert_eq!(
            vis[(128 * GRID_W + 135) as usize],
            0,
            "beyond radius should be hidden"
        );
        // Tile at distance 4 should be visible
        assert_eq!(
            vis[(128 * GRID_W + 132) as usize],
            255,
            "within radius should be visible"
        );
    }

    #[test]
    fn test_thin_wall_blocks_directionally() {
        let mut grid = empty_grid();
        // Place a thin wall at (130, 128) with wall on WEST edge (blocks vision from west)
        // Viewer at (128, 128) looks east toward (130, 128)
        let (flags, edge_mask) = make_thin_wall_flags(0, 3, 1); // edge=W, thickness=1
        let wall_idx = (128 * GRID_W + 130) as usize;
        grid[wall_idx] = make_block(BT_WALL as u8, make_wall_height(3, edge_mask), flags);

        let mut vis = vec![0u8; (GRID_W * GRID_H) as usize];
        compute_fov(&grid, &[], &mut vis, 128, 128, 10);

        // Tile behind the thin wall's walled edge (131, 128) should NOT be visible
        assert_eq!(
            vis[(128 * GRID_W + 131) as usize],
            0,
            "tile behind thin wall's walled edge should be hidden"
        );
        // Tile before the wall (129, 128) should be visible
        assert_eq!(
            vis[(128 * GRID_W + 129) as usize],
            255,
            "tile before thin wall should be visible"
        );
    }

    #[test]
    fn test_thin_wall_transparent_from_open_side() {
        let mut grid = empty_grid();
        // Place a thin wall at (130, 128) with wall on NORTH edge only
        // Viewer at (128, 128) looks east — should see through the open part
        let (flags, edge_mask) = make_thin_wall_flags(0, 0, 1); // edge=N, thickness=1
        let wall_idx = (128 * GRID_W + 130) as usize;
        grid[wall_idx] = make_block(BT_WALL as u8, make_wall_height(3, edge_mask), flags);

        let mut vis = vec![0u8; (GRID_W * GRID_H) as usize];
        compute_fov(&grid, &[], &mut vis, 128, 128, 10);

        // Tile behind the wall (131, 128): visible because E-W sightline
        // doesn't cross the north edge wall
        assert_eq!(
            vis[(128 * GRID_W + 131) as usize],
            255,
            "should see through thin wall's open side"
        );
    }
}

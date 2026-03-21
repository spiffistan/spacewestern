//! Pleb (colonist) — struct, appearance, movement, A* pathfinding, activity.

use crate::grid::*;
use crate::needs::PlebNeeds;
use crate::materials::{build_material_table, NUM_MATERIALS};
use std::sync::OnceLock;

/// Cached walkable lookup table indexed by block type. True = walkable at height 0.
static WALKABLE_TABLE: OnceLock<[bool; NUM_MATERIALS]> = OnceLock::new();

fn walkable_table() -> &'static [bool; NUM_MATERIALS] {
    WALKABLE_TABLE.get_or_init(|| {
        let mats = build_material_table();
        let mut table = [false; NUM_MATERIALS];
        for (i, m) in mats.iter().enumerate() {
            table[i] = m.walkable > 0.5;
        }
        table
    })
}

/// Check if a block type is walkable (from material table).
fn is_type_walkable(bt: u32) -> bool {
    if (bt as usize) < NUM_MATERIALS {
        walkable_table()[bt as usize]
    } else {
        false
    }
}

/// What the pleb is currently doing.
#[derive(Clone, Debug, PartialEq)]
pub enum PlebActivity {
    Idle,
    Walking,          // following a path (player-ordered or auto)
    Sleeping,         // in bed, recovering rest
    Harvesting(f32),  // progress 0-1, harvesting a berry bush at nearby tile
    Eating,           // consuming food (quick action)
    Hauling,          // carrying item to a storage crate
    Farming(f32),     // progress 0-1, planting or harvesting a crop
    /// Crisis override — pleb acts autonomously, ignoring player input.
    /// Inner activity is what they're doing (Walking to food/bed, Harvesting, Eating, Sleeping).
    Crisis(Box<PlebActivity>, &'static str), // (inner_activity, reason_label)
}

impl PlebActivity {
    /// Returns true if the pleb is in a crisis state (player input blocked).
    pub fn is_crisis(&self) -> bool {
        matches!(self, PlebActivity::Crisis(_, _))
    }

    /// Get the inner activity (unwraps crisis wrapper if present).
    pub fn inner(&self) -> &PlebActivity {
        match self {
            PlebActivity::Crisis(inner, _) => inner,
            other => other,
        }
    }

    /// Get the crisis reason label, if in crisis.
    pub fn crisis_reason(&self) -> Option<&'static str> {
        match self {
            PlebActivity::Crisis(_, reason) => Some(reason),
            _ => None,
        }
    }
}

pub use crate::resources::PlebInventory;

/// Appearance data for rendering a pleb (Rimworld-style).
#[derive(Clone, Debug)]
pub struct PlebAppearance {
    pub skin_r: f32, pub skin_g: f32, pub skin_b: f32,
    pub hair_r: f32, pub hair_g: f32, pub hair_b: f32,
    pub shirt_r: f32, pub shirt_g: f32, pub shirt_b: f32,
    pub pants_r: f32, pub pants_g: f32, pub pants_b: f32,
    pub hair_style: u32,  // 0=bald, 1=short, 2=medium, 3=long
}

impl PlebAppearance {
    /// Generate random appearance from a seed.
    pub fn random(seed: u32) -> Self {
        let hash = |i: u32| -> f32 {
            let h = seed.wrapping_mul(2654435761).wrapping_add(i.wrapping_mul(1013904223));
            (h & 0xFFFF) as f32 / 65535.0
        };

        // Skin tone range (warm tones)
        let skin_base = hash(0);
        let skin_r = 0.65 + skin_base * 0.30;
        let skin_g = 0.50 + skin_base * 0.25;
        let skin_b = 0.35 + skin_base * 0.20;

        // Hair color
        let hair_base = hash(1);
        let (hair_r, hair_g, hair_b) = if hair_base < 0.3 {
            (0.15 + hash(2) * 0.15, 0.10 + hash(2) * 0.10, 0.05) // dark brown/black
        } else if hair_base < 0.6 {
            (0.45 + hash(2) * 0.15, 0.30 + hash(2) * 0.10, 0.15) // brown
        } else if hair_base < 0.8 {
            (0.70 + hash(2) * 0.20, 0.55 + hash(2) * 0.15, 0.20) // blonde
        } else {
            (0.55 + hash(2) * 0.15, 0.15, 0.10) // red
        };

        // Shirt color (varied)
        let shirt_hue = hash(3);
        let (shirt_r, shirt_g, shirt_b) = if shirt_hue < 0.2 {
            (0.25, 0.40, 0.65) // blue
        } else if shirt_hue < 0.4 {
            (0.55, 0.30, 0.25) // red/brown
        } else if shirt_hue < 0.6 {
            (0.30, 0.50, 0.30) // green
        } else if shirt_hue < 0.8 {
            (0.55, 0.55, 0.50) // gray
        } else {
            (0.60, 0.50, 0.30) // tan
        };

        // Pants color (muted)
        let pants_hue = hash(4);
        let (pants_r, pants_g, pants_b) = if pants_hue < 0.4 {
            (0.25, 0.25, 0.35) // dark blue/gray
        } else if pants_hue < 0.7 {
            (0.35, 0.30, 0.20) // brown
        } else {
            (0.30, 0.30, 0.30) // dark gray
        };

        let hair_style = (hash(5) * 4.0) as u32;

        PlebAppearance {
            skin_r, skin_g, skin_b,
            hair_r, hair_g, hair_b,
            shirt_r, shirt_g, shirt_b,
            pants_r, pants_g, pants_b,
            hair_style,
        }
    }
}

/// GPU-side pleb data for rendering (packed for storage buffer).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuPleb {
    pub x: f32, pub y: f32, pub angle: f32, pub selected: f32,
    pub torch: f32, pub headlight: f32, pub carrying: f32, pub _pad1: f32,
    pub skin_r: f32, pub skin_g: f32, pub skin_b: f32, pub hair_style: f32,
    pub hair_r: f32, pub hair_g: f32, pub hair_b: f32, pub _pad2: f32,
    pub shirt_r: f32, pub shirt_g: f32, pub shirt_b: f32, pub _pad3: f32,
    pub pants_r: f32, pub pants_g: f32, pub pants_b: f32, pub _pad4: f32,
}

pub struct Pleb {
    pub id: usize,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub path: Vec<(i32, i32)>,
    pub path_idx: usize,
    pub torch_on: bool,
    pub headlight_on: bool,
    pub appearance: PlebAppearance,
    pub needs: PlebNeeds,
    pub prev_x: f32, // previous frame position (for detecting movement)
    pub prev_y: f32,
    pub activity: PlebActivity,
    pub inventory: PlebInventory,
    pub harvest_target: Option<(i32, i32)>, // grid coords of bush being harvested
    pub haul_target: Option<(i32, i32)>,    // grid coords of storage crate to deliver to
    pub is_enemy: bool,
    pub wander_timer: f32,
    pub work_target: Option<(i32, i32)>, // position of current work task
}

impl Pleb {
    pub fn new(id: usize, name: String, x: f32, y: f32, seed: u32) -> Self {
        Pleb {
            id, name, x, y,
            angle: 0.0,
            path: Vec::new(),
            path_idx: 0,
            torch_on: false,
            headlight_on: false,
            appearance: PlebAppearance::random(seed),
            needs: PlebNeeds::default(),
            prev_x: x,
            prev_y: y,
            activity: PlebActivity::Idle,
            inventory: PlebInventory::default(),
            harvest_target: None,
            haul_target: None,
            is_enemy: false,
            wander_timer: 0.0,
            work_target: None,
        }
    }

    pub fn to_gpu(&self, selected: bool) -> GpuPleb {
        let a = &self.appearance;
        GpuPleb {
            x: self.x, y: self.y, angle: self.angle,
            selected: if selected { 1.0 } else { 0.0 },
            torch: if self.torch_on { 1.0 } else { 0.0 },
            headlight: if self.headlight_on { 1.0 } else { 0.0 },
            carrying: if self.inventory.carrying.is_some() { 1.0 } else { 0.0 },
            _pad1: 0.0,
            skin_r: a.skin_r, skin_g: a.skin_g, skin_b: a.skin_b,
            hair_style: a.hair_style as f32,
            hair_r: a.hair_r, hair_g: a.hair_g, hair_b: a.hair_b, _pad2: 0.0,
            shirt_r: a.shirt_r, shirt_g: a.shirt_g, shirt_b: a.shirt_b, _pad3: 0.0,
            pants_r: a.pants_r, pants_g: a.pants_g, pants_b: a.pants_b, _pad4: 0.0,
        }
    }
}

pub const MAX_PLEBS: usize = 16;

/// Names pool for random pleb names.
const NAMES: &[&str] = &[
    "Jeff", "Sarah", "Marcus", "Elena", "Dmitri", "Yuki", "Carlos", "Amara",
    "Olaf", "Priya", "Liam", "Zara", "Kento", "Ingrid", "Rashid", "Mei",
];

pub fn random_name(seed: u32) -> String {
    let idx = (seed.wrapping_mul(2654435761) >> 16) as usize % NAMES.len();
    NAMES[idx].to_string()
}

/// Check if a pleb can stand at continuous position (x, y) using 4-corner bounding box.
pub fn is_walkable_pos(grid: &[u32], x: f32, y: f32) -> bool {
    let r = 0.25;
    for &(cx, cy) in &[(x - r, y - r), (x + r, y - r), (x - r, y + r), (x + r, y + r)] {
        let bx = cx.floor() as i32;
        let by = cy.floor() as i32;
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 { return false; }
        let b = grid[(by as u32 * GRID_W + bx as u32) as usize];
        let bt = b & 0xFF;
        let bh = (b >> 8) & 0xFF;
        let is_door = (b >> 16) & 1 != 0;
        let is_dug_shallow = bt == BT_DUG_GROUND && bh <= 1;
        let bt32 = bt as u32;
        let is_pipe = ((bt32 >= 15 && bt32 <= 20) || bt32 == BT_RESTRICTOR
            || bt32 == BT_LIQUID_PIPE || bt32 == BT_PIPE_BRIDGE
            || bt32 == BT_LIQUID_INTAKE || bt32 == BT_LIQUID_PUMP || bt32 == BT_LIQUID_OUTPUT) && bh <= 1;
        // Diagonal wall: check which side of the diagonal this corner is on
        if bt == BT_DIAGONAL {
            let variant = ((b >> 19) & 3) as u32;
            let lfx = cx - (cx.floor());
            let lfy = cy - (cy.floor());
            let on_wall = match variant {
                0 => lfy > (1.0 - lfx),
                1 => lfy > lfx,
                2 => lfy < (1.0 - lfx),
                _ => lfy < lfx,
            };
            if on_wall { return false; }
            continue; // open half is walkable
        }
        if !is_door && !is_dug_shallow && !is_pipe && (bh > 0 || !is_type_walkable(bt)) {
            return false;
        }
    }
    true
}

/// Find the nearest walkable tile adjacent to (gx, gy). Used when pathfinding to non-walkable targets (e.g. crates, walls).
pub fn adjacent_walkable(grid: &[u32], gx: i32, gy: i32) -> Option<(i32, i32)> {
    for &(dx, dy) in &[(0i32, -1i32), (0, 1), (-1, 0), (1, 0), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
        let nx = gx + dx;
        let ny = gy + dy;
        if is_walkable_pos(grid, nx as f32 + 0.5, ny as f32 + 0.5) {
            return Some((nx, ny));
        }
    }
    None
}

/// A* pathfinding on the block grid. Returns path from start to goal (inclusive), or empty if unreachable.
pub fn astar_path(grid: &[u32], start: (i32, i32), goal: (i32, i32)) -> Vec<(i32, i32)> {
    use std::collections::{BinaryHeap, HashMap};
    use std::cmp::Reverse;

    if start == goal { return vec![goal]; }

    let is_walk = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= GRID_W as i32 || y >= GRID_H as i32 { return false; }
        let b = grid[(y as u32 * GRID_W + x as u32) as usize];
        let bt = b & 0xFF;
        let bh = (b >> 8) & 0xFF;
        let is_door = (b >> 16) & 1 != 0;
        let bt32 = bt as u32;
        let is_any_pipe = (bt32 >= 15 && bt32 <= 20) || bt32 == BT_RESTRICTOR
            || bt32 == BT_LIQUID_PIPE || bt32 == BT_PIPE_BRIDGE
            || bt32 == BT_LIQUID_INTAKE || bt32 == BT_LIQUID_PUMP || bt32 == BT_LIQUID_OUTPUT;
        is_door || (bh == 0 && is_type_walkable(bt)) || (bt == BT_DUG_GROUND && bh <= 1) || (is_any_pipe && bh <= 1)
            || bt32 == BT_DIAGONAL // diagonal wall: partially walkable (continuous check handles collision)
    };

    if !is_walk(goal.0, goal.1) { return vec![]; }

    let heuristic = |a: (i32, i32)| -> i32 {
        (a.0 - goal.0).abs() + (a.1 - goal.1).abs()
    };

    let mut open = BinaryHeap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();

    g_score.insert(start, 0);
    open.push(Reverse((heuristic(start), start)));

    while let Some(Reverse((_, current))) = open.pop() {
        if current == goal {
            let mut path = vec![current];
            let mut node = current;
            while let Some(&prev) = came_from.get(&node) {
                path.push(prev);
                node = prev;
            }
            path.reverse();
            return path;
        }

        let g = *g_score.get(&current).unwrap_or(&i32::MAX);

        for &(ndx, ndy) in &[(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
            let next = (current.0 + ndx, current.1 + ndy);
            if !is_walk(next.0, next.1) { continue; }

            let ng = g + 1;
            if ng < *g_score.get(&next).unwrap_or(&i32::MAX) {
                g_score.insert(next, ng);
                came_from.insert(next, current);
                open.push(Reverse((ng + heuristic(next), next)));
            }
        }
    }

    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::make_block;

    /// Create a small test grid. All dirt floor (walkable) with optional walls.
    fn test_grid(walls: &[(u32, u32)]) -> Vec<u32> {
        let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
        for &(x, y) in walls {
            grid[(y * GRID_W + x) as usize] = make_block(1, 3, 0); // stone wall
        }
        grid
    }

    #[test]
    fn test_astar_same_start_goal() {
        let grid = test_grid(&[]);
        let path = astar_path(&grid, (5, 5), (5, 5));
        assert_eq!(path, vec![(5, 5)]);
    }

    #[test]
    fn test_astar_straight_line() {
        let grid = test_grid(&[]);
        let path = astar_path(&grid, (5, 5), (5, 8));
        assert!(!path.is_empty());
        assert_eq!(path.first(), Some(&(5, 5)));
        assert_eq!(path.last(), Some(&(5, 8)));
        // Should be 4 steps (including start and goal)
        assert_eq!(path.len(), 4);
    }

    #[test]
    fn test_astar_around_wall() {
        // Wall blocking direct path from (5,5) to (5,8)
        let grid = test_grid(&[(5, 6), (5, 7)]);
        let path = astar_path(&grid, (5, 5), (5, 8));
        assert!(!path.is_empty());
        assert_eq!(path.last(), Some(&(5, 8)));
        // Path should go around (longer than 4)
        assert!(path.len() > 4);
        // Path should not contain wall tiles
        for &(px, py) in &path {
            assert!(!(px == 5 && (py == 6 || py == 7)), "path goes through wall");
        }
    }

    #[test]
    fn test_astar_unreachable() {
        // Completely walled-off goal
        let grid = test_grid(&[(9, 9), (10, 9), (11, 9), (9, 10), (11, 10), (9, 11), (10, 11), (11, 11)]);
        let path = astar_path(&grid, (5, 5), (10, 10));
        assert!(path.is_empty(), "should be empty for unreachable goal");
    }

    #[test]
    fn test_astar_goal_is_wall() {
        let grid = test_grid(&[(10, 10)]);
        let path = astar_path(&grid, (5, 5), (10, 10));
        assert!(path.is_empty(), "should be empty when goal is a wall");
    }

    #[test]
    fn test_walkable_pos_open_ground() {
        let grid = test_grid(&[]);
        assert!(is_walkable_pos(&grid, 5.5, 5.5));
    }

    #[test]
    fn test_walkable_pos_wall() {
        let grid = test_grid(&[(5, 5)]);
        assert!(!is_walkable_pos(&grid, 5.5, 5.5));
    }

    #[test]
    fn test_walkable_pos_near_wall_edge() {
        let grid = test_grid(&[(5, 5)]);
        // Just outside the wall (pleb radius is 0.25)
        assert!(is_walkable_pos(&grid, 4.5, 5.5)); // left of wall
        // On the wall
        assert!(!is_walkable_pos(&grid, 5.2, 5.2));
    }

    #[test]
    fn test_walkable_pos_door() {
        let mut grid = test_grid(&[]);
        // Place a closed door (type 4, height 1, flag=door)
        grid[(5 * GRID_W + 5) as usize] = make_block(4, 1, 1);
        // Doors are walkable (plebs open them)
        assert!(is_walkable_pos(&grid, 5.5, 5.5));
    }

    #[test]
    fn test_appearance_deterministic() {
        let a1 = PlebAppearance::random(42);
        let a2 = PlebAppearance::random(42);
        assert_eq!(a1.skin_r, a2.skin_r);
        assert_eq!(a1.hair_r, a2.hair_r);
        assert_eq!(a1.shirt_r, a2.shirt_r);

        // Different seed = different appearance
        let a3 = PlebAppearance::random(99);
        assert_ne!(a1.skin_r, a3.skin_r);
    }
}

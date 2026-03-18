//! Pleb (colonist) — struct, movement, and A* pathfinding.

use crate::grid::{GRID_W, GRID_H};

pub struct Pleb {
    pub x: f32,
    pub y: f32,
    pub angle: f32,         // facing direction in radians
    pub path: Vec<(i32, i32)>,  // A* path waypoints
    pub path_idx: usize,
    pub torch_on: bool,     // fire torch (T to toggle)
    pub headlight_on: bool, // directional headlamp (G to toggle)
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
        if !is_door && (bh > 0 || (bt != 0 && bt != 2 && bt != 6 && bt != 7 && bt != 10 && bt != 13 && bt != 15 && bt != 16 && bt != 17 && bt != 18 && bt != 26 && bt != 27 && bt != 28)) {
            return false;
        }
    }
    true
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
        is_door || (bh == 0 && (bt == 0 || bt == 2 || bt == 6 || bt == 7 || bt == 10 || bt == 13))
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

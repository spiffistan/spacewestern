//! Pleb communication (shouts) and group movement (flocking).
//! Decoupled from core simulation — reads pleb state, writes small adjustments.
//! See docs/dn/DN-013-communication-flocking.md for design.

use crate::grid::*;
use crate::pleb::{BubbleKind, Pleb};

// --- Constants ---

const SHOUT_COOLDOWN: f32 = 3.0; // seconds between shouts per pleb
const GROUP_RADIUS: f32 = 8.0; // tiles — plebs within this form an implicit group
const COMBAT_PROXIMITY: f32 = 15.0; // tiles — enemies this close disable cohesion (disperse)
const APPROACH_PROXIMITY: f32 = 25.0; // tiles — enemies this close reduce speed

// --- Flock Spacing Presets ---

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlockSpacing {
    Tight,
    #[default]
    Normal,
    Spread,
}

impl FlockSpacing {
    pub fn min_spacing(self) -> f32 {
        match self {
            FlockSpacing::Tight => 1.0,
            FlockSpacing::Normal => 4.0,
            FlockSpacing::Spread => 8.0,
        }
    }

    pub fn max_spacing(self) -> f32 {
        match self {
            FlockSpacing::Tight => 4.0,
            FlockSpacing::Normal => 8.0,
            FlockSpacing::Spread => 16.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FlockSpacing::Tight => "Tight",
            FlockSpacing::Normal => "Normal",
            FlockSpacing::Spread => "Spread",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            FlockSpacing::Tight => FlockSpacing::Normal,
            FlockSpacing::Normal => FlockSpacing::Spread,
            FlockSpacing::Spread => FlockSpacing::Tight,
        }
    }
}

// --- Shout System ---

#[derive(Clone, Copy, Debug)]
pub enum ShoutKind {
    Alert,    // "Enemy spotted!"
    Retreat,  // "Fall back!"
    Help,     // "I'm hit!"
    Covering, // "Covering!"
    Clear,    // "All clear"
}

impl ShoutKind {
    pub fn range(self) -> f32 {
        match self {
            ShoutKind::Alert => 20.0,
            ShoutKind::Retreat => 15.0,
            ShoutKind::Help => 20.0,
            ShoutKind::Covering => 10.0,
            ShoutKind::Clear => 15.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ShoutKind::Alert => "Contact!",
            ShoutKind::Retreat => "Fall back!",
            ShoutKind::Help => "I'm hit!",
            ShoutKind::Covering => "Covering!",
            ShoutKind::Clear => "Clear!",
        }
    }

    pub fn sound_freq(self) -> f32 {
        match self {
            ShoutKind::Alert => 300.0,
            ShoutKind::Retreat => 250.0,
            ShoutKind::Help => 400.0,
            ShoutKind::Covering => 350.0,
            ShoutKind::Clear => 280.0,
        }
    }

    pub fn sound_db(self) -> f32 {
        match self {
            ShoutKind::Help => 70.0,
            _ => 60.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Shout {
    pub kind: ShoutKind,
    pub x: f32,
    pub y: f32,
    pub pleb_idx: usize,
    pub is_enemy: bool,
}

/// Scan all plebs for state transitions that trigger shouts.
pub fn collect_shouts(plebs: &mut [Pleb], dt: f32) -> Vec<Shout> {
    let mut shouts = Vec::new();

    for (i, pleb) in plebs.iter_mut().enumerate() {
        if pleb.is_dead {
            continue;
        }

        // Tick cooldown
        if pleb.last_shout_timer > 0.0 {
            pleb.last_shout_timer -= dt;
        }

        // Compute current health band: 0=healthy, 1=wounded(<50%), 2=critical(<35%)
        let health_band = if pleb.needs.health < 0.35 {
            2
        } else if pleb.needs.health < 0.5 {
            1
        } else {
            0
        };

        let can_shout = pleb.last_shout_timer <= 0.0;

        // Alert: first target acquisition
        if can_shout
            && pleb.aim_target.is_some()
            && pleb.aim_progress >= 0.0
            && pleb.aim_progress < 0.05
        {
            // Only on fresh engagement (aim_progress just started)
            shouts.push(Shout {
                kind: ShoutKind::Alert,
                x: pleb.x,
                y: pleb.y,
                pleb_idx: i,
                is_enemy: pleb.is_enemy,
            });
            pleb.last_shout_timer = SHOUT_COOLDOWN;
        }

        // Help: health dropped into wounded band
        if can_shout && health_band > pleb.prev_health_band && health_band >= 1 {
            shouts.push(Shout {
                kind: ShoutKind::Help,
                x: pleb.x,
                y: pleb.y,
                pleb_idx: i,
                is_enemy: pleb.is_enemy,
            });
            pleb.last_shout_timer = SHOUT_COOLDOWN;
        }

        // Retreat: critical health
        if can_shout && health_band == 2 && pleb.prev_health_band < 2 {
            shouts.push(Shout {
                kind: ShoutKind::Retreat,
                x: pleb.x,
                y: pleb.y,
                pleb_idx: i,
                is_enemy: pleb.is_enemy,
            });
            pleb.last_shout_timer = SHOUT_COOLDOWN;
        }

        pleb.prev_health_band = health_band;
    }

    shouts
}

/// Process shouts: each pleb within range reacts to shouts from their faction.
pub fn process_shouts(plebs: &mut [Pleb], shouts: &[Shout], grid: &[u32], wall_data: &[u16]) {
    if shouts.is_empty() {
        return;
    }

    // Show bubble on the shouting pleb
    for shout in shouts {
        if let Some(p) = plebs.get_mut(shout.pleb_idx) {
            p.set_bubble(BubbleKind::Text(shout.kind.label().into()), 1.5);
        }
    }

    // Process reactions (need to avoid double-mutable-borrow, use index-based)
    let pleb_count = plebs.len();
    for shout in shouts {
        for pi in 0..pleb_count {
            if pi == shout.pleb_idx {
                continue;
            }
            let p = &plebs[pi];
            if p.is_dead || p.is_enemy != shout.is_enemy {
                continue; // different faction
            }

            let dx = p.x - shout.x;
            let dy = p.y - shout.y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Wall muffling: halve effective range if edge-blocked
            let effective_range = if dist > 1.0 {
                let blocked = edge_blocked_wd(
                    grid,
                    wall_data,
                    shout.x.floor() as i32,
                    shout.y.floor() as i32,
                    p.x.floor() as i32,
                    p.y.floor() as i32,
                );
                if blocked {
                    shout.kind.range() * 0.5
                } else {
                    shout.kind.range()
                }
            } else {
                shout.kind.range()
            };

            if dist > effective_range {
                continue;
            }

            // --- Reactions ---
            let p = &mut plebs[pi];
            match shout.kind {
                ShoutKind::Alert => {
                    // Drafted with no target: face the alert direction
                    if p.drafted && p.aim_target.is_none() {
                        p.angle = (shout.y - p.y).atan2(shout.x - p.x);
                    }
                }
                ShoutKind::Help => {
                    // If close and drafted with no target, move toward caller
                    if p.drafted && p.aim_target.is_none() && dist < 10.0 {
                        p.angle = (shout.y - p.y).atan2(shout.x - p.x);
                    }
                }
                ShoutKind::Retreat | ShoutKind::Covering | ShoutKind::Clear => {
                    // Information only — no forced behavior change
                }
            }
        }
    }
}

// --- Flocking / Group Movement ---

/// Flocking adjustment for a single pleb: (dx, dy) velocity offset.
pub struct FlockAdjust {
    pub pleb_idx: usize,
    pub dx: f32,
    pub dy: f32,
    pub speed_mul: f32, // 1.0 = no change, < 1.0 = slow down
}

/// Check if two plebs are in the same flock (same group_id, or both drafted same-faction ungrouped).
fn in_same_flock(a: &Pleb, b: &Pleb) -> bool {
    if a.is_enemy != b.is_enemy {
        return false;
    }
    // Explicit group: must match
    if let (Some(ga), Some(gb)) = (a.group_id, b.group_id) {
        return ga == gb;
    }
    // Implicit group: both drafted, same faction, no explicit group
    a.group_id.is_none() && b.group_id.is_none() && a.drafted && b.drafted
}

/// Compute flocking adjustments for all grouped/drafted plebs (both factions).
pub fn compute_flocking(
    plebs: &[Pleb],
    opponent_positions: &[(f32, f32)],
    spacing: FlockSpacing,
) -> Vec<FlockAdjust> {
    let min_spacing = spacing.min_spacing();
    let max_spacing = spacing.max_spacing();
    let mut adjustments = Vec::new();

    let nearest_opponent = |px: f32, py: f32| -> f32 {
        opponent_positions
            .iter()
            .map(|&(ex, ey)| ((px - ex).powi(2) + (py - ey).powi(2)).sqrt())
            .fold(f32::MAX, f32::min)
    };

    // Pre-compute distance-to-goal for speed matching
    let dist_to_goal: Vec<f32> = plebs
        .iter()
        .map(|p| {
            if p.path_idx < p.path.len() {
                let last = p.path[p.path.len() - 1];
                let dx = last.0 as f32 + 0.5 - p.x;
                let dy = last.1 as f32 + 0.5 - p.y;
                (dx * dx + dy * dy).sqrt()
            } else {
                0.0
            }
        })
        .collect();

    for (i, pleb) in plebs.iter().enumerate() {
        if pleb.is_dead {
            continue;
        }
        // Must be in a group (explicit or implicit via drafted)
        if pleb.group_id.is_none() && !pleb.drafted {
            continue;
        }

        let enemy_dist = nearest_opponent(pleb.x, pleb.y);
        let in_combat = enemy_dist < COMBAT_PROXIMITY;
        let approaching = enemy_dist < APPROACH_PROXIMITY && !in_combat;

        let mut adj_dx = 0.0f32;
        let mut adj_dy = 0.0f32;
        let mut speed_mul = 1.0f32;

        if approaching {
            speed_mul *= 0.85;
        }

        // Speed matching: compute average dist-to-goal among flock-mates that are moving
        let is_moving = pleb.path_idx < pleb.path.len();
        if is_moving {
            let mut flock_sum = dist_to_goal[i];
            let mut flock_count = 1u32;
            for (j, other) in plebs.iter().enumerate() {
                if j == i || other.is_dead || !in_same_flock(pleb, other) {
                    continue;
                }
                if other.path_idx < other.path.len() {
                    flock_sum += dist_to_goal[j];
                    flock_count += 1;
                }
            }
            if flock_count > 1 {
                let avg_dist = flock_sum / flock_count as f32;
                let my_dist = dist_to_goal[i];
                if avg_dist > 1.0 {
                    // Closer than average → slow down; further → speed up slightly
                    let ratio = (my_dist / avg_dist).clamp(0.7, 1.15);
                    speed_mul *= ratio;
                }
            }
        }

        for (j, other) in plebs.iter().enumerate() {
            if j == i || other.is_dead || !in_same_flock(pleb, other) {
                continue;
            }
            let dx = pleb.x - other.x;
            let dy = pleb.y - other.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 0.1 || dist > GROUP_RADIUS {
                continue;
            }
            let nx = dx / dist;
            let ny = dy / dist;

            if dist < min_spacing {
                let force = (min_spacing - dist) / min_spacing;
                adj_dx += nx * force * 2.0;
                adj_dy += ny * force * 2.0;
            }
            if !in_combat && dist > max_spacing {
                let force = ((dist - max_spacing) / max_spacing).min(0.2) * 0.15;
                adj_dx -= nx * force;
                adj_dy -= ny * force;
            }
        }

        if adj_dx.abs() > 0.001 || adj_dy.abs() > 0.001 || speed_mul < 0.99 {
            adjustments.push(FlockAdjust {
                pleb_idx: i,
                dx: adj_dx,
                dy: adj_dy,
                speed_mul,
            });
        }
    }

    adjustments
}

// --- Group Management ---

/// Assign a pleb to a group. Pass `None` to remove from group.
pub fn set_group(pleb: &mut Pleb, group_id: Option<u8>) {
    pleb.group_id = group_id;
}

/// Find the next unused group ID across all plebs.
pub fn next_group_id(plebs: &[Pleb]) -> u8 {
    let mut used = [false; 256];
    for p in plebs {
        if let Some(gid) = p.group_id {
            used[gid as usize] = true;
        }
    }
    for (i, &u) in used.iter().enumerate() {
        if !u && i > 0 {
            return i as u8;
        }
    }
    1 // fallback
}

/// Assign all plebs in `indices` to a new group. Returns the group ID used.
pub fn form_group(plebs: &mut [Pleb], indices: &[usize]) -> u8 {
    let gid = next_group_id(plebs);
    for &i in indices {
        if let Some(p) = plebs.get_mut(i) {
            p.group_id = Some(gid);
        }
    }
    gid
}

/// Remove a pleb from its group.
pub fn leave_group(pleb: &mut Pleb) {
    pleb.group_id = None;
}

/// Dissolve a group: all plebs with this group_id become ungrouped.
pub fn dissolve_group(plebs: &mut [Pleb], group_id: u8) {
    for p in plebs.iter_mut() {
        if p.group_id == Some(group_id) {
            p.group_id = None;
        }
    }
}

// --- Destination spreading for group moves ---

/// Compute offset positions for N plebs moving to the same destination.
/// Returns offsets from center; first pleb goes to center, rest spread in a ring.
pub fn spread_offsets(count: usize, spacing: f32) -> Vec<(f32, f32)> {
    if count <= 1 {
        return vec![(0.0, 0.0)];
    }
    let mut offsets = Vec::with_capacity(count);
    offsets.push((0.0, 0.0));
    let remaining = count - 1;
    // For 2-6 extra plebs: single ring. For 7+: fill inner ring first, then outer.
    let mut placed = 0;
    let mut ring = 1u32;
    while placed < remaining {
        let radius = spacing * ring as f32;
        let slots = (6 * ring as usize).max(remaining - placed);
        let n = (remaining - placed).min(slots);
        for i in 0..n {
            let angle = std::f32::consts::TAU * i as f32 / n as f32;
            offsets.push((radius * angle.cos(), radius * angle.sin()));
            placed += 1;
        }
        ring += 1;
    }
    offsets
}

// --- Debug: flock link data for visualization ---

/// A link between two plebs for debug rendering.
pub struct FlockLink {
    pub ax: f32,
    pub ay: f32,
    pub bx: f32,
    pub by: f32,
    pub force: FlockForce,
    pub strength: f32, // 0.0–1.0 normalized force magnitude
}

#[derive(Clone, Copy)]
pub enum FlockForce {
    Separation, // too close — red
    Cohesion,   // too far — blue
    Group,      // in group range, neutral — gray
}

/// Compute flock links for debug overlay. Set `show_enemies` to include enemy groups.
pub fn compute_flock_links(
    plebs: &[Pleb],
    opponent_positions: &[(f32, f32)],
    show_enemies: bool,
    spacing: FlockSpacing,
) -> Vec<FlockLink> {
    let min_spacing = spacing.min_spacing();
    let max_spacing = spacing.max_spacing();
    let mut links = Vec::new();

    let nearest_opponent = |px: f32, py: f32| -> f32 {
        opponent_positions
            .iter()
            .map(|&(ex, ey)| ((px - ex).powi(2) + (py - ey).powi(2)).sqrt())
            .fold(f32::MAX, f32::min)
    };

    for (i, a) in plebs.iter().enumerate() {
        if a.is_dead {
            continue;
        }
        if !show_enemies && a.is_enemy {
            continue;
        }
        if a.group_id.is_none() && !a.drafted {
            continue;
        }
        let in_combat = nearest_opponent(a.x, a.y) < COMBAT_PROXIMITY;

        for (j, b) in plebs.iter().enumerate() {
            if j <= i || b.is_dead || !in_same_flock(a, b) {
                continue;
            }
            let dx = a.x - b.x;
            let dy = a.y - b.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > GROUP_RADIUS || dist < 0.1 {
                continue;
            }

            let (force, strength) = if dist < min_spacing {
                (FlockForce::Separation, (min_spacing - dist) / min_spacing)
            } else if !in_combat && dist > max_spacing {
                (
                    FlockForce::Cohesion,
                    ((dist - max_spacing) / (GROUP_RADIUS - max_spacing)).min(1.0),
                )
            } else {
                (FlockForce::Group, 1.0 - (dist / GROUP_RADIUS))
            };

            links.push(FlockLink {
                ax: a.x,
                ay: a.y,
                bx: b.x,
                by: b.y,
                force,
                strength,
            });
        }
    }

    links
}

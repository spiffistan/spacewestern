//! Fire system — burning, spreading, and consumption of flammable blocks.
//!
//! Fire state is temperature-driven: a block burns when block_temp > ignition_temp.
//! The CPU tracks burn_progress per block; the GPU thermal system handles heat conduction.

use crate::block_defs::BlockRegistry;
use crate::grid::*;
use std::collections::HashMap;

// --- Fire constants ---
const BURN_TEMP_BASE: f32 = 350.0; // burning blocks maintain at least this temp
const BURN_TEMP_BOOST: f32 = 150.0; // additional temp based on material
const SPREAD_CHECK_INTERVAL: u32 = 10; // check fire spread every N frames
const SPREAD_TEMP_INJECT: f32 = 50.0; // heat injected into neighbor per spread tick
const RAIN_PROGRESS_REDUCE: f32 = 0.05; // burn progress reduction per sec in rain
const RAIN_THRESHOLD: f32 = 0.3; // rain intensity needed to extinguish
const WET_DAMPING: f32 = 0.8; // how much wetness slows burn (0=none, 1=full)
const WET_MIN_FACTOR: f32 = 0.1; // minimum burn rate even when very wet
const WET_BLOCK_THRESHOLD: f32 = 0.8; // wetness above which blocks won't ignite
const BASE_IGNITE_CHANCE: f32 = 0.03; // per-tick chance of neighbor catching fire
const WET_IGNITE_DAMPING: f32 = 0.7; // how much wetness reduces ignite chance

/// Burn time in seconds for a block type. Returns None if not flammable.
fn burn_time(bt: u32) -> Option<f32> {
    match bt {
        BT_TREE => Some(30.0),
        BT_DIRT => Some(5.0), // grass burns fast
        BT_BENCH => Some(15.0),
        BT_WOOD_WALL => Some(25.0),
        BT_WOOD_FLOOR => Some(20.0),
        BT_ROUGH_FLOOR => Some(15.0), // rough planks burn faster
        BT_BED => Some(12.0),
        BT_BERRY_BUSH => Some(10.0),
        BT_CRATE => Some(15.0),
        BT_CROP => Some(8.0),
        _ => None,
    }
}

/// What block type replaces a burned block.
pub fn burn_replacement_pub(bt: u32) -> u32 {
    burn_replacement(bt)
}
fn burn_replacement(bt: u32) -> u32 {
    match bt {
        BT_WOOD_WALL => BT_AIR,                       // wall collapses
        BT_BENCH | BT_BED | BT_CRATE => BT_AIR,       // furniture gone
        BT_TREE | BT_BERRY_BUSH | BT_CROP => BT_DIRT, // charred ground
        BT_WOOD_FLOOR | BT_ROUGH_FLOOR => BT_DIRT,    // exposed dirt beneath
        _ => BT_DIRT,
    }
}

/// Per-frame fire tick. Returns a Vec of (grid_idx, temperature) overrides
/// to write to the GPU block_temps buffer, and a Vec of (grid_idx) for
/// blocks that should be destroyed this frame.
pub fn tick_fire(
    grid: &[u32],
    burn_progress: &mut HashMap<usize, f32>,
    dt: f32,
    time_speed: f32,
    frame_count: u32,
    rain_intensity: f32,
    wind_angle: f32,
    _wind_magnitude: f32,
    wetness: &[f32],
    fire_intensity: f32,
) -> (Vec<(usize, f32)>, Vec<usize>) {
    let mut temp_overrides = Vec::new();
    let mut destroyed = Vec::new();
    let grid_size = (GRID_W * GRID_H) as usize;
    let reg = BlockRegistry::cached();
    let t = dt * time_speed;

    // --- Burn progress + self-heating ---
    let burning_indices: Vec<usize> = burn_progress.keys().copied().collect();
    for idx in &burning_indices {
        let idx = *idx;
        if idx >= grid_size {
            continue;
        }
        let block = grid[idx];
        let bt = block_type_rs(block);

        // Scorched dirt (flags bit 0) — already burned, remove
        let bf = block_flags_rs(block);
        if bt == BT_DIRT && (bf & 1) != 0 {
            burn_progress.remove(&idx);
            continue;
        }

        let bt_time = match burn_time(bt) {
            Some(t) => t,
            None => {
                burn_progress.remove(&idx);
                continue;
            }
        };

        let progress = match burn_progress.get_mut(&idx) {
            Some(p) => p,
            None => continue, // entry removed by another pass
        };

        // Advance burn progress
        let wet = if idx < wetness.len() {
            wetness[idx]
        } else {
            0.0
        };
        let wet_factor = (1.0 - wet * WET_DAMPING).max(WET_MIN_FACTOR);
        *progress += t / bt_time * wet_factor;

        // Rain extinguishing (outdoor blocks only)
        let roof_h = roof_height_rs(block);
        if rain_intensity > RAIN_THRESHOLD && roof_h == 0 {
            *progress -= RAIN_PROGRESS_REDUCE * rain_intensity * t;
        }

        if *progress >= 1.0 {
            destroyed.push(idx);
        } else if *progress <= 0.0 {
            // Extinguished by rain
            burn_progress.remove(&idx);
            continue;
        }

        // Self-heating: maintain high temperature (scaled by fire_intensity)
        let burn_temp = (BURN_TEMP_BASE + BURN_TEMP_BOOST * (*progress).min(1.0)) * fire_intensity;
        temp_overrides.push((idx, burn_temp));
    }

    // Remove destroyed blocks from burn_progress
    for &idx in &destroyed {
        burn_progress.remove(&idx);
    }

    // --- Fire spread (every N frames) ---
    if frame_count % SPREAD_CHECK_INTERVAL == 0 && !burning_indices.is_empty() {
        let wind_dx = wind_angle.cos();
        let wind_dy = wind_angle.sin();

        for &idx in &burning_indices {
            if idx >= grid_size {
                continue;
            }
            let bx = (idx % GRID_W as usize) as i32;
            let by = (idx / GRID_W as usize) as i32;

            // Check 4 cardinal neighbors
            for &(dx, dy) in &[(0i32, -1i32), (0, 1), (1, 0), (-1, 0)] {
                let nx = bx + dx;
                let ny = by + dy;
                if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
                    continue;
                }
                let nidx = (ny as u32 * GRID_W + nx as u32) as usize;

                // Skip already burning
                if burn_progress.contains_key(&nidx) {
                    continue;
                }

                let nb = grid[nidx];
                let nbt = block_type_rs(nb);
                let ndef = match reg.get(nbt) {
                    Some(d) => d,
                    None => continue,
                };
                if !ndef.is_flammable {
                    continue;
                }

                // Scorched dirt (flags bit 0) — grass already burned, skip
                let nbf = block_flags_rs(nb);
                if nbt == BT_DIRT && (nbf & 1) != 0 {
                    continue;
                }

                // Wind bonus: spreading downwind is easier
                let wind_dot = dx as f32 * wind_dx + dy as f32 * wind_dy;
                let wind_bonus = if wind_dot > 0.0 { 1.0 + wind_dot } else { 1.0 };

                // Wetness resistance
                let wet = if nidx < wetness.len() {
                    wetness[nidx]
                } else {
                    0.0
                };
                if wet > WET_BLOCK_THRESHOLD {
                    continue;
                }

                // Inject heat into neighbor (thermal system will propagate)
                let heat_inject = SPREAD_TEMP_INJECT * wind_bonus * (1.0 - wet * 0.5); // wet ground absorbs less heat
                temp_overrides.push((nidx, heat_inject));

                // Check if neighbor is hot enough to ignite
                // (We don't have GPU temps on CPU — use the injected heat as proxy.
                //  After several spread ticks of heat injection, the thermal system
                //  will have raised the neighbor above ignition_temp. We check
                //  approximate accumulated heat here.)
                // For now: ignite if we've been injecting heat for a while
                // This is approximate — true ignition happens when the block
                // actually reaches ignition_temp on GPU. As a shortcut, ignite
                // neighbors probabilistically based on proximity and wind.
                let ignite_chance =
                    BASE_IGNITE_CHANCE * wind_bonus * (1.0 - wet * WET_IGNITE_DAMPING);
                let hash = (nidx as u32)
                    .wrapping_mul(2654435761)
                    .wrapping_add(frame_count * 1013904223);
                let roll = (hash & 0xFFFF) as f32 / 65535.0;
                if roll < ignite_chance {
                    burn_progress.insert(nidx, 0.0);
                }
            }
        }
    }

    (temp_overrides, destroyed)
}

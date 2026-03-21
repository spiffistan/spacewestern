//! Zone system — designated areas for farming, storage, etc.
//! Zones are overlays on the grid, not block types.

use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum ZoneKind {
    Growing,
    Storage,
}

#[derive(Clone, Debug)]
pub struct Zone {
    pub kind: ZoneKind,
    pub tiles: HashSet<(i32, i32)>,
}

impl Zone {
    pub fn new(kind: ZoneKind) -> Self {
        Zone { kind, tiles: HashSet::new() }
    }
}

/// Global work task that a pleb can claim.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkTask {
    Plant(i32, i32),    // plant a crop at this position
    Harvest(i32, i32),  // harvest any plant (crop, berry bush, etc.)
}

/// Work priority ordering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorkPriority {
    PlantFirst,
    HarvestFirst,
}

impl WorkTask {
    pub fn position(&self) -> (i32, i32) {
        match self {
            WorkTask::Plant(x, y) | WorkTask::Harvest(x, y) => (*x, *y),
        }
    }
}

/// Crop growth stages (stored in block height byte for type BT_CROP).
pub const CROP_PLANTED: u32 = 0;
pub const CROP_SPROUT: u32 = 1;
pub const CROP_GROWING: u32 = 2;
pub const CROP_MATURE: u32 = 3;

/// Game seconds for a crop to advance one growth stage under ideal conditions.
/// With environmental factors < 1.0, effective time is much longer.
/// 60s × 4 stages = 240 game seconds (~4 game-days at 50% speed).
pub const CROP_GROW_TIME: f32 = 60.0;

/// Optimal temperature range for crop growth (°C). Bell curve peaks here.
pub const CROP_OPTIMAL_LOW: f32 = 15.0;
pub const CROP_OPTIMAL_HIGH: f32 = 28.0;
/// Absolute limits — zero growth outside these.
pub const CROP_TEMP_MIN: f32 = 5.0;
pub const CROP_TEMP_MAX: f32 = 40.0;

/// Crop growth status for UI display.
pub struct CropStatus {
    pub stage_name: &'static str,
    pub stage: u32,
    pub progress: f32,        // 0-1 within current stage
    pub growth_rate: f32,     // 0-1 combined growth multiplier
    pub temp_factor: f32,
    pub sun_factor: f32,
    pub water_factor: f32,
    pub limiting: &'static str, // which factor is worst
}

/// Compute crop growth status for a tile. Returns None if not a crop.
pub fn crop_status(
    block: u32, grid_idx: u32, timer: f32,
    time_of_day: f32, sun_intensity: f32, rain_intensity: f32,
    water_table: f32, surface_water: f32,
) -> Option<CropStatus> {
    let bt = block & 0xFF;
    if bt != 47 { return None; } // BT_CROP
    let stage = (block >> 8) & 0xFF;
    let stage_name = match stage {
        0 => "Planted",
        1 => "Sprout",
        2 => "Growing",
        3 => "Mature",
        _ => "Unknown",
    };
    if stage >= CROP_MATURE {
        return Some(CropStatus {
            stage_name, stage, progress: 1.0, growth_rate: 0.0,
            temp_factor: 0.0, sun_factor: 0.0, water_factor: 0.0, limiting: "Ready to harvest",
        });
    }

    let progress = (timer / CROP_GROW_TIME).clamp(0.0, 1.0);

    let day_frac = time_of_day / 60.0;
    let sun_t = ((day_frac - 0.15) / 0.7).clamp(0.0, 1.0);
    let sun_curve = (sun_t * std::f32::consts::PI).sin();
    let approx_temp = 5.0 + 20.0 * sun_curve;

    let temp_factor = if approx_temp < CROP_TEMP_MIN || approx_temp > CROP_TEMP_MAX {
        0.0
    } else if approx_temp >= CROP_OPTIMAL_LOW && approx_temp <= CROP_OPTIMAL_HIGH {
        1.0
    } else if approx_temp < CROP_OPTIMAL_LOW {
        (approx_temp - CROP_TEMP_MIN) / (CROP_OPTIMAL_LOW - CROP_TEMP_MIN)
    } else {
        (CROP_TEMP_MAX - approx_temp) / (CROP_TEMP_MAX - CROP_OPTIMAL_HIGH)
    };

    let sun_factor = (sun_intensity * 1.2).clamp(0.0, 1.0);

    let wt_moisture = ((water_table + 2.0) / 2.5).clamp(0.0, 1.0);
    let rain_moisture = (rain_intensity * 0.5).min(0.3);
    let surface_moisture = (surface_water * 2.0).clamp(0.0, 0.8); // surface water is very effective
    let water_avail = (wt_moisture + rain_moisture + surface_moisture).clamp(0.0, 1.0);
    let water_factor = if water_avail < 0.1 {
        water_avail * 2.0
    } else if water_avail < 0.7 {
        0.2 + water_avail * 1.14
    } else {
        1.0 - (water_avail - 0.7) * 0.3
    };

    let hash = (grid_idx.wrapping_mul(2654435761).wrapping_add(stage * 1013904223)) & 0xFFFF;
    let random_factor = 0.7 + (hash as f32 / 65535.0) * 0.6;

    let growth_rate = temp_factor * sun_factor * water_factor * random_factor;

    let limiting = if stage >= CROP_MATURE { "Mature" }
        else if temp_factor < 0.01 { "Too cold/hot" }
        else if sun_factor < 0.01 { "No sunlight" }
        else if water_factor < 0.1 { "Needs water" }
        else if temp_factor < sun_factor && temp_factor < water_factor { "Temperature" }
        else if sun_factor < water_factor { "Sunlight" }
        else { "Water" };

    Some(CropStatus {
        stage_name, stage, progress, growth_rate, temp_factor, sun_factor, water_factor, limiting,
    })
}

/// Generate work tasks from zones + grid state.
/// Returns tasks that are not yet being worked on.
pub fn generate_work_tasks(
    zones: &[Zone],
    grid: &[u32],
    grid_w: u32,
    active_tasks: &HashSet<(i32, i32)>, // positions already claimed by a pleb
) -> Vec<WorkTask> {
    let mut tasks = Vec::new();

    for zone in zones {
        match zone.kind {
            ZoneKind::Growing => {
                for &(x, y) in &zone.tiles {
                    if x < 0 || y < 0 || x >= grid_w as i32 || y >= (grid.len() as u32 / grid_w) as i32 {
                        continue;
                    }
                    if active_tasks.contains(&(x, y)) { continue; }

                    let idx = (y as u32 * grid_w + x as u32) as usize;
                    let block = grid[idx];
                    let bt = block & 0xFF;
                    let bh = (block >> 8) & 0xFF;

                    if bt == 2 && bh == 0 {
                        // Empty dirt in growing zone → needs planting
                        tasks.push(WorkTask::Plant(x, y));
                    } else if bt == 47 && bh >= CROP_MATURE {
                        // Mature crop → needs harvesting
                        tasks.push(WorkTask::Harvest(x, y));
                    }
                }
            }
            ZoneKind::Storage => {
                // Storage zones don't generate work tasks directly.
                // Items are hauled there via context menu or auto-haul.
            }
        }
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::make_block;

    #[test]
    fn test_growing_zone_generates_plant_tasks() {
        let mut zone = Zone::new(ZoneKind::Growing);
        zone.tiles.insert((5, 5));
        zone.tiles.insert((6, 5));

        let grid_w = 256u32;
        let mut grid = vec![make_block(2, 0, 0); (grid_w * grid_w) as usize]; // all dirt

        let tasks = generate_work_tasks(&[zone], &grid, grid_w, &HashSet::new());
        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().any(|t| *t == WorkTask::Plant(5, 5)));
        assert!(tasks.iter().any(|t| *t == WorkTask::Plant(6, 5)));
    }

    #[test]
    fn test_mature_crop_generates_harvest_task() {
        let mut zone = Zone::new(ZoneKind::Growing);
        zone.tiles.insert((5, 5));

        let grid_w = 256u32;
        let mut grid = vec![make_block(2, 0, 0); (grid_w * grid_w) as usize];
        // Place mature crop
        let idx = (5 * grid_w + 5) as usize;
        grid[idx] = make_block(47, CROP_MATURE as u8, 0);

        let tasks = generate_work_tasks(&[zone], &grid, grid_w, &HashSet::new());
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0], WorkTask::Harvest(5, 5));
    }

    #[test]
    fn test_active_tasks_excluded() {
        let mut zone = Zone::new(ZoneKind::Growing);
        zone.tiles.insert((5, 5));

        let grid_w = 256u32;
        let grid = vec![make_block(2, 0, 0); (grid_w * grid_w) as usize];

        let mut active = HashSet::new();
        active.insert((5, 5));

        let tasks = generate_work_tasks(&[zone], &grid, grid_w, &active);
        assert!(tasks.is_empty());
    }
}

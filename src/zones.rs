//! Zone system — designated areas for farming, storage, etc.
//! Zones are overlays on the grid, not block types.

use crate::grid::*;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum ZoneKind {
    Growing,
    Storage,
    Dig,
    Berm,
}

/// A dig zone with target depth and cross-section profile.
#[derive(Clone, Debug)]
pub struct DigZone {
    pub tiles: HashSet<(i32, i32)>,
    pub target_depth: f32,
    pub profile: crate::terrain::CrossProfile,
    /// Width of the zone in tiles (for profile depth calculation). 0 = auto-detect.
    pub width: f32,
    /// Original elevation at each tile center when the zone was created.
    pub base_elevations: std::collections::HashMap<(i32, i32), f32>,
}

/// A berm zone: terrain is raised by dumping dirt here.
#[derive(Clone, Debug)]
pub struct BermZone {
    pub tiles: HashSet<(i32, i32)>,
    pub target_height: f32, // how far above current surface to raise
}

#[derive(Clone, Debug)]
pub struct Zone {
    pub kind: ZoneKind,
    pub tiles: HashSet<(i32, i32)>,
}

impl Zone {
    pub fn new(kind: ZoneKind) -> Self {
        Zone {
            kind,
            tiles: HashSet::new(),
        }
    }
}

/// Global work task that a pleb can claim.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkTask {
    Plant(i32, i32),   // plant a crop at this position
    Harvest(i32, i32), // harvest any plant (crop, berry bush, etc.)
    Dig(i32, i32),     // dig terrain at this position
    Fill(i32, i32),    // dump dirt to raise terrain at this position
}

/// Work priority ordering (legacy, used for plant/harvest preference).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorkPriority {
    PlantFirst,
    HarvestFirst,
}

/// Work type categories for the priority system.
pub const WORK_FARM: usize = 0;
#[allow(dead_code)]
pub const WORK_BUILD: usize = 1;
pub const WORK_CRAFT: usize = 2;
pub const WORK_HAUL: usize = 3;
pub const WORK_TYPE_COUNT: usize = 4;

pub const WORK_TYPE_NAMES: [&str; WORK_TYPE_COUNT] = ["Farm", "Build", "Craft", "Haul"];

/// Per-pleb work priorities. 0 = disabled, 1-3 = priority (1 = highest).
/// Default: all enabled at priority 3.
pub fn default_work_priorities() -> [u8; WORK_TYPE_COUNT] {
    [3, 3, 3, 3]
}

impl WorkTask {
    pub fn position(&self) -> (i32, i32) {
        match self {
            WorkTask::Plant(x, y)
            | WorkTask::Harvest(x, y)
            | WorkTask::Dig(x, y)
            | WorkTask::Fill(x, y) => (*x, *y),
        }
    }
}

/// Crop growth stages (stored in block height byte for type BT_CROP).
pub const CROP_PLANTED: u32 = 0;
#[allow(dead_code)]
pub const CROP_SPROUT: u32 = 1;
#[allow(dead_code)]
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
    pub progress: f32,    // 0-1 within current stage
    pub growth_rate: f32, // 0-1 combined growth multiplier
    pub temp_factor: f32,
    pub sun_factor: f32,
    pub water_factor: f32,
    pub limiting: &'static str, // which factor is worst
}

/// Compute crop growth status for a tile. Returns None if not a crop.
pub fn crop_status(
    block: u32,
    grid_idx: u32,
    timer: f32,
    time_of_day: f32,
    sun_intensity: f32,
    rain_intensity: f32,
    water_table: f32,
    surface_water: f32,
) -> Option<CropStatus> {
    let bt = block_type_rs(block);
    if bt != BT_CROP {
        return None;
    }
    let stage = block_height_rs(block) as u32;
    let stage_name = match stage {
        0 => "Planted",
        1 => "Sprout",
        2 => "Growing",
        3 => "Mature",
        _ => "Unknown",
    };
    if stage >= CROP_MATURE {
        return Some(CropStatus {
            stage_name,
            stage,
            progress: 1.0,
            growth_rate: 0.0,
            temp_factor: 0.0,
            sun_factor: 0.0,
            water_factor: 0.0,
            limiting: "Ready to harvest",
        });
    }

    let progress = (timer / CROP_GROW_TIME).clamp(0.0, 1.0);

    const DAY_DURATION: f32 = 60.0;
    const DAWN_FRAC: f32 = 0.15;
    const DAY_LENGTH_FRAC: f32 = 0.7;
    const TEMP_MIN: f32 = 5.0;
    const TEMP_RANGE: f32 = 20.0;
    let day_frac = time_of_day / DAY_DURATION;
    let sun_t = ((day_frac - DAWN_FRAC) / DAY_LENGTH_FRAC).clamp(0.0, 1.0);
    let sun_curve = (sun_t * std::f32::consts::PI).sin();
    let approx_temp = TEMP_MIN + TEMP_RANGE * sun_curve;

    let temp_factor = if !(CROP_TEMP_MIN..=CROP_TEMP_MAX).contains(&approx_temp) {
        0.0
    } else if (CROP_OPTIMAL_LOW..=CROP_OPTIMAL_HIGH).contains(&approx_temp) {
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

    let hash = (grid_idx
        .wrapping_mul(2654435761)
        .wrapping_add(stage * 1013904223))
        & 0xFFFF;
    let random_factor = 0.7 + (hash as f32 / 65535.0) * 0.6;

    let growth_rate = temp_factor * sun_factor * water_factor * random_factor;

    let limiting = if stage >= CROP_MATURE {
        "Mature"
    } else if temp_factor < 0.01 {
        "Too cold/hot"
    } else if sun_factor < 0.01 {
        "No sunlight"
    } else if water_factor < 0.1 {
        "Needs water"
    } else if temp_factor < sun_factor && temp_factor < water_factor {
        "Temperature"
    } else if sun_factor < water_factor {
        "Sunlight"
    } else {
        "Water"
    };

    Some(CropStatus {
        stage_name,
        stage,
        progress,
        growth_rate,
        temp_factor,
        sun_factor,
        water_factor,
        limiting,
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
                    if x < 0
                        || y < 0
                        || x >= grid_w as i32
                        || y >= (grid.len() as u32 / grid_w) as i32
                    {
                        continue;
                    }
                    if active_tasks.contains(&(x, y)) {
                        continue;
                    }

                    let idx = (y as u32 * grid_w + x as u32) as usize;
                    let block = grid[idx];
                    let bt = block_type_rs(block);
                    let bh = block_height_rs(block) as u32;

                    if bt == BT_GROUND && bh == 0 {
                        // Empty dirt in growing zone → needs planting
                        tasks.push(WorkTask::Plant(x, y));
                    } else if bt == BT_CROP && bh >= CROP_MATURE {
                        // Mature crop → needs harvesting
                        tasks.push(WorkTask::Harvest(x, y));
                    }
                }
            }
            ZoneKind::Storage | ZoneKind::Dig | ZoneKind::Berm => {
                // Storage: hauled via context menu.
                // Dig/Berm: handled separately via generate_dig/fill_tasks.
            }
        }
    }

    tasks
}

/// Generate dig tasks from dig zones. Checks sub-tile elevation to see if work remains.
pub fn generate_dig_tasks(
    dig_zones: &[DigZone],
    sub_elevation: &[f32],
    active_tasks: &HashSet<(i32, i32)>,
) -> Vec<WorkTask> {
    let mut tasks = Vec::new();

    for dz in dig_zones {
        for &(x, y) in &dz.tiles {
            if active_tasks.contains(&(x, y)) {
                continue;
            }
            // Check if any sub-cell under this tile still needs digging
            let sx_base = (x as u32) * crate::terrain::ELEV_SCALE;
            let sy_base = (y as u32) * crate::terrain::ELEV_SCALE;
            let mut needs_dig = false;
            for dy in 0..crate::terrain::ELEV_SCALE {
                for dx in 0..crate::terrain::ELEV_SCALE {
                    let sx = sx_base + dx;
                    let sy = sy_base + dy;
                    if sx >= crate::terrain::ELEV_W || sy >= crate::terrain::ELEV_H {
                        continue;
                    }
                    let idx = (sy * crate::terrain::ELEV_W + sx) as usize;
                    if idx < sub_elevation.len() {
                        let current = sub_elevation[idx];
                        // Target = original elevation minus target depth
                        let base = dz.base_elevations.get(&(x, y)).copied().unwrap_or(current);
                        let target = base - dz.target_depth;
                        if current > target + 0.02 {
                            needs_dig = true;
                            break;
                        }
                    }
                }
                if needs_dig {
                    break;
                }
            }
            if needs_dig {
                tasks.push(WorkTask::Dig(x, y));
            }
        }
    }

    tasks
}

/// Generate fill/berm tasks. Checks if tile elevation is below target height.
pub fn generate_fill_tasks(
    berm_zones: &[BermZone],
    sub_elevation: &[f32],
    active_tasks: &HashSet<(i32, i32)>,
) -> Vec<WorkTask> {
    let mut tasks = Vec::new();

    for bz in berm_zones {
        let target_h = bz.target_height;

        for &(x, y) in &bz.tiles {
            if active_tasks.contains(&(x, y)) {
                continue;
            }
            let cur =
                crate::terrain::sample_elevation(sub_elevation, x as f32 + 0.5, y as f32 + 0.5);
            if cur < target_h - 0.02 {
                tasks.push(WorkTask::Fill(x, y));
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

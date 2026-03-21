//! Zone system — designated areas for farming, storage, etc.
//! Zones are overlays on the grid, not block types.

use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum ZoneKind {
    Growing,
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

/// Game seconds for a crop to advance one growth stage at comfortable temperature.
pub const CROP_GROW_TIME: f32 = 20.0;

/// Temperature range for optimal crop growth (°C).
pub const CROP_MIN_TEMP: f32 = 10.0;
pub const CROP_MAX_TEMP: f32 = 35.0;

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

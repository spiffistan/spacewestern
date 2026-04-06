//! Room detection — flood-fill enclosed spaces from walls, doors, roofs.
//! Rooms have properties (size, amenities) that affect pleb mood.

use crate::grid::*;

/// A detected room — an enclosed space bounded by walls and doors.
#[derive(Clone, Debug)]
pub struct Room {
    pub id: u16,
    pub tiles: Vec<(i32, i32)>,
    pub size: usize,
    pub is_roofed: bool, // all tiles have roof
    pub has_campfire: bool,
    pub has_bed: bool,
    pub has_light: bool,     // any light source (lamp, campfire)
    pub has_table: bool,     // future: table furniture
    pub has_door: bool,      // at least one door in the boundary
    pub label: &'static str, // auto-detected room type
}

impl Room {
    /// Mood modifier for sleeping in this room
    pub fn sleep_mood(&self) -> f32 {
        let mut mood = 0.0;
        if self.is_roofed {
            mood += 3.0; // roof over head
        }
        if self.has_bed {
            mood += 5.0; // proper bed
        }
        if self.has_light {
            mood += 1.0; // not sleeping in darkness
        }
        if self.size >= 4 && self.size <= 16 {
            mood += 2.0; // cozy room (not too small, not too big)
        } else if self.size < 4 {
            mood -= 2.0; // cramped
        }
        mood
    }

    /// Mood modifier for eating in this room
    pub fn eat_mood(&self) -> f32 {
        let mut mood = 0.0;
        if self.is_roofed {
            mood += 2.0;
        }
        if self.has_table {
            mood += 5.0; // ate at a table
        }
        if self.has_campfire {
            mood += 2.0; // warm meal by the fire
        }
        mood
    }

    /// Auto-detect room label from contents
    fn detect_label(&mut self) {
        self.label = if self.has_bed && self.has_light {
            "Bedroom"
        } else if self.has_bed {
            "Sleeping quarters"
        } else if self.has_campfire && self.size <= 12 {
            "Kitchen"
        } else if self.has_campfire {
            "Common room"
        } else if self.size <= 4 {
            "Closet"
        } else if self.size > 30 {
            "Hall"
        } else {
            "Room"
        };
    }
}

/// Detect all enclosed rooms in the grid.
/// Returns a list of rooms and a per-tile room ID map (0 = no room).
pub fn detect_rooms(
    grid_data: &[u32],
    wall_data: &[u16],
    doors: &[crate::grid::Door],
) -> (Vec<Room>, Vec<u16>) {
    let w = GRID_W as i32;
    let h = GRID_H as i32;
    let total = (GRID_W * GRID_H) as usize;
    let mut room_map: Vec<u16> = vec![0; total]; // 0 = unassigned
    let mut rooms: Vec<Room> = Vec::new();
    let mut next_id: u16 = 1;

    // Build a door passability lookup
    let door_open: std::collections::HashSet<(i32, i32)> = doors
        .iter()
        .filter(|d| d.is_passable())
        .map(|d| (d.x, d.y))
        .collect();

    for start_y in 0..h {
        for start_x in 0..w {
            let start_idx = (start_y as u32 * GRID_W + start_x as u32) as usize;
            if room_map[start_idx] != 0 {
                continue; // already assigned
            }
            // Only start from roofed, walkable tiles
            if roof_height_rs(grid_data[start_idx]) == 0 {
                continue; // not roofed
            }
            let bt = block_type_rs(grid_data[start_idx]);
            let bh = block_height_rs(grid_data[start_idx]);
            if bh > 0 && is_wall_block(bt) {
                continue; // wall block, not floor
            }

            // Flood fill from this tile
            let mut tiles: Vec<(i32, i32)> = Vec::new();
            let mut stack: Vec<(i32, i32)> = vec![(start_x, start_y)];
            let mut is_enclosed = true;
            let mut has_campfire = false;
            let mut has_bed = false;
            let mut has_light = false;
            let mut has_door = false;
            let mut all_roofed = true;

            while let Some((cx, cy)) = stack.pop() {
                let idx = (cy as u32 * GRID_W + cx as u32) as usize;
                if room_map[idx] != 0 {
                    continue;
                }

                // Check if this tile is roofed
                if roof_height_rs(grid_data[idx]) == 0 {
                    // Reached an unroofed tile — room is not fully enclosed
                    // But don't stop — mark as not enclosed and continue to find full extent
                    all_roofed = false;
                    is_enclosed = false;
                    continue; // don't expand through unroofed tiles
                }

                // Check if tile is a solid wall (don't include walls in the room)
                let bt2 = block_type_rs(grid_data[idx]);
                let bh2 = block_height_rs(grid_data[idx]);
                if bh2 > 0 && is_wall_block(bt2) {
                    continue;
                }

                room_map[idx] = next_id;
                tiles.push((cx, cy));

                // Detect amenities
                match bt2 {
                    BT_CAMPFIRE | BT_FIREPLACE => {
                        has_campfire = true;
                        has_light = true;
                    }
                    BT_BED => has_bed = true,
                    BT_CEILING_LIGHT | BT_TABLE_LAMP | BT_FLOOR_LAMP => has_light = true,
                    _ => {}
                }

                // Cap room size to prevent runaway fills
                if tiles.len() > 200 {
                    is_enclosed = false;
                    break;
                }

                // Expand to 4 neighbors, checking wall edges
                let neighbors = [(0, -1, 0u8, 2u8), (1, 0, 1, 3), (0, 1, 2, 0), (-1, 0, 3, 1)];
                for &(dx, dy, from_edge, to_edge) in &neighbors {
                    let nx = cx + dx;
                    let ny = cy + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                    if room_map[nidx] != 0 {
                        continue;
                    }

                    // Check if wall blocks this edge
                    let wd_here = wall_data[idx];
                    let wd_there = wall_data[nidx];

                    // Wall on our side facing neighbor?
                    let blocked_here = wd_has_edge(wd_here, from_edge)
                        && !((wd_here & WD_HAS_DOOR) != 0
                            && (door_open.contains(&(cx, cy)) || (wd_here & WD_DOOR_OPEN) != 0));

                    // Wall on neighbor's side facing us?
                    let blocked_there = wd_has_edge(wd_there, to_edge)
                        && !((wd_there & WD_HAS_DOOR) != 0
                            && (door_open.contains(&(nx, ny)) || (wd_there & WD_DOOR_OPEN) != 0));

                    if blocked_here || blocked_there {
                        // Wall blocks passage — check for door
                        if (wd_here & WD_HAS_DOOR) != 0 || (wd_there & WD_HAS_DOOR) != 0 {
                            has_door = true;
                        }
                        continue;
                    }

                    stack.push((nx, ny));
                }
            }

            if tiles.len() >= 2 && is_enclosed {
                let mut room = Room {
                    id: next_id,
                    size: tiles.len(),
                    tiles,
                    is_roofed: all_roofed,
                    has_campfire,
                    has_bed,
                    has_light,
                    has_table: false, // future
                    has_door,
                    label: "Room",
                };
                room.detect_label();
                rooms.push(room);
                next_id += 1;
            } else {
                // Not a valid room — clear the room_map entries
                for &(tx, ty) in &tiles {
                    let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                    room_map[tidx] = 0;
                }
            }
        }
    }

    (rooms, room_map)
}

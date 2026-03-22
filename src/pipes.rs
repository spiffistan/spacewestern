//! Pipe network simulation — 1D pressure/flow system overlaid on the block grid.
//!
//! Components: Pipe (15), Pump (16), Tank (17), Valve (18), Outlet (19).
//! Simulation runs on CPU each frame. Gas composition (smoke, O2, CO2, temp)
//! flows through the pipe network from high to low pressure.

use crate::grid::{GRID_W, GRID_H, block_type_rs, block_flags_rs, DIR_MASKS};

/// For a 3-tile bridge, find the partner entry/exit tile index.
/// Bridge segment 0 (entry) connects to segment 2 (exit) 2 tiles away in the bridge direction.
/// Returns None if the tile isn't a bridge entry/exit or partner is out of bounds.
pub fn bridge_partner(grid: &[u32], idx: u32) -> Option<u32> {
    let block = grid[idx as usize];
    let bt = block_type_rs(block);
    if bt != 50 { return None; }
    let flags = block_flags_rs(block);
    let seg = (flags >> 3) & 3;
    let rot = (flags >> 5) & 3;
    if seg == 1 { return None; } // middle tile has no partner
    // Direction offsets based on rotation
    let (dx, dy): (i32, i32) = match rot {
        0 => (0, 1),  // N: entry is north, exit is south (+y)
        1 => (1, 0),  // E: entry is west, exit is east (+x)
        2 => (0, -1), // S: reversed
        _ => (-1, 0), // W: reversed
    };
    let sign = if seg == 0 { 2i32 } else { -2 }; // entry looks +2, exit looks -2
    let x = (idx % GRID_W) as i32 + dx * sign;
    let y = (idx / GRID_W) as i32 + dy * sign;
    if x < 0 || y < 0 || x >= GRID_W as i32 || y >= GRID_H as i32 { return None; }
    Some(y as u32 * GRID_W + x as u32)
}

/// Per-pipe-block state.
#[derive(Clone, Debug)]
pub struct PipeCell {
    pub pressure: f32,         // internal pressure (0 = atmospheric, >0 = pressurized)
    pub gas: [f32; 4],         // [smoke, O2, CO2, temperature]
    pub volume: f32,           // effective volume (tank=10, pipe=1)
    pub pump_rate: f32,        // for pumps/inlets: adjustable flow rate (0-20)
    pub flow_x: f32,           // net flow direction X (positive = east)
    pub flow_y: f32,           // net flow direction Y (positive = south)
}

impl Default for PipeCell {
    fn default() -> Self {
        PipeCell {
            pressure: 0.0,
            gas: [0.0, 1.0, 0.0, 15.0],
            volume: 1.0,
            pump_rate: 24.0, // default pump speed (3x previous)
            flow_x: 0.0,
            flow_y: 0.0,
        }
    }
}

/// Which sides of a pipe block are connected to other pipe components.
/// Used for rendering (straight, corner, T, cross) and flow simulation.
#[derive(Clone, Copy, Debug, Default)]
pub struct PipeConnections {
    pub north: bool,
    pub south: bool,
    pub east: bool,
    pub west: bool,
}

impl PipeConnections {
    pub fn count(&self) -> u32 {
        self.north as u32 + self.south as u32 + self.east as u32 + self.west as u32
    }

    /// Encode as a u32 bitmask for the shader (bit0=N, bit1=S, bit2=E, bit3=W)
    pub fn as_bits(&self) -> u32 {
        (self.north as u32) | ((self.south as u32) << 1) | ((self.east as u32) << 2) | ((self.west as u32) << 3)
    }
}

/// Check if a block type is part of the gas pipe network.
pub fn is_gas_pipe_component(bt: u8) -> bool {
    (bt >= 15 && bt <= 20) || bt == 46 || bt == 50
}

/// Check if a block type is part of the liquid pipe network.
pub fn is_liquid_pipe_component(bt: u8) -> bool {
    bt == 49 || bt == 50 || bt == 52 || bt == 53 || bt == 54
}

/// Check if a block type is part of ANY pipe network (gas or liquid).
pub fn is_pipe_component(bt: u8) -> bool {
    is_gas_pipe_component(bt) || is_liquid_pipe_component(bt)
}

/// Compute connections for a pipe block at (x, y) by checking neighbors.
pub fn get_connections(grid: &[u32], x: i32, y: i32) -> PipeConnections {
    let check = |nx: i32, ny: i32| -> bool {
        if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
            return false;
        }
        let b = grid[(ny as u32 * GRID_W + nx as u32) as usize];
        is_pipe_component(block_type_rs(b))
    };

    PipeConnections {
        north: check(x, y - 1),
        south: check(x, y + 1),
        east: check(x + 1, y),
        west: check(x - 1, y),
    }
}

/// The pipe network state, rebuilt when grid changes.
pub struct PipeNetwork {
    /// Map from grid index (y * GRID_W + x) to pipe cell state.
    /// Only contains entries for pipe component blocks.
    pub cells: std::collections::HashMap<u32, PipeCell>,
}

impl PipeNetwork {
    pub fn new() -> Self {
        PipeNetwork {
            cells: std::collections::HashMap::new(),
        }
    }

    /// Rebuild the network from the grid using a component predicate.
    /// Preserves existing cell state — only adds/removes as needed.
    pub fn rebuild_with(&mut self, grid: &[u32], is_component: fn(u8) -> bool) {
        self.cells.retain(|&idx, _| {
            let bt = block_type_rs(grid[idx as usize]);
            is_component(bt)
        });
        for y in 0..GRID_H {
            for x in 0..GRID_W {
                let idx = y * GRID_W + x;
                let bt = block_type_rs(grid[idx as usize]);
                if is_component(bt) && !self.cells.contains_key(&idx) {
                    let mut cell = PipeCell::default();
                    if bt == 17 { cell.volume = 10.0; }
                    self.cells.insert(idx, cell);
                }
            }
        }
    }

    /// Rebuild using the default gas pipe predicate.
    pub fn rebuild(&mut self, grid: &[u32]) {
        self.rebuild_with(grid, is_gas_pipe_component);
    }

    /// Simulate one tick of pressure equalization.
    /// `dt` is the frame delta time.
    /// `grid` is the block grid for reading valve states, pump directions, etc.
    /// Returns a list of (x, y, gas[4], velocity) for outlet injections.
    pub fn tick(&mut self, dt: f32, grid: &[u32], _pipe_width: f32) -> Vec<(f32, f32, [f32; 4], f32)> {
        let mut outlet_injections = Vec::new();
        let indices: Vec<u32> = self.cells.keys().copied().collect();

        // --- Pressure relaxation: multiple iterations like the voltage system ---
        // Pumps SET pressure (voltage source), outlets DRAIN (consumer), pipes MIX (wire relaxation).
        let iterations = 4;
        for _ in 0..iterations {
            for &idx in &indices {
                let x = (idx % GRID_W) as i32;
                let y = (idx / GRID_W) as i32;
                let block = grid[idx as usize];
                let bt = block_type_rs(block);
                let flags = block_flags_rs(block);

                // Closed valve: no flow
                if bt == 18 && (flags & 4) == 0 { continue; }

                // Pumps: maintain target pressure (like generators maintain voltage)
                if bt == 16 || bt == 53 {
                    let rate = self.cells[&idx].pump_rate;
                    let cur = self.cells[&idx].pressure;
                    // Mix toward target — pump pushes harder when pressure is low
                    if let Some(cell) = self.cells.get_mut(&idx) {
                        cell.pressure = cur + (rate - cur).max(0.0) * 0.3;
                    }
                    continue;
                }

                // Gas inlet: also acts as a pressure source
                if bt == 20 {
                    let rate = self.cells[&idx].pump_rate;
                    let cur = self.cells[&idx].pressure;
                    if let Some(cell) = self.cells.get_mut(&idx) {
                        cell.pressure = cur + (rate * 0.8 - cur).max(0.0) * 0.2;
                    }
                    continue;
                }

                // Outlets/outputs: drain toward 0 (like consumer load)
                if bt == 19 || bt == 54 {
                    if let Some(cell) = self.cells.get_mut(&idx) {
                        cell.pressure *= 0.5; // drain 50% per iteration → rapid convergence to low
                    }
                    // Don't skip — also do relaxation below so flow reaches the outlet
                }

                // --- Relaxation: mix toward neighbor average (like wire voltage) ---
                let pipe_h = (block >> 8) & 0xFF;
                let conn_mask = pipe_h >> 4;
                let is_bridge = bt == 50;
                let bridge_seg = ((flags >> 3) & 3) as i32;
                let bridge_rot = ((flags >> 5) & 3) as i32;

                let mut neighbor_sum = 0.0f32;
                let mut neighbor_count = 0.0f32;

                // Bridge partner (teleport)
                if is_bridge && (bridge_seg == 0 || bridge_seg == 2) {
                    if let Some(pidx) = bridge_partner(grid, idx) {
                        if let Some(p) = self.cells.get(&pidx) {
                            neighbor_sum += p.pressure;
                            neighbor_count += 1.0;
                        }
                    }
                }

                let dir_masks = DIR_MASKS;
                for &(dx, dy, dmask) in &dir_masks {
                    // Bridge direction filtering
                    if is_bridge {
                        let is_ns = dy != 0;
                        let is_ew = dx != 0;
                        let bridge_is_ns = bridge_rot % 2 == 0;
                        if bridge_seg == 1 {
                            if bridge_is_ns && is_ns { continue; }
                            if !bridge_is_ns && is_ew { continue; }
                        } else {
                            let (out_dx, out_dy) = match bridge_rot {
                                0 => (0, if bridge_seg == 0 { -1 } else { 1 }),
                                1 => (if bridge_seg == 0 { -1 } else { 1 }, 0),
                                2 => (0, if bridge_seg == 0 { 1 } else { -1 }),
                                _ => (if bridge_seg == 0 { 1 } else { -1 }, 0),
                            };
                            if dx != out_dx || dy != out_dy { continue; }
                        }
                    }
                    if (bt == 15 || bt == 46 || bt == 49) && conn_mask != 0 && (conn_mask & dmask) == 0 { continue; }

                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 { continue; }
                    let nidx = ny as u32 * GRID_W + nx as u32;
                    if let Some(neighbor) = self.cells.get(&nidx) {
                        let nb = grid[nidx as usize];
                        let nbt = block_type_rs(nb);
                        let nflags = block_flags_rs(nb);
                        if nbt == 18 && (nflags & 4) == 0 { continue; } // closed valve
                        neighbor_sum += neighbor.pressure;
                        neighbor_count += 1.0;
                    }
                }

                if neighbor_count > 0.0 {
                    let avg = neighbor_sum / neighbor_count;
                    if let Some(cell) = self.cells.get_mut(&idx) {
                        // Restrictor: reduced relaxation rate (bottleneck)
                        let alpha = if bt == 46 {
                            let level = ((block >> 8) & 0x0F) as f32;
                            0.1 * (level / 10.0).max(0.05)
                        } else {
                            0.5 // fast relaxation for normal pipes
                        };
                        cell.pressure = cell.pressure * (1.0 - alpha) + avg * alpha;
                    }
                }
            }
        }

        // --- Gas composition transfer (single pass, delta-based) ---
        let mut gas_delta: std::collections::HashMap<u32, [f32; 4]> = std::collections::HashMap::new();
        let mut flow_accum: std::collections::HashMap<u32, (f32, f32)> = std::collections::HashMap::new();

        for &idx in &indices {
            let x = (idx % GRID_W) as i32;
            let y = (idx / GRID_W) as i32;
            let block = grid[idx as usize];
            let bt = block_type_rs(block);
            let flags = block_flags_rs(block);
            if bt == 18 && (flags & 4) == 0 { continue; }
            let cell = &self.cells[&idx];
            let pipe_h = (block >> 8) & 0xFF;
            let conn_mask = pipe_h >> 4;
            let dir_masks: [(i32, i32, u32); 4] = [(0, -1, 0x1), (0, 1, 0x4), (1, 0, 0x2), (-1, 0, 0x8)];

            for &(dx, dy, dmask) in &dir_masks {
                if (bt == 15 || bt == 46 || bt == 49) && conn_mask != 0 && (conn_mask & dmask) == 0 { continue; }
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 { continue; }
                let nidx = ny as u32 * GRID_W + nx as u32;
                if let Some(neighbor) = self.cells.get(&nidx) {
                    let dp = cell.pressure - neighbor.pressure;
                    // Flow direction for visualization
                    if dp.abs() > 0.001 {
                        let fa = flow_accum.entry(idx).or_insert((0.0, 0.0));
                        fa.0 += dx as f32 * dp;
                        fa.1 += dy as f32 * dp;
                    }
                    // Gas transfer: pressure-driven (advection) + diffusion
                    // Advection: gas flows from high to low pressure
                    let adv_rate = if dp > 0.0 { (dp * 0.05).min(0.15) } else { 0.0 };
                    // Diffusion: gas equalizes regardless of pressure (heat conduction, mixing)
                    let diff_rate = dt * 1.5;
                    let rate = (adv_rate + diff_rate).min(0.25);
                    if rate > 0.001 {
                        let gd = gas_delta.entry(nidx).or_insert([0.0; 4]);
                        for i in 0..4 { gd[i] += (cell.gas[i] - neighbor.gas[i]) * rate; }
                    }
                }
            }

            // Gas inlet: extract gas from room
            if bt == 20 {
                let mut env_gas = [0.0f32, 1.0, 0.0, 15.0];
                let mut found_room = false;
                for &(adx, ady) in &[(0i32, -1i32), (0, 1), (1, 0), (-1, 0)] {
                    let ax = x + adx;
                    let ay = y + ady;
                    if ax < 0 || ay < 0 || ax >= GRID_W as i32 || ay >= GRID_H as i32 { continue; }
                    let ab = grid[(ay as u32 * GRID_W + ax as u32) as usize];
                    let abt = block_type_rs(ab);
                    if is_pipe_component(abt) { continue; }
                    let adj_has_roof = (block_flags_rs(ab) & 2) != 0;
                    let mut near_fire = false;
                    for fy in -5i32..=5 {
                        for fx in -5i32..=5 {
                            let cx = x + fx; let cy = y + fy;
                            if cx < 0 || cy < 0 || cx >= GRID_W as i32 || cy >= GRID_H as i32 { continue; }
                            if block_type_rs(grid[(cy as u32 * GRID_W + cx as u32) as usize]) == 6 { near_fire = true; break; }
                        }
                        if near_fire { break; }
                    }
                    if adj_has_roof && near_fire { env_gas = [0.6, 0.5, 0.25, 80.0]; found_room = true; }
                    else if adj_has_roof { env_gas = [0.0, 0.9, 0.02, 18.0]; found_room = true; }
                    if found_room { break; }
                }
                let gd = gas_delta.entry(idx).or_insert([0.0; 4]);
                for i in 0..4 { gd[i] += (env_gas[i] - cell.gas[i]) * 3.0 * dt; }
            }

            // Liquid intake: water properties
            if bt == 52 {
                let seg = (flags >> 3) & 3;
                let has_water = if seg == 1 { true } else {
                    let mut found = false;
                    for &(adx, ady) in &[(0i32, -1i32), (0, 1), (1, 0), (-1, 0)] {
                        let ax = x + adx; let ay = y + ady;
                        if ax < 0 || ay < 0 || ax >= GRID_W as i32 || ay >= GRID_H as i32 { continue; }
                        let ab = grid[(ay as u32 * GRID_W + ax as u32) as usize];
                        let abt_val = block_type_rs(ab);
                        if abt_val == 3 || abt_val == 32 { found = true; break; }
                        if abt_val == 52 && ((block_flags_rs(ab) >> 3) & 3) == 1 { found = true; break; }
                    }
                    found
                };
                if has_water {
                    let gd = gas_delta.entry(idx).or_insert([0.0; 4]);
                    let water_gas = [0.0f32, 0.0, 0.0, 15.0];
                    for i in 0..4 { gd[i] += (water_gas[i] - cell.gas[i]) * 2.0 * dt; }
                }
            }
        }

        // Apply gas deltas
        for (&idx, gd) in &gas_delta {
            if let Some(cell) = self.cells.get_mut(&idx) {
                for i in 0..4 { cell.gas[i] = (cell.gas[i] + gd[i]).max(0.0); }
                cell.gas[0] = cell.gas[0].min(2.0);
                cell.gas[1] = cell.gas[1].min(1.0);
                cell.gas[2] = cell.gas[2].min(2.0);
                cell.gas[3] = cell.gas[3].clamp(-20.0, 500.0);
            }
        }

        // Flow vectors (smoothed)
        for (&idx, &(fx, fy)) in &flow_accum {
            if let Some(cell) = self.cells.get_mut(&idx) {
                cell.flow_x = cell.flow_x * 0.8 + fx * 0.2;
                cell.flow_y = cell.flow_y * 0.8 + fy * 0.2;
            }
        }
        for (&idx, cell) in self.cells.iter_mut() {
            if !flow_accum.contains_key(&idx) {
                cell.flow_x *= 0.9;
                cell.flow_y *= 0.9;
            }
        }

        // Pressure cap
        for cell in self.cells.values_mut() {
            cell.pressure = cell.pressure.min(50.0);
        }

        // Outlet emissions: emit the drained pressure as world injections
        for &idx in &indices {
            let real_idx = idx as usize;
            if real_idx >= grid.len() { continue; }
            let bt = block_type_rs(grid[real_idx]);
            if bt == 19 || bt == 54 {
                if let Some(cell) = self.cells.get_mut(&idx) {
                    if cell.pressure > 0.01 {
                        let drain = cell.pressure * 0.5;
                        cell.pressure -= drain;
                        let x = (idx % GRID_W) as f32 + 0.5;
                        let y = (idx / GRID_W) as f32 + 0.5;
                        outlet_injections.push((x, y, cell.gas, drain));
                    }
                }
            }
        }

        // Thermal dissipation: pipes lose heat to ambient
        let ambient_temp = 15.0;
        for cell in self.cells.values_mut() {
            let temp_diff = cell.gas[3] - ambient_temp;
            cell.gas[3] -= temp_diff * 0.005 * dt;
        }

        outlet_injections
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::make_block;

    /// Create a small test grid with specific blocks at given positions.
    fn test_grid(blocks: &[((u32, u32), u8, u8)]) -> Vec<u32> {
        let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize]; // all dirt
        for &((x, y), bt, h) in blocks {
            grid[(y * GRID_W + x) as usize] = make_block(bt, h, 0);
        }
        grid
    }

    #[test]
    fn test_liquid_pump_builds_pressure() {
        // Layout: liquid_pipe(49) — liquid_pump(53) — liquid_pipe(49)
        let grid = test_grid(&[
            ((10, 10), 49, 1), // liquid pipe
            ((11, 10), 53, 1), // liquid pump
            ((12, 10), 49, 1), // liquid pipe
        ]);
        let mut net = PipeNetwork::new();
        net.rebuild_with(&grid, is_liquid_pipe_component);

        assert_eq!(net.cells.len(), 3, "Should have 3 cells");
        assert!(net.cells.contains_key(&(10 * GRID_W + 10)), "pipe cell");
        assert!(net.cells.contains_key(&(10 * GRID_W + 11)), "pump cell");
        assert!(net.cells.contains_key(&(10 * GRID_W + 12)), "pipe cell");

        // Run several ticks — pump should build pressure
        for _ in 0..60 {
            net.tick(1.0 / 60.0, &grid, 5.0);
        }
        let pump_pressure = net.cells[&(10 * GRID_W + 11)].pressure;
        assert!(pump_pressure > 1.0, "Pump should have built pressure, got {}", pump_pressure);
        // Neighbors should also have pressure from flow
        let pipe_pressure = net.cells[&(10 * GRID_W + 10)].pressure;
        assert!(pipe_pressure > 0.1, "Pipe should have pressure from pump, got {}", pipe_pressure);
    }

    #[test]
    fn test_liquid_output_creates_injections() {
        // Layout: liquid_pump(53) — liquid_pipe(49) — liquid_output(54)
        let grid = test_grid(&[
            ((10, 10), 53, 1),
            ((11, 10), 49, 1),
            ((12, 10), 54, 1),
        ]);
        let mut net = PipeNetwork::new();
        net.rebuild_with(&grid, is_liquid_pipe_component);

        assert_eq!(net.cells.len(), 3);

        // Run enough ticks for pressure to build and reach output
        let mut total_injections = 0;
        for _ in 0..120 {
            let inj = net.tick(1.0 / 60.0, &grid, 5.0);
            total_injections += inj.len();
        }
        assert!(total_injections > 0, "Output should have produced injections");
        let output_pressure = net.cells[&(10 * GRID_W + 12)].pressure;
        // Output releases pressure, so it should be less than the pump
        let pump_pressure = net.cells[&(10 * GRID_W + 10)].pressure;
        assert!(pump_pressure > output_pressure,
            "Pump ({:.2}) should have more pressure than output ({:.2})", pump_pressure, output_pressure);
    }

    #[test]
    fn test_liquid_network_isolation_from_gas() {
        // Gas pipe and liquid pipe next to each other — should NOT flow between them
        let grid = test_grid(&[
            ((10, 10), 15, 1),  // gas pipe
            ((11, 10), 49, 1),  // liquid pipe
        ]);
        let mut gas_net = PipeNetwork::new();
        gas_net.rebuild(&grid);
        let mut liq_net = PipeNetwork::new();
        liq_net.rebuild_with(&grid, is_liquid_pipe_component);

        assert_eq!(gas_net.cells.len(), 1, "Gas net should have 1 cell");
        assert_eq!(liq_net.cells.len(), 1, "Liquid net should have 1 cell");
        // They don't share cells
        assert!(gas_net.cells.contains_key(&(10 * GRID_W + 10)));
        assert!(!gas_net.cells.contains_key(&(10 * GRID_W + 11)));
        assert!(liq_net.cells.contains_key(&(10 * GRID_W + 11)));
        assert!(!liq_net.cells.contains_key(&(10 * GRID_W + 10)));
    }

    #[test]
    fn test_intake_to_output_full_pipeline() {
        // Realistic setup: intake(seg0 on ground, seg1 on water) → pipe → output
        // Full pipeline: intake → pump → pipe → pipe → output
        let mut grid = test_grid(&[
            ((10, 10), 52, 1),  // intake seg 0 (ground side)
            ((11, 10), 52, 1),  // intake seg 1 (water side)
            ((12, 10), 53, 1),  // liquid pump (provides pressure)
            ((13, 10), 49, 1),  // liquid pipe
            ((14, 10), 54, 1),  // liquid output
        ]);
        // Set intake seg 1 flags
        grid[(10 * GRID_W + 11) as usize] = make_block(52, 1, 1 << 3);

        let mut net = PipeNetwork::new();
        net.rebuild_with(&grid, is_liquid_pipe_component);
        assert_eq!(net.cells.len(), 5, "All 5 tiles should have cells");

        // Run for 2 seconds (120 ticks)
        let mut total_injections = 0;
        for _ in 0..120 {
            let inj = net.tick(1.0 / 60.0, &grid, 5.0);
            total_injections += inj.len();
        }

        // Pump should have built pressure
        let pump_p = net.cells[&(10 * GRID_W + 12)].pressure;
        assert!(pump_p > 0.5, "Pump should have pressure, got {:.2}", pump_p);

        // Output should have produced injections
        assert!(total_injections > 0, "Output should have produced water injections");
    }

    #[test]
    fn test_intake_without_pump_no_pressure() {
        // Intake alone should NOT generate pressure — needs a pump
        let mut grid = test_grid(&[
            ((10, 10), 52, 1),  // intake seg 0
            ((11, 10), 52, 1),  // intake seg 1
            ((12, 10), 49, 1),  // liquid pipe
        ]);
        grid[(10 * GRID_W + 11) as usize] = make_block(52, 1, 1 << 3);

        let mut net = PipeNetwork::new();
        net.rebuild_with(&grid, is_liquid_pipe_component);

        for _ in 0..60 {
            net.tick(1.0 / 60.0, &grid, 5.0);
        }
        let p = net.cells[&(10 * GRID_W + 12)].pressure;
        assert!(p < 0.01, "Pipe should have no pressure without pump, got {:.4}", p);
    }
}

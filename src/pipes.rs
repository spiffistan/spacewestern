//! Pipe network simulation — 1D pressure/flow system overlaid on the block grid.
//!
//! Components: Pipe, Pump, Tank, Valve, Outlet, Inlet, Restrictor, Bridge.
//! Simulation runs on CPU each frame. Gas composition (smoke, O2, CO2, temp)
//! flows through the pipe network from high to low pressure.

use crate::grid::*;

// --- Pipe simulation constants ---
const TANK_VOLUME: f32 = 10.0;
const DEFAULT_PUMP_RATE: f32 = 24.0;

// Pressure relaxation
const PRESSURE_ITERATIONS: usize = 4;
const PUMP_CONVERGENCE_RATE: f32 = 0.3;
const INLET_SOURCE_FACTOR: f32 = 0.8;
const INLET_CONVERGENCE_RATE: f32 = 0.2;
const OUTLET_DRAIN_FACTOR: f32 = 0.5;
const PIPE_RELAXATION_ALPHA: f32 = 0.5;
const RESTRICTOR_MAX_LEVEL: f32 = 10.0;
const RESTRICTOR_BASE_ALPHA: f32 = 0.1;
const RESTRICTOR_MIN_ALPHA: f32 = 0.05;
const MAX_PRESSURE: f32 = 50.0;
const OUTLET_EMISSION_THRESHOLD: f32 = 0.01;

// Gas transfer
const ADVECTION_SCALE: f32 = 0.05;
const ADVECTION_MAX: f32 = 0.15;
const DIFFUSION_RATE: f32 = 1.5;
const MAX_TRANSFER_RATE: f32 = 0.25;
const MIN_TRANSFER_RATE: f32 = 0.001;
const FLOW_THRESHOLD: f32 = 0.001;
const INLET_ABSORPTION_RATE: f32 = 3.0;
const INTAKE_ABSORPTION_RATE: f32 = 2.0;

// Gas composition limits
const MAX_SMOKE: f32 = 2.0;
const MAX_O2: f32 = 1.0;
const MAX_CO2: f32 = 2.0;
const MIN_TEMP: f32 = -20.0;
const MAX_TEMP: f32 = 500.0;

// Flow smoothing
const FLOW_RETAIN: f32 = 0.8;
const FLOW_NEW: f32 = 0.2;
const FLOW_DECAY: f32 = 0.9;

// Thermal dissipation
const AMBIENT_TEMP: f32 = 15.0;
const THERMAL_DISSIPATION: f32 = 0.005;

// Gas composition indices
const GAS_SMOKE: usize = 0;
const GAS_O2: usize = 1;
const GAS_CO2: usize = 2;
const GAS_TEMP: usize = 3;

// Default atmospheric gas composition
const ATMOSPHERE: [f32; 4] = [0.0, 1.0, 0.0, 15.0];
const SMOKY_ROOM: [f32; 4] = [0.6, 0.5, 0.25, 80.0];
const CLEAN_ROOM: [f32; 4] = [0.0, 0.9, 0.02, 18.0];
const WATER_GAS: [f32; 4] = [0.0, 0.0, 0.0, 15.0];

// Fire detection radius for inlet gas sampling
const FIRE_SCAN_RADIUS: i32 = 5;

/// For a 3-tile bridge, find the partner entry/exit tile index.
/// Bridge segment 0 (entry) connects to segment 2 (exit) 2 tiles away.
pub fn bridge_partner(grid: &[u32], idx: u32) -> Option<u32> {
    let block = grid[idx as usize];
    let bt = block_type_rs(block);
    if bt != BT_PIPE_BRIDGE {
        return None;
    }
    let flags = block_flags_rs(block);
    let seg = (flags >> 3) & 3;
    if seg == 1 {
        return None;
    } // middle tile has no partner
    let rot = (flags >> 5) & 3;
    let (dx, dy): (i32, i32) = match rot {
        0 => (0, 1),  // N: south
        1 => (1, 0),  // E: east
        2 => (0, -1), // S: north
        _ => (-1, 0), // W: west
    };
    let sign = if seg == 0 { 2i32 } else { -2 };
    let x = (idx % GRID_W) as i32 + dx * sign;
    let y = (idx / GRID_W) as i32 + dy * sign;
    if x < 0 || y < 0 || x >= GRID_W as i32 || y >= GRID_H as i32 {
        return None;
    }
    Some(y as u32 * GRID_W + x as u32)
}

/// Per-pipe-block state.
#[derive(Clone, Debug)]
pub struct PipeCell {
    pub pressure: f32,
    pub gas: [f32; 4], // [smoke, O2, CO2, temperature]
    pub volume: f32,
    pub pump_rate: f32,
    pub flow_x: f32,
    pub flow_y: f32,
}

impl Default for PipeCell {
    fn default() -> Self {
        PipeCell {
            pressure: 0.0,
            gas: ATMOSPHERE,
            volume: 1.0,
            pump_rate: DEFAULT_PUMP_RATE,
            flow_x: 0.0,
            flow_y: 0.0,
        }
    }
}

/// Which sides of a pipe block are connected to other pipe components.
#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub struct PipeConnections {
    pub north: bool,
    pub south: bool,
    pub east: bool,
    pub west: bool,
}

#[allow(dead_code)]
impl PipeConnections {
    pub fn count(&self) -> u32 {
        self.north as u32 + self.south as u32 + self.east as u32 + self.west as u32
    }

    pub fn as_bits(&self) -> u32 {
        (self.north as u32)
            | ((self.south as u32) << 1)
            | ((self.east as u32) << 2)
            | ((self.west as u32) << 3)
    }
}

pub fn is_gas_pipe_component(bt: u32) -> bool {
    bt_is!(
        bt,
        BT_PIPE,
        BT_PUMP,
        BT_TANK,
        BT_VALVE,
        BT_OUTLET,
        BT_INLET,
        BT_RESTRICTOR,
        BT_PIPE_BRIDGE
    )
}

pub fn is_liquid_pipe_component(bt: u32) -> bool {
    bt_is!(
        bt,
        BT_LIQUID_PIPE,
        BT_PIPE_BRIDGE,
        BT_LIQUID_INTAKE,
        BT_LIQUID_PUMP,
        BT_LIQUID_OUTPUT
    )
}

pub fn is_pipe_component(bt: u32) -> bool {
    is_gas_pipe_component(bt) || is_liquid_pipe_component(bt)
}

#[allow(dead_code)]
pub fn get_connections(grid: &[u32], x: i32, y: i32) -> PipeConnections {
    let check = |nx: i32, ny: i32| -> bool {
        if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
            return false;
        }
        is_pipe_component(block_type_rs(
            grid[(ny as u32 * GRID_W + nx as u32) as usize],
        ))
    };
    PipeConnections {
        north: check(x, y - 1),
        south: check(x, y + 1),
        east: check(x + 1, y),
        west: check(x - 1, y),
    }
}

/// Helper: check if a neighbor valve is closed.
fn is_closed_valve(grid: &[u32], idx: u32) -> bool {
    let block = grid[idx as usize];
    block_type_rs(block) == BT_VALVE && (block_flags_rs(block) & 4) == 0
}

/// Helper: check if block type uses connection mask for neighbor filtering.
fn uses_connection_mask(bt: u32) -> bool {
    bt_is!(bt, BT_PIPE, BT_RESTRICTOR, BT_LIQUID_PIPE)
}

/// The pipe network state, rebuilt when grid changes.
pub struct PipeNetwork {
    pub cells: std::collections::HashMap<u32, PipeCell>,
    scratch_indices: Vec<u32>,
    scratch_gas_delta: std::collections::HashMap<u32, [f32; 4]>,
    scratch_flow_accum: std::collections::HashMap<u32, (f32, f32)>,
}

impl PipeNetwork {
    pub fn new() -> Self {
        PipeNetwork {
            cells: std::collections::HashMap::new(),
            scratch_indices: Vec::new(),
            scratch_gas_delta: std::collections::HashMap::new(),
            scratch_flow_accum: std::collections::HashMap::new(),
        }
    }

    /// Rebuild the network from the grid using a component predicate.
    pub fn rebuild_with(&mut self, grid: &[u32], is_component: fn(u32) -> bool) {
        self.cells
            .retain(|&idx, _| is_component(block_type_rs(grid[idx as usize])));
        for y in 0..GRID_H {
            for x in 0..GRID_W {
                let idx = y * GRID_W + x;
                let bt = block_type_rs(grid[idx as usize]);
                if is_component(bt) && !self.cells.contains_key(&idx) {
                    let mut cell = PipeCell::default();
                    if bt == BT_TANK {
                        cell.volume = TANK_VOLUME;
                    }
                    self.cells.insert(idx, cell);
                }
            }
        }
    }

    pub fn rebuild(&mut self, grid: &[u32]) {
        self.rebuild_with(grid, is_gas_pipe_component);
    }

    /// Simulate one tick of pressure equalization and gas transfer.
    pub fn tick(
        &mut self,
        dt: f32,
        grid: &[u32],
        _pipe_width: f32,
    ) -> Vec<(f32, f32, [f32; 4], f32)> {
        let mut outlet_injections = Vec::new();
        let mut indices = std::mem::take(&mut self.scratch_indices);
        indices.clear();
        indices.extend(self.cells.keys().copied());

        // --- Pressure relaxation ---
        for _ in 0..PRESSURE_ITERATIONS {
            for &idx in &indices {
                let x = (idx % GRID_W) as i32;
                let y = (idx / GRID_W) as i32;
                let block = grid[idx as usize];
                let bt = block_type_rs(block);
                let flags = block_flags_rs(block);

                if bt == BT_VALVE && (flags & 4) == 0 {
                    continue;
                }

                // Pumps: maintain target pressure
                if bt_is!(bt, BT_PUMP, BT_LIQUID_PUMP) {
                    let rate = self.cells[&idx].pump_rate;
                    let cur = self.cells[&idx].pressure;
                    if let Some(cell) = self.cells.get_mut(&idx) {
                        cell.pressure = cur + (rate - cur).max(0.0) * PUMP_CONVERGENCE_RATE;
                    }
                    continue;
                }

                // Gas inlet: pressure source
                if bt == BT_INLET {
                    let rate = self.cells[&idx].pump_rate;
                    let cur = self.cells[&idx].pressure;
                    if let Some(cell) = self.cells.get_mut(&idx) {
                        cell.pressure = cur
                            + (rate * INLET_SOURCE_FACTOR - cur).max(0.0) * INLET_CONVERGENCE_RATE;
                    }
                    continue;
                }

                // Outlets: drain toward 0
                if bt_is!(bt, BT_OUTLET, BT_LIQUID_OUTPUT) {
                    if let Some(cell) = self.cells.get_mut(&idx) {
                        cell.pressure *= OUTLET_DRAIN_FACTOR;
                    }
                }

                // --- Neighbor pressure relaxation ---
                let pipe_h = (block >> 8) & 0xFF;
                let conn_mask = pipe_h >> 4;
                let is_bridge = bt == BT_PIPE_BRIDGE;
                let bridge_seg = ((flags >> 3) & 3) as i32;
                let bridge_rot = ((flags >> 5) & 3) as i32;

                let mut neighbor_sum = 0.0f32;
                let mut neighbor_count = 0.0f32;

                // Bridge teleport
                if is_bridge && (bridge_seg == 0 || bridge_seg == 2) {
                    if let Some(pidx) = bridge_partner(grid, idx) {
                        if let Some(p) = self.cells.get(&pidx) {
                            neighbor_sum += p.pressure;
                            neighbor_count += 1.0;
                        }
                    }
                }

                for &(dx, dy, dmask) in &DIR_MASKS {
                    // Bridge direction filtering
                    if is_bridge {
                        let is_ns = dy != 0;
                        let is_ew = dx != 0;
                        let bridge_is_ns = bridge_rot % 2 == 0;
                        if bridge_seg == 1 {
                            if bridge_is_ns && is_ns {
                                continue;
                            }
                            if !bridge_is_ns && is_ew {
                                continue;
                            }
                        } else {
                            let (out_dx, out_dy) = match bridge_rot {
                                0 => (0, if bridge_seg == 0 { -1 } else { 1 }),
                                1 => (if bridge_seg == 0 { -1 } else { 1 }, 0),
                                2 => (0, if bridge_seg == 0 { 1 } else { -1 }),
                                _ => (if bridge_seg == 0 { 1 } else { -1 }, 0),
                            };
                            if dx != out_dx || dy != out_dy {
                                continue;
                            }
                        }
                    }
                    if uses_connection_mask(bt) && conn_mask != 0 && (conn_mask & dmask) == 0 {
                        continue;
                    }

                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
                        continue;
                    }
                    let nidx = ny as u32 * GRID_W + nx as u32;
                    if is_closed_valve(grid, nidx) {
                        continue;
                    }
                    if let Some(neighbor) = self.cells.get(&nidx) {
                        neighbor_sum += neighbor.pressure;
                        neighbor_count += 1.0;
                    }
                }

                if neighbor_count > 0.0 {
                    let avg = neighbor_sum / neighbor_count;
                    if let Some(cell) = self.cells.get_mut(&idx) {
                        let alpha = if bt == BT_RESTRICTOR {
                            let level = ((block >> 8) & 0x0F) as f32;
                            RESTRICTOR_BASE_ALPHA
                                * (level / RESTRICTOR_MAX_LEVEL).max(RESTRICTOR_MIN_ALPHA)
                        } else {
                            PIPE_RELAXATION_ALPHA
                        };
                        cell.pressure = cell.pressure * (1.0 - alpha) + avg * alpha;
                    }
                }
            }
        }

        // --- Gas composition transfer ---
        let mut gas_delta = std::mem::take(&mut self.scratch_gas_delta);
        let mut flow_accum = std::mem::take(&mut self.scratch_flow_accum);
        gas_delta.clear();
        flow_accum.clear();

        for &idx in &indices {
            let x = (idx % GRID_W) as i32;
            let y = (idx / GRID_W) as i32;
            let block = grid[idx as usize];
            let bt = block_type_rs(block);
            let flags = block_flags_rs(block);
            if bt == BT_VALVE && (flags & 4) == 0 {
                continue;
            }
            let cell = &self.cells[&idx];
            let pipe_h = (block >> 8) & 0xFF;
            let conn_mask = pipe_h >> 4;

            for &(dx, dy, dmask) in &DIR_MASKS {
                if uses_connection_mask(bt) && conn_mask != 0 && (conn_mask & dmask) == 0 {
                    continue;
                }
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
                    continue;
                }
                let nidx = ny as u32 * GRID_W + nx as u32;
                if let Some(neighbor) = self.cells.get(&nidx) {
                    let dp = cell.pressure - neighbor.pressure;
                    if dp.abs() > FLOW_THRESHOLD {
                        let fa = flow_accum.entry(idx).or_insert((0.0, 0.0));
                        fa.0 += dx as f32 * dp;
                        fa.1 += dy as f32 * dp;
                    }
                    let adv_rate = if dp > 0.0 {
                        (dp * ADVECTION_SCALE).min(ADVECTION_MAX)
                    } else {
                        0.0
                    };
                    let diff_rate = dt * DIFFUSION_RATE;
                    let rate = (adv_rate + diff_rate).min(MAX_TRANSFER_RATE);
                    if rate > MIN_TRANSFER_RATE {
                        let gd = gas_delta.entry(nidx).or_insert([0.0; 4]);
                        for i in 0..4 {
                            gd[i] += (cell.gas[i] - neighbor.gas[i]) * rate;
                        }
                    }
                }
            }

            // Gas inlet: extract gas from adjacent room
            if bt == BT_INLET {
                let env_gas = self.sample_inlet_environment(grid, x, y);
                let gd = gas_delta.entry(idx).or_insert([0.0; 4]);
                for i in 0..4 {
                    gd[i] += (env_gas[i] - cell.gas[i]) * INLET_ABSORPTION_RATE * dt;
                }
            }

            // Liquid intake: water properties
            if bt == BT_LIQUID_INTAKE {
                let seg = (flags >> 3) & 3;
                let has_water = seg == 1 || self.check_adjacent_water(grid, x, y);
                if has_water {
                    let gd = gas_delta.entry(idx).or_insert([0.0; 4]);
                    for i in 0..4 {
                        gd[i] += (WATER_GAS[i] - cell.gas[i]) * INTAKE_ABSORPTION_RATE * dt;
                    }
                }
            }
        }

        // Apply gas deltas
        for (&idx, gd) in &gas_delta {
            if let Some(cell) = self.cells.get_mut(&idx) {
                for i in 0..4 {
                    cell.gas[i] = (cell.gas[i] + gd[i]).max(0.0);
                }
                cell.gas[GAS_SMOKE] = cell.gas[GAS_SMOKE].min(MAX_SMOKE);
                cell.gas[GAS_O2] = cell.gas[GAS_O2].min(MAX_O2);
                cell.gas[GAS_CO2] = cell.gas[GAS_CO2].min(MAX_CO2);
                cell.gas[GAS_TEMP] = cell.gas[GAS_TEMP].clamp(MIN_TEMP, MAX_TEMP);
            }
        }

        // Flow vectors (smoothed)
        for (&idx, &(fx, fy)) in &flow_accum {
            if let Some(cell) = self.cells.get_mut(&idx) {
                cell.flow_x = cell.flow_x * FLOW_RETAIN + fx * FLOW_NEW;
                cell.flow_y = cell.flow_y * FLOW_RETAIN + fy * FLOW_NEW;
            }
        }
        for (&idx, cell) in self.cells.iter_mut() {
            if !flow_accum.contains_key(&idx) {
                cell.flow_x *= FLOW_DECAY;
                cell.flow_y *= FLOW_DECAY;
            }
        }

        // Pressure cap
        for cell in self.cells.values_mut() {
            cell.pressure = cell.pressure.min(MAX_PRESSURE);
        }

        // Outlet emissions
        for &idx in &indices {
            if idx as usize >= grid.len() {
                continue;
            }
            let bt = block_type_rs(grid[idx as usize]);
            if bt_is!(bt, BT_OUTLET, BT_LIQUID_OUTPUT) {
                if let Some(cell) = self.cells.get_mut(&idx) {
                    if cell.pressure > OUTLET_EMISSION_THRESHOLD {
                        let drain = cell.pressure * OUTLET_DRAIN_FACTOR;
                        cell.pressure -= drain;
                        let wx = (idx % GRID_W) as f32 + 0.5;
                        let wy = (idx / GRID_W) as f32 + 0.5;
                        outlet_injections.push((wx, wy, cell.gas, drain));
                    }
                }
            }
        }

        // Thermal dissipation
        for cell in self.cells.values_mut() {
            let temp_diff = cell.gas[GAS_TEMP] - AMBIENT_TEMP;
            cell.gas[GAS_TEMP] -= temp_diff * THERMAL_DISSIPATION * dt;
        }

        self.scratch_indices = indices;
        self.scratch_gas_delta = gas_delta;
        self.scratch_flow_accum = flow_accum;

        outlet_injections
    }

    /// Sample gas composition from the room adjacent to an inlet.
    fn sample_inlet_environment(&self, grid: &[u32], x: i32, y: i32) -> [f32; 4] {
        for &(adx, ady) in &[(0i32, -1i32), (0, 1), (1, 0), (-1, 0)] {
            let ax = x + adx;
            let ay = y + ady;
            if ax < 0 || ay < 0 || ax >= GRID_W as i32 || ay >= GRID_H as i32 {
                continue;
            }
            let ab = grid[(ay as u32 * GRID_W + ax as u32) as usize];
            let abt = block_type_rs(ab);
            if is_pipe_component(abt) {
                continue;
            }
            let has_roof = (block_flags_rs(ab) & 2) != 0;
            if !has_roof {
                continue;
            }

            let near_fire = self.scan_for_fire(grid, x, y);
            return if near_fire { SMOKY_ROOM } else { CLEAN_ROOM };
        }
        ATMOSPHERE
    }

    /// Check if there's a fireplace within scan radius.
    fn scan_for_fire(&self, grid: &[u32], cx: i32, cy: i32) -> bool {
        for fy in -FIRE_SCAN_RADIUS..=FIRE_SCAN_RADIUS {
            for fx in -FIRE_SCAN_RADIUS..=FIRE_SCAN_RADIUS {
                let sx = cx + fx;
                let sy = cy + fy;
                if sx < 0 || sy < 0 || sx >= GRID_W as i32 || sy >= GRID_H as i32 {
                    continue;
                }
                let pbt = block_type_rs(grid[(sy as u32 * GRID_W + sx as u32) as usize]);
                if pbt == BT_FIREPLACE || pbt == BT_CAMPFIRE {
                    return true;
                }
            }
        }
        false
    }

    /// Check if any adjacent tile has water (for liquid intake).
    fn check_adjacent_water(&self, grid: &[u32], x: i32, y: i32) -> bool {
        for &(adx, ady) in &[(0i32, -1i32), (0, 1), (1, 0), (-1, 0)] {
            let ax = x + adx;
            let ay = y + ady;
            if ax < 0 || ay < 0 || ax >= GRID_W as i32 || ay >= GRID_H as i32 {
                continue;
            }
            let ab = grid[(ay as u32 * GRID_W + ax as u32) as usize];
            let abt = block_type_rs(ab);
            if bt_is!(abt, BT_WATER, BT_DUG_GROUND) {
                return true;
            }
            if abt == BT_LIQUID_INTAKE && ((block_flags_rs(ab) >> 3) & 3) == 1 {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::make_block;

    fn test_grid(blocks: &[((u32, u32), u8, u8)]) -> Vec<u32> {
        let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
        for &((x, y), bt, h) in blocks {
            grid[(y * GRID_W + x) as usize] = make_block(bt, h, 0);
        }
        grid
    }

    #[test]
    fn test_liquid_pump_builds_pressure() {
        let grid = test_grid(&[((10, 10), 49, 1), ((11, 10), 53, 1), ((12, 10), 49, 1)]);
        let mut net = PipeNetwork::new();
        net.rebuild_with(&grid, is_liquid_pipe_component);
        assert_eq!(net.cells.len(), 3);
        for _ in 0..60 {
            net.tick(1.0 / 60.0, &grid, 5.0);
        }
        let pump_p = net.cells[&(10 * GRID_W + 11)].pressure;
        assert!(
            pump_p > 1.0,
            "Pump should have built pressure, got {}",
            pump_p
        );
        let pipe_p = net.cells[&(10 * GRID_W + 10)].pressure;
        assert!(
            pipe_p > 0.1,
            "Pipe should have pressure from pump, got {}",
            pipe_p
        );
    }

    #[test]
    fn test_liquid_output_creates_injections() {
        let grid = test_grid(&[((10, 10), 53, 1), ((11, 10), 49, 1), ((12, 10), 54, 1)]);
        let mut net = PipeNetwork::new();
        net.rebuild_with(&grid, is_liquid_pipe_component);
        let mut total = 0;
        for _ in 0..120 {
            total += net.tick(1.0 / 60.0, &grid, 5.0).len();
        }
        assert!(total > 0, "Output should have produced injections");
    }

    #[test]
    fn test_liquid_network_isolation_from_gas() {
        let grid = test_grid(&[((10, 10), 15, 1), ((11, 10), 49, 1)]);
        let mut gas = PipeNetwork::new();
        gas.rebuild(&grid);
        let mut liq = PipeNetwork::new();
        liq.rebuild_with(&grid, is_liquid_pipe_component);
        assert_eq!(gas.cells.len(), 1);
        assert_eq!(liq.cells.len(), 1);
        assert!(gas.cells.contains_key(&(10 * GRID_W + 10)));
        assert!(!gas.cells.contains_key(&(10 * GRID_W + 11)));
    }

    #[test]
    fn test_intake_to_output_full_pipeline() {
        let mut grid = test_grid(&[
            ((10, 10), 52, 1),
            ((11, 10), 52, 1),
            ((12, 10), 53, 1),
            ((13, 10), 49, 1),
            ((14, 10), 54, 1),
        ]);
        grid[(10 * GRID_W + 11) as usize] = make_block(52, 1, 1 << 3);
        let mut net = PipeNetwork::new();
        net.rebuild_with(&grid, is_liquid_pipe_component);
        assert_eq!(net.cells.len(), 5);
        let mut total = 0;
        for _ in 0..120 {
            total += net.tick(1.0 / 60.0, &grid, 5.0).len();
        }
        assert!(net.cells[&(10 * GRID_W + 12)].pressure > 0.5);
        assert!(total > 0);
    }

    #[test]
    fn test_intake_without_pump_no_pressure() {
        let mut grid = test_grid(&[((10, 10), 52, 1), ((11, 10), 52, 1), ((12, 10), 49, 1)]);
        grid[(10 * GRID_W + 11) as usize] = make_block(52, 1, 1 << 3);
        let mut net = PipeNetwork::new();
        net.rebuild_with(&grid, is_liquid_pipe_component);
        for _ in 0..60 {
            net.tick(1.0 / 60.0, &grid, 5.0);
        }
        let p = net.cells[&(10 * GRID_W + 12)].pressure;
        assert!(p < 0.01, "No pressure without pump, got {:.4}", p);
    }
}

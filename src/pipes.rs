//! Pipe network simulation — 1D pressure/flow system overlaid on the block grid.
//!
//! Components: Pipe (15), Pump (16), Tank (17), Valve (18), Outlet (19).
//! Simulation runs on CPU each frame. Gas composition (smoke, O2, CO2, temp)
//! flows through the pipe network from high to low pressure.

use crate::grid::{GRID_W, GRID_H, block_type_rs, block_flags_rs};

/// Per-pipe-block state.
#[derive(Clone, Debug)]
pub struct PipeCell {
    pub pressure: f32,         // internal pressure (0 = atmospheric, >0 = pressurized)
    pub gas: [f32; 4],         // [smoke, O2, CO2, temperature]
    pub volume: f32,           // effective volume (tank=10, pipe=1)
}

impl Default for PipeCell {
    fn default() -> Self {
        PipeCell {
            pressure: 0.0,
            gas: [0.0, 1.0, 0.0, 15.0], // atmospheric: no smoke, full O2, no CO2, 15°C
            volume: 1.0,
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

/// Check if a block type is part of the pipe network.
pub fn is_pipe_component(bt: u8) -> bool {
    bt >= 15 && bt <= 19
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

    /// Rebuild the network from the grid. Called when grid changes.
    pub fn rebuild(&mut self, grid: &[u32]) {
        self.cells.clear();
        for y in 0..GRID_H {
            for x in 0..GRID_W {
                let idx = y * GRID_W + x;
                let bt = block_type_rs(grid[idx as usize]);
                if is_pipe_component(bt) {
                    let mut cell = PipeCell::default();
                    // Tanks have large volume
                    if bt == 17 {
                        cell.volume = 10.0;
                    }
                    self.cells.insert(idx, cell);
                }
            }
        }
    }

    /// Simulate one tick of pressure equalization.
    /// `dt` is the frame delta time.
    /// `grid` is the block grid for reading valve states, pump directions, etc.
    /// Returns a list of (x, y, gas[4], velocity) for outlet injections.
    pub fn tick(&mut self, dt: f32, grid: &[u32]) -> Vec<(f32, f32, [f32; 4], f32)> {
        let mut outlet_injections = Vec::new();

        // Collect all pipe indices for iteration
        let indices: Vec<u32> = self.cells.keys().copied().collect();

        // Phase 1: compute flows between connected cells
        let mut pressure_delta: std::collections::HashMap<u32, f32> = std::collections::HashMap::new();
        let mut gas_delta: std::collections::HashMap<u32, [f32; 4]> = std::collections::HashMap::new();

        for &idx in &indices {
            let x = (idx % GRID_W) as i32;
            let y = (idx / GRID_W) as i32;
            let block = grid[idx as usize];
            let bt = block_type_rs(block);
            let flags = block_flags_rs(block);

            // Valves: check if open (bit2 = is_open)
            if bt == 18 && (flags & 4) == 0 {
                continue; // closed valve, no flow through
            }

            let cell = &self.cells[&idx];
            let conductance = if bt == 17 { 2.0 } else { 1.0 }; // tanks flow faster

            for &(dx, dy) in &[(0i32, -1i32), (0, 1), (1, 0), (-1, 0)] {
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
                    continue;
                }
                let nidx = ny as u32 * GRID_W + nx as u32;

                // Check if neighbor is also a pipe component and if valve is open
                if let Some(neighbor) = self.cells.get(&nidx) {
                    let nb = grid[nidx as usize];
                    let nbt = block_type_rs(nb);
                    let nflags = block_flags_rs(nb);

                    // Closed valve blocks flow
                    if nbt == 18 && (nflags & 4) == 0 {
                        continue;
                    }

                    // Flow proportional to pressure difference
                    let dp = cell.pressure - neighbor.pressure;
                    let flow = dp * conductance * dt * 2.0;

                    // Pressure transfer
                    *pressure_delta.entry(idx).or_insert(0.0) -= flow / cell.volume;
                    *pressure_delta.entry(nidx).or_insert(0.0) += flow / neighbor.volume;

                    // Gas composition transfer (proportional to flow)
                    if flow > 0.001 {
                        // Gas flows from this cell to neighbor
                        let gas_flow_frac = (flow / cell.volume).min(0.1);
                        let gd = gas_delta.entry(nidx).or_insert([0.0; 4]);
                        for i in 0..4 {
                            gd[i] += (cell.gas[i] - neighbor.gas[i]) * gas_flow_frac;
                        }
                    }
                }
            }

            // Phase 2: Pump — inject pressure + extract gas from environment
            if bt == 16 {
                let dir_bits = (flags >> 3) & 3; // bits 3-4 = direction
                let cell = self.cells.get(&idx).unwrap();
                // Pump adds pressure continuously
                let pump_rate = 0.5 * dt;
                *pressure_delta.entry(idx).or_insert(0.0) += pump_rate;
            }

            // Phase 3: Outlet — release gas into environment
            if bt == 19 {
                let cell = self.cells.get(&idx).unwrap();
                if cell.pressure > 0.1 {
                    let release_rate = cell.pressure.min(0.5) * dt;
                    *pressure_delta.entry(idx).or_insert(0.0) -= release_rate / cell.volume;

                    let fx = x as f32 + 0.5;
                    let fy = y as f32 + 0.5;
                    outlet_injections.push((fx, fy, cell.gas, cell.pressure));
                }
            }
        }

        // Apply deltas
        for (&idx, &dp) in &pressure_delta {
            if let Some(cell) = self.cells.get_mut(&idx) {
                cell.pressure = (cell.pressure + dp).max(0.0);
            }
        }
        for (&idx, gd) in &gas_delta {
            if let Some(cell) = self.cells.get_mut(&idx) {
                for i in 0..4 {
                    cell.gas[i] = (cell.gas[i] + gd[i]).max(0.0);
                }
                cell.gas[1] = cell.gas[1].min(1.0); // O2 cap
            }
        }

        // Pressure decay (slow leak to prevent infinite buildup)
        for cell in self.cells.values_mut() {
            cell.pressure *= 0.999;
        }

        outlet_injections
    }
}

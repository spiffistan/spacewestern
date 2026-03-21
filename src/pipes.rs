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

/// Check if a block type is part of the pipe network.
pub fn is_pipe_component(bt: u8) -> bool {
    (bt >= 15 && bt <= 20) || bt == 46
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

    /// Rebuild the network from the grid. Preserves existing cell state —
    /// only adds new cells and removes cells that are no longer pipe components.
    pub fn rebuild(&mut self, grid: &[u32]) {
        // Remove cells whose blocks are no longer pipe components
        self.cells.retain(|&idx, _| {
            is_pipe_component(block_type_rs(grid[idx as usize]))
        });
        // Add new cells for newly placed pipe components
        for y in 0..GRID_H {
            for x in 0..GRID_W {
                let idx = y * GRID_W + x;
                let bt = block_type_rs(grid[idx as usize]);
                if is_pipe_component(bt) && !self.cells.contains_key(&idx) {
                    let mut cell = PipeCell::default();
                    if bt == 17 { cell.volume = 10.0; }
                    // New cells start at atmospheric, will equalize with neighbors
                    self.cells.insert(idx, cell);
                }
            }
        }
    }

    /// Simulate one tick of pressure equalization.
    /// `dt` is the frame delta time.
    /// `grid` is the block grid for reading valve states, pump directions, etc.
    /// Returns a list of (x, y, gas[4], velocity) for outlet injections.
    pub fn tick(&mut self, dt: f32, grid: &[u32], pipe_width: f32) -> Vec<(f32, f32, [f32; 4], f32)> {
        let mut outlet_injections = Vec::new();

        // Collect all pipe indices for iteration
        let indices: Vec<u32> = self.cells.keys().copied().collect();

        // Phase 1: compute flows between connected cells
        let mut pressure_delta: std::collections::HashMap<u32, f32> = std::collections::HashMap::new();
        let mut gas_delta: std::collections::HashMap<u32, [f32; 4]> = std::collections::HashMap::new();
        let mut flow_accum: std::collections::HashMap<u32, (f32, f32)> = std::collections::HashMap::new();

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
            let conductance = if bt == 17 { pipe_width * 2.0 }
                else if bt == 46 {
                    // Restrictor: adjustable flow restriction (height lower nibble = level 1-10)
                    let level = ((block >> 8) & 0x0F) as f32;
                    pipe_width * (level / 10.0).max(0.05) * 0.3 // 0.3x max, down to near-zero
                }
                else { pipe_width };

            // Connection mask for directional pipes (height byte bits 4-7: N=0x10,E=0x20,S=0x40,W=0x80)
            let pipe_h = (block >> 8) & 0xFF;
            let conn_mask = pipe_h >> 4;
            let dir_masks: [(i32, i32, u32); 4] = [(0, -1, 0x1), (0, 1, 0x4), (1, 0, 0x2), (-1, 0, 0x8)]; // N,S,E,W

            for &(dx, dy, dmask) in &dir_masks {
                // If pipe has a connection mask, only flow in connected directions
                if (bt == 15 || bt == 46) && conn_mask != 0 && (conn_mask & dmask) == 0 { continue; }

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

                    // Flow uses the minimum conductance of both endpoints (bottleneck)
                    let neighbor_cond = if nbt == 17 { pipe_width * 2.0 }
                        else if nbt == 46 {
                            let nlevel = ((nb >> 8) & 0x0F) as f32;
                            pipe_width * (nlevel / 10.0).max(0.05) * 0.3
                        }
                        else { pipe_width };
                    let eff_cond = conductance.min(neighbor_cond);
                    let dp = cell.pressure - neighbor.pressure;
                    let flow = dp * eff_cond * dt * 2.0;

                    // Accumulate flow direction (outflow = positive dp → flow toward neighbor)
                    if dp > 0.001 {
                        let fa = flow_accum.entry(idx).or_insert((0.0, 0.0));
                        fa.0 += dx as f32 * flow;
                        fa.1 += dy as f32 * flow;
                    } else if dp < -0.001 {
                        // Inflow from neighbor
                        let fa = flow_accum.entry(idx).or_insert((0.0, 0.0));
                        fa.0 += dx as f32 * flow; // flow is negative here
                        fa.1 += dy as f32 * flow;
                    }

                    // Pressure transfer
                    *pressure_delta.entry(idx).or_insert(0.0) -= flow / cell.volume;
                    *pressure_delta.entry(nidx).or_insert(0.0) += flow / neighbor.volume;

                    // Gas composition transfer: unidirectional (flow direction only)
                    // Each pair is visited from both sides, so only transfer from
                    // high-pressure to low-pressure side to avoid double-counting.
                    let diffusion_rate = 1.0 * dt;
                    let flow_rate = (flow.abs() * 0.1).min(0.15);
                    let rate = (flow_rate + diffusion_rate).min(0.2);

                    if dp > 0.0 {
                        // This cell has higher pressure → send gas to neighbor
                        let gd = gas_delta.entry(nidx).or_insert([0.0; 4]);
                        for i in 0..4 {
                            gd[i] += (cell.gas[i] - neighbor.gas[i]) * rate;
                        }
                    }
                }
            }

            // Phase 2: Pump — inject pressure at adjustable rate
            if bt == 16 {
                let rate = self.cells[&idx].pump_rate;
                *pressure_delta.entry(idx).or_insert(0.0) += rate * dt;
            }

            // Inlet (type 20) — extract gas from adjacent room into pipe network
            if bt == 20 {
                let rate = self.cells[&idx].pump_rate;
                *pressure_delta.entry(idx).or_insert(0.0) += rate * 0.8 * dt;

                // Sample adjacent non-pipe blocks to determine what gas to extract
                // This approximates reading the fluid sim at the inlet position
                let mut env_gas = [0.0f32, 1.0, 0.0, 15.0]; // default: fresh air
                let mut found_room = false;
                for &(adx, ady) in &[(0i32, -1i32), (0, 1), (1, 0), (-1, 0)] {
                    let ax = x + adx;
                    let ay = y + ady;
                    if ax < 0 || ay < 0 || ax >= GRID_W as i32 || ay >= GRID_H as i32 { continue; }
                    let aidx = (ay as u32 * GRID_W + ax as u32) as usize;
                    let ab = grid[aidx];
                    let abt = block_type_rs(ab);
                    if is_pipe_component(abt) { continue; } // skip pipe neighbors

                    let adj_has_roof = (block_flags_rs(ab) & 2) != 0;

                    // Check for fire nearby (within 5 blocks of inlet)
                    let mut near_fire = false;
                    for fy in -5i32..=5 {
                        for fx in -5i32..=5 {
                            let cx = x + fx;
                            let cy = y + fy;
                            if cx < 0 || cy < 0 || cx >= GRID_W as i32 || cy >= GRID_H as i32 { continue; }
                            let cb = grid[(cy as u32 * GRID_W + cx as u32) as usize];
                            if block_type_rs(cb) == 6 { near_fire = true; break; }
                        }
                        if near_fire { break; }
                    }

                    if adj_has_roof && near_fire {
                        // Indoor room with fire: smoky, hot, depleted O2
                        env_gas = [0.6, 0.5, 0.25, 80.0];
                        found_room = true;
                    } else if adj_has_roof {
                        // Indoor room without fire: slightly stale
                        env_gas = [0.0, 0.9, 0.02, 18.0];
                        found_room = true;
                    }
                    if found_room { break; }
                }

                let gd = gas_delta.entry(idx).or_insert([0.0; 4]);
                for i in 0..4 {
                    gd[i] += (env_gas[i] - cell.gas[i]) * 3.0 * dt; // strong extraction
                }
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
                // Clamp all gas values to sane ranges
                cell.gas[0] = cell.gas[0].min(2.0);   // smoke
                cell.gas[1] = cell.gas[1].min(1.0);   // O2
                cell.gas[2] = cell.gas[2].min(2.0);   // CO2
                cell.gas[3] = cell.gas[3].clamp(-20.0, 500.0); // temp
            }
        }

        // Apply flow vectors (smoothed to avoid jitter)
        for (&idx, &(fx, fy)) in &flow_accum {
            if let Some(cell) = self.cells.get_mut(&idx) {
                cell.flow_x = cell.flow_x * 0.8 + fx * 0.2;
                cell.flow_y = cell.flow_y * 0.8 + fy * 0.2;
            }
        }
        // Decay flow on cells with no current flow
        for (&idx, cell) in self.cells.iter_mut() {
            if !flow_accum.contains_key(&idx) {
                cell.flow_x *= 0.9;
                cell.flow_y *= 0.9;
            }
        }

        // Pressure decay + cap
        for cell in self.cells.values_mut() {
            cell.pressure = (cell.pressure * 0.998).min(50.0); // cap at 50
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

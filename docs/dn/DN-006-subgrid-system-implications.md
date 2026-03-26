# DN-006: Sub-Grid System — Full Implications Audit

## Status: Proposed

## Context

DN-003 (sub-grid placement), DN-004 (thin walls), and DN-005 (windows/doors as wall features) introduce a 4×4 sub-grid within each tile. This document audits every game system that interacts with the tile grid and specifies what changes are needed.

## The Common Pattern: Edge-Based Queries

Multiple systems currently ask: "is this tile a wall?" With thin walls, the correct question becomes: "is the edge between tile A and tile B blocked?"

This edge-blocking query is used by pathfinding, vision, lighting, thermal, fluid, fire spread, and sound propagation. It should be a single shared function on both CPU and GPU:

### Rust (CPU)
```rust
/// Is movement/propagation blocked between adjacent tiles?
/// Checks both tiles: if either has a wall on the shared edge, it's blocked.
/// Open doors on the shared edge make it passable.
fn edge_blocked(
    grid: &[u32],
    features: &[u8],  // wall_features buffer
    ax: i32, ay: i32,
    bx: i32, by: i32,
) -> bool
```

### WGSL (GPU)
```wgsl
fn edge_blocked(ax: i32, ay: i32, bx: i32, by: i32) -> bool
```

Used by: pathfinding (pleb.rs), vision (fog.rs), fire spread (fire.rs), and all GPU shaders (lightmap, thermal, fluid, sound).

The logic:
1. Determine crossing direction (N/E/S/W) from the delta between A and B
2. Check if tile A has a wall on that exit edge (from wall_edges bits + thickness)
3. Check if tile B has a wall on the entry edge
4. If either has a wall: check for an open door at that edge (from features buffer)
5. If a window exists at that edge: depends on the system (vision: pass, fire: block, sound: attenuate)

---

## System-by-System Analysis

### 1. Pathfinding (pleb.rs)

**Current:** `is_walk(x, y)` — binary per-tile. Checks block type, height, door state. A* iterates 8 neighbors and checks if each is walkable.

**Change needed:** The neighbor check becomes:
```
is_walk(next.x, next.y) && !edge_blocked(current, next)
```

A tile with a thin wall on one edge is walkable (open sub-cells exist) but crossing its walled edge is blocked.

**Diagonal movement:** Currently requires both cardinal neighbors to be walkable (no corner-cutting). With thin walls, diagonal movement also needs to check that the diagonal doesn't cross a wall edge. A diagonal move from (0,0) to (1,1) crosses the E edge of (0,0) AND the S edge of (0,1) — both must be unblocked.

**Difficulty:** Medium. The A* loop already has a per-neighbor filter. Adding `edge_blocked` is one extra call per neighbor. The diagonal corner-cutting check (line 550-554) needs edge-awareness too.

**File:** `src/pleb.rs` — `astar_path_full()` (line 460)

---

### 2. Fog of War / Vision (fog.rs)

**Current:** `blocks_vision(grid, x, y)` returns bool. 8-octant recursive shadowcasting treats each tile as opaque or transparent. Walls block, glass/trees don't.

**Change needed:** The shadowcasting algorithm processes cells as it scans outward. When it evaluates a cell, it needs to know if the *edge crossed to reach that cell* is blocked, not just whether the cell itself is opaque.

Two approaches:

**A) Edge check at cell transition:** In `cast_light()`, when checking if cell (mx, my) blocks vision, also check if the edge from the previous cell to (mx, my) is blocked. This modifies the `blocks_vision` call at lines 106 and 114 of fog.rs.

**B) Partial transparency:** Treat thin-walled tiles as partially opaque. A wall with a 2-wide window on a 4-cell edge is 50% transparent. Shadowcasting with partial transparency is more complex but more accurate.

**Recommendation:** Approach A for now. Edge-check is simpler and gives correct binary results (either you can see through or you can't).

**Windows:** A wall edge with a window sub-cell should transmit vision through the window portion. Simplest rule: if any sub-cell on the crossed edge is a window, vision passes. This gives windows a clear gameplay purpose — you can see through them.

**Difficulty:** High. Shadowcasting algorithms assume isotropic per-cell opacity. Making them edge-aware requires modifying the core loop where cells are evaluated.

**File:** `src/fog.rs` — `cast_light()` (line 52), `blocks_vision()` (line 20)

---

### 3. Interior Lighting / Sunbeams (raytrace.wgsl)

**Current:** `trace_interior_sun_ray()` traces from a pixel toward the sun. Checks `is_glass()` at each tile — glass transmits light with tint, walls block.

**Change needed:** At each tile along the ray, check the wall features buffer:
1. If the tile is a wall with no window → block light
2. If the tile is a wall with a window → check if the ray crosses through a window sub-cell or a solid sub-cell
3. Window sub-cells transmit light with glass tint; solid sub-cells block

The sub-cell check: the ray enters the tile at a specific `(fx, fy)` position. Map that to sub-cell coordinates. Read the features byte to see if that sub-cell is window or wall.

**Also affects:** `compute_proximity_glow()` traces visibility between light sources and pixels. This uses `trace_glow_visibility()` which checks walls. Thin walls should only block glow across walled edges.

**Difficulty:** Medium. The ray-marching loop already iterates tiles. Adding a features buffer lookup and sub-cell check per wall tile is a small addition.

**Files:** `src/shaders/raytrace.wgsl` — `trace_interior_sun_ray()`, `trace_glow_visibility()`

---

### 4. Lightmap Propagation (lightmap.wgsl, lightmap_propagate.wgsl)

**Current:** Light seeds are placed at light sources. Propagation spreads outward, attenuated by distance. `is_wall()` blocks propagation — walls are opaque barriers.

**Change needed:** When propagating light from tile A to neighbor B, check `edge_blocked(A, B)`:
- If the edge has a solid wall → block light completely
- If the edge has a window → partial transmission (e.g., 60% of glass's `light_transmission` value × window width fraction)
- If no wall on this edge → propagate normally

The propagation shader samples 4 cardinal neighbors. Each neighbor check becomes:
```wgsl
if !edge_blocked(x, y, nx, ny) {
    neighbor_light += textureLoad(lightmap, vec2(nx, ny), 0).r;
    count += 1.0;
}
```

**Difficulty:** Medium. The propagation loop structure is straightforward. The features buffer needs to be bound as a new `@binding`.

**Files:** `src/shaders/lightmap.wgsl`, `src/shaders/lightmap_propagate.wgsl`

---

### 5. Thermal Simulation (thermal.wgsl)

**Current:** Heat conducts between all adjacent tiles based on material conductivity. No directional blocking — heat flows freely through walls to neighbors.

**Change needed:** Heat conduction across a walled edge should be reduced or blocked:
- Solid wall edge → minimal conduction (walls are thermal insulators, not perfect barriers — some heat leaks through)
- Window edge → higher conduction (glass is a thermal bridge)
- Open door edge → full conduction (air exchange)
- No wall edge → normal conduction

The neighbor loop (lines 102-116 of thermal.wgsl) becomes:
```wgsl
for each neighbor:
    if edge_blocked(current, neighbor) {
        // Wall: very low conduction (insulation)
        let leak = 0.02 * mat.conductivity;
        neighbor_heat += block_temps[nidx] * leak;
    } else {
        neighbor_heat += block_temps[nidx];
    }
    neighbor_count += 1.0;
```

**Window thermal bridge:** A wall with a window conducts more heat across that edge than a solid wall. Windows are the weak point in insulation — this is physically correct and gameplay-relevant (insulated walls vs glass windows matter).

**Difficulty:** Medium. Same loop structure, add edge check per neighbor.

**File:** `src/shaders/thermal.wgsl` — `main_thermal()` (line 102)

---

### 6. Fluid Simulation (fluid.rs, fluid shaders)

**Current:** `build_obstacle_field()` creates a 256×256 u8 texture. Wall tiles are 255 (obstacle), open tiles are 0. The fluid solver treats obstacles as hard boundaries.

**Change needed:** The obstacle field becomes edge-aware. A thin wall tile is not a full obstacle — it's partially open. Two approaches:

**A) Directional obstacle field:** Instead of one u8 per tile (obstacle/not), use 4 bits per tile — one bit per edge (N/E/S/W blocked). The fluid solver checks per-edge blocking when sampling neighbors. This changes the obstacle texture from R8 to R8 with packed bits.

**B) Sub-cell resolution obstacle field:** 4× resolution obstacle texture (1024×1024). Each sub-cell is obstacle or open. The fluid solver operates at the same resolution or samples the higher-res texture. This is expensive — 4× grid means 16× fluid sim cost.

**Recommendation:** Approach A. The fluid solver already samples 4 cardinal neighbors per cell. Checking an edge bit per direction is cheap.

**Difficulty:** Medium-High. The fluid shaders (advection, pressure, divergence) all sample neighbors. Each needs the directional obstacle check. The obstacle field generation changes from per-tile to per-edge.

**Files:** `src/fluid.rs` — `build_obstacle_field()`, all fluid shaders

---

### 7. Sound Propagation (sound.wgsl)

**Current:** `is_wall(x, y)` returns bool. Wall cells have zero pressure/velocity (hard boundary). Glass attenuates by 0.7×. The 5-point Laplacian samples 4 cardinal neighbors with `read_pressure()`.

**Change needed:** `read_pressure()` checks `is_wall()` for the target tile. With thin walls, it should check `edge_blocked(current, target)`:
```wgsl
fn read_pressure(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> f32 {
    if edge_blocked(from_x, from_y, to_x, to_y) {
        // Check for window → partial transmission
        if has_window_on_edge(from_x, from_y, to_x, to_y) {
            return textureLoad(sound_in, vec2(to_x, to_y), 0).r * 0.5; // window attenuates
        }
        return 0.0; // solid wall reflects
    }
    return textureLoad(sound_in, vec2(to_x, to_y), 0).r;
}
```

Sound through windows should be attenuated but not blocked (you can hear through glass, just muffled). Sound through walls should be nearly zero. Open doors transmit fully.

**Difficulty:** Medium. Same pattern as thermal — edge check per neighbor in the Laplacian.

**File:** `src/shaders/sound.wgsl` — `is_wall()`, `read_pressure()`, `main_sound()`

---

### 8. Fire Spread (fire.rs)

**Current:** Checks 4 cardinal neighbors per burning block. No wall-awareness — fire can currently spread through walls if the neighbor is flammable.

**Change needed:** Fire should not spread across a walled edge (firewall). Before checking if a neighbor is flammable, check `edge_blocked(burning_tile, neighbor)`.

Windows: fire should not spread through glass. But extreme heat could eventually break a window (future feature — not part of this proposal).

Open doors: fire SHOULD spread through open doors. This creates gameplay: close doors to contain fires.

**Difficulty:** Low. One edge_blocked check per neighbor in the spread loop.

**File:** `src/fire.rs` — `tick_fire()` spread section (line 150)

---

### 9. Shadow Map (shadow_map.wgsl)

**Current:** The shadow map traces rays from the sun to determine which tiles are in shadow. Walls cast shadows.

**Change needed:** Thin walls cast thinner shadows. A 1-sub-cell-thick wall on the north edge of a tile only shadows the northern portion of the tile, not the full tile. The shadow ray needs sub-cell awareness when passing through thin-walled tiles.

**Difficulty:** Medium. The ray trace needs to check wall thickness to determine if the ray passes through wall or open sub-cells.

**File:** `src/shaders/shadow_map.wgsl`

---

### 10. Needs / Environment Sampling (needs.rs)

**Current:** `sample_environment()` checks if a pleb is indoors (roof flag), air quality (from fluid dye), temperature (from block_temps). Doesn't directly check walls.

**Impact:** Minimal. Indoors detection uses roof flags, not wall detection. Temperature comes from the thermal system which will be updated separately.

**One consideration:** A pleb next to a window should feel wind/cold from outside. The thermal system's edge-aware conduction handles this naturally — tiles near windows will be cooler than tiles far from windows.

**Difficulty:** None (handled by thermal system changes).

---

### 11. Construction / Placement (placement.rs)

**Current:** Walls placed as full-tile blocks. Windows/doors replace wall blocks.

**Change needed:**
- **Thin wall placement:** Player specifies which edge (or auto-detected from drag direction). Placement sets wall_edges bits + thickness on the tile.
- **Window/door placement:** Player clicks a wall tile. System modifies the features buffer instead of replacing the block type. Wall material is preserved.
- **Furniture placement:** Validation checks sub-cell overlap with wall sub-cells on the same tile.
- **Blueprint system:** Needs "modify existing block" variant for windows/doors, not just "place new block."

**Difficulty:** Medium. Placement UI changes are significant but well-scoped.

**Files:** `src/placement.rs`, `src/types.rs` (Blueprint)

---

### 12. GPU Resource Management (gpu_init.rs, main.rs)

**New GPU resources needed:**

| Resource | Format | Size | Bound to |
|----------|--------|------|----------|
| `wall_features` buffer | `u8` per tile (or `u32` for alignment) | 64-256 KB | raytrace, lightmap, lightmap_propagate, thermal, sound, fluid shaders |
| Updated obstacle field | Directional bits | Same size | fluid shaders |

The features buffer needs a new `@binding` slot in every shader that uses it. Currently shaders have 8-15 bindings each. Adding one more is within limits.

Upload frequency: only when features change (wall/window/door placed or removed). Same pattern as grid_dirty flag.

**Difficulty:** Low-Medium. Boilerplate-heavy but routine — same pattern as existing buffers.

**File:** `src/gpu_init.rs` — bind group layouts, buffer creation, binding slots

---

### 13. Save/Load

**New data to persist:**
- `wall_features: Vec<u8>` — parallel to `grid_data`
- If thin walls use flag bits in grid_data, those are already saved

**Migration:** Existing saves have no features buffer. Initialize all to 0 (no features). Existing BT_GLASS blocks could be auto-migrated to wall-with-window on load, or left as-is for backward compatibility.

**Difficulty:** Low.

---

### 14. Bullet / Projectile Physics (physics.rs)

**Current:** DDA ray trace for bullets. Checks block height to determine if a bullet hits a wall.

**Change needed:** A bullet traveling through a tile with a thin wall should only be blocked if the bullet's ray crosses the wall sub-cells. A bullet passing through the open sub-cells of a thin-walled tile should continue.

This is the same sub-cell check as sunbeam tracing: determine which sub-cell the ray passes through, check if it's wall or open.

**Difficulty:** Medium. DDA already traces per-tile. Adding sub-cell check per wall tile.

**File:** `src/physics.rs`

---

## Implementation Phases

### Phase 1: Data + Rendering (Visual Only)
**Files:** grid.rs, blocks.toml, raytrace.wgsl, gpu_init.rs, placement.rs
- Wall features buffer (u8 per tile)
- Thin wall rendering at variable thickness
- Window/door rendering as wall features
- Placement UI
- **No gameplay changes** — pathfinding, vision, etc. still use old per-tile logic

### Phase 2: Pathfinding + Doors
**Files:** pleb.rs, placement.rs
- `edge_blocked()` function (Rust)
- A* uses edge blocking
- Door toggle modifies features buffer
- Diagonal corner-cutting respects wall edges

### Phase 3: Vision + Shadow
**Files:** fog.rs, shadow_map.wgsl
- Edge-aware `blocks_vision()`
- Shadowcasting uses edge checks at cell transitions
- Windows transmit vision

### Phase 4: Light + Sound
**Files:** lightmap.wgsl, lightmap_propagate.wgsl, sound.wgsl, raytrace.wgsl
- `edge_blocked()` in WGSL (shared utility)
- Lightmap propagation edge-aware
- Sunbeam tracing sub-cell-aware
- Sound propagation edge-aware with window attenuation

### Phase 5: Thermal + Fluid
**Files:** thermal.wgsl, fluid.rs, fluid shaders
- Thermal conduction edge-aware (with window thermal bridge)
- Directional fluid obstacle field
- Fluid solver edge-aware

### Phase 6: Fire + Physics + Polish
**Files:** fire.rs, physics.rs
- Fire spread checks edge blocking
- Bullet DDA sub-cell-aware
- Blueprint system for wall modifications

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Performance: edge_blocked in hot loops | Medium | Simple bit operations — 2 buffer reads + bit mask per call. Profile after Phase 2. |
| Shader complexity: new binding in all shaders | Low | One buffer added, same pattern as existing bindings. |
| Backward compatibility: existing saves | Low | Features buffer initializes to 0 = no change. |
| Visual artifacts: thin wall shadow/light seams | Medium | May need sub-pixel smoothing at wall edges. Address in Phase 3-4. |
| Diagonal pathfinding edge cases | Medium | Thorough testing of corner interactions — L-shaped walls, doorways at corners. |
| Fluid sim instability with directional obstacles | Medium | Test with simple cases first. Directional obstacles are well-studied in CFD. |

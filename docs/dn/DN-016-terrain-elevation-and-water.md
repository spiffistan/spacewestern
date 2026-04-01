# DN-016: Sub-Tile Terrain Elevation, Digging, and Water Flow

## Overview

Replace the per-tile terrain height with a continuous 1024x1024 elevation heightmap (4x4 sub-tiles per game tile). Digging modifies elevation with soft brushes, creating natural slopes and channels. Water flows through a GPU pipe model at 512x512 resolution, pooling in depressions and creating functional moats, irrigation, and drainage.

## Architecture

### Three layers

| Layer | Resolution | Format | Purpose |
|-------|-----------|--------|---------|
| Elevation | 1024x1024 | R32Float | Terrain height at sub-tile precision |
| Water depth | 512x512 (ping-pong pair) | R32Float | Water above terrain at each cell |
| Water flux | 512x512 | Rgba32Float | Outflow rate in 4 cardinal directions |

Block grid (256x256) remains tile-aligned for buildings, walls, items. Elevation is the continuous landscape underneath.

### Elevation heightmap

Generated during world gen from the existing elevation array (256x256), upscaled to 1024x1024 with bicubic interpolation + fractal noise for natural sub-tile variation. Each sub-cell stores a height value in world units (0.0 = sea level baseline).

The heightmap is stored on GPU as a texture and on CPU as a `Vec<f32>` for digging operations and pathfinding queries.

## Digging

### Player interaction

1. Select "Dig" from build menu (category: Terrain)
2. Choose preset: Shallow (-0.3), Trench (-0.8), Moat (-1.5), or Custom depth slider
3. Drag a line or rectangle on the map
4. Zone appears as a marked overlay (brown dashed outline with depth label)
5. Plebs with digging work priority go dig

### Dig zone data

```rust
pub struct DigZone {
    pub tiles: Vec<(i32, i32)>,   // tile coords covered
    pub target_depth: f32,        // how far below current surface
    pub cross_profile: CrossProfile, // shape of the cross-section
    pub priority: u8,
}

pub enum CrossProfile {
    Flat,      // uniform depth across width (irrigation channel)
    VShape,    // deeper in center, sloped sides (natural ditch)
    UShape,    // flat bottom, steep sides (trench/moat)
}
```

### Cross-section profiles

For a dig zone N tiles wide, the target depth varies across the width:

**Flat**: every sub-cell gets the same target depth. Good for foundations, irrigation.

**V-Shape** (default for ditches): depth follows a smooth curve peaking at center:
```
depth_at(x) = target_depth * (1.0 - (2*x/width - 1)^2)
```
A 3-tile-wide V-ditch at target -0.8: edges are at -0.0, center is at -0.8. Natural drainage shape.

**U-Shape** (for moats/trenches): flat bottom with steep sides:
```
depth_at(x) = target_depth * smoothstep(0, 0.3, min(x/width, 1-x/width))
```
A 3-tile-wide U-moat at target -1.5: outer 0.3 width slopes down, center 0.4 is flat at full depth.

### Pleb dig execution

1. Pleb walks to nearest unfinished edge of dig zone
2. Plays shovel swing animation (swing_progress, ~1.5s per stroke)
3. Each stroke applies a soft circular brush to the elevation heightmap:

```rust
const DIG_BRUSH_RADIUS: f32 = 1.8;  // sub-cells (~0.45 tiles)
const DIG_DEPTH_PER_STROKE: f32 = 0.04; // with shovel
const STROKES_PER_REPOSITION: u32 = 4;

// Per sub-cell within brush radius:
let dist = distance(sub_cell, pleb_sub_pos);
let falloff = smoothstep(DIG_BRUSH_RADIUS, 0.0, dist);

// Profile-aware: don't dig below the target depth at this position
let target = zone.depth_at(sub_cell);
let current = elevation[sub_cell] - original_surface[sub_cell];
let remaining = (target - current).min(0.0).abs();
let dig = (DIG_DEPTH_PER_STROKE * falloff).min(remaining);

elevation[sub_cell] -= dig;
dirt_produced += dig;
```

4. Every STROKES_PER_REPOSITION strokes, pleb shuffles 0.5 tiles along the zone
5. When the near edge is done, pleb steps into the depression and works deeper
6. Dirt materializes as ground items near the pleb (~1 dirt item per 0.15 volume removed)

### Digging speed modifiers

```rust
pub const DIG_SPEED_SHOVEL: f32 = 1.0;
pub const DIG_SPEED_PICK: f32 = 0.7;     // slower but handles rock
pub const DIG_SPEED_HANDS: f32 = 0.25;
pub const DIG_SPEED_CLAY: f32 = 0.6;
pub const DIG_SPEED_ROCK: f32 = 0.3;     // requires pick
pub const DIG_SPEED_WET: f32 = 0.5;      // below water table
pub const DIG_DEPTH_PENALTY: f32 = 0.1;  // -10% per 0.5 depth
pub const DIG_SKILL_BONUS: f32 = 0.08;   // +8% per construction skill level
```

## Dirt as Resource

Each elevation unit removed generates dirt. Dirt is a physical resource:

| Use | Mechanism |
|-----|-----------|
| **Berm** | "Raise terrain" zone. Pleb dumps dirt, elevation rises with soft brush |
| **Mud wall** | Crafting recipe: dirt + water = mud brick |
| **Fill** | Reverse dig: fill zone raises elevation back up |
| **Path** | Flatten + compact terrain for roads |
| **Dump** | Designated dump zone, no use (stockpiled for later) |

If nobody hauls dirt, it piles up next to the dig site as ground items.

## Water Flow: GPU Pipe Model

### Simulation (512x512)

Two compute passes per frame in `water.wgsl`:

**Pass 1 — Flux update:**
```wgsl
// Read elevation from 1024x1024 (sample at 2x2 center for this water cell)
let my_elev = sample_elevation(pos * 2 + 1);  // center of 2x2 patch
let my_surface = my_elev + water_depth[pos];

// For each cardinal neighbor:
let n_elev = sample_elevation(neighbor_pos * 2 + 1);
let n_surface = n_elev + water_depth[neighbor_pos];

let delta_h = my_surface - n_surface;
flux[dir] = max(0.0, flux[dir] + GRAVITY * delta_h * dt);

// Clamp total outflow to available water (volume conservation)
let total_out = flux.r + flux.g + flux.b + flux.a;
if total_out > water_depth[pos] / dt {
    flux *= water_depth[pos] / (total_out * dt);
}

// Walls block flux (check obstacle texture)
if wall_blocks(pos, dir) { flux[dir] = 0.0; }
```

**Pass 2 — Depth update:**
```wgsl
// Inflow from neighbors' outflow toward us
let inflow = neighbor_flux_toward_me(pos);
let outflow = flux.r + flux.g + flux.b + flux.a;

water_depth[pos] += (inflow - outflow) * dt;
water_depth[pos] = max(0.0, water_depth[pos]);

// Rain input
water_depth[pos] += rain_rate * dt;

// Evaporation (temperature-dependent)
water_depth[pos] -= evaporation_rate * temperature * dt;
water_depth[pos] = max(0.0, water_depth[pos]);
```

### Constants

```rust
pub const WATER_SIM_W: u32 = 512;
pub const WATER_SIM_H: u32 = 512;
pub const ELEVATION_W: u32 = 1024;
pub const ELEVATION_H: u32 = 1024;
pub const WATER_GRAVITY: f32 = 9.81;
pub const WATER_DAMPING: f32 = 0.995;      // slight friction
pub const RAIN_RATE: f32 = 0.001;           // per second during rain
pub const EVAPORATION_RATE: f32 = 0.0001;   // per second per degree above 20C
pub const GROUNDWATER_SEEP: f32 = 0.0005;   // per second when below water table
```

### Groundwater seep

When the elevation at a water cell drops below the local water table value, water seeps in:
```wgsl
let water_table = sample_water_table(pos);
if elevation < water_table && water_depth[pos] < water_table - elevation {
    water_depth[pos] += GROUNDWATER_SEEP * dt;
}
```

This makes ditches near rivers/wet areas fill naturally, while ditches on dry hilltops stay dry unless rain fills them.

## Rendering

### Terrain elevation

Replace per-tile height reads with bilinear-sampled elevation heightmap:

```wgsl
let elev = bilinear_sample(elevation_tex, world_x * 4.0, world_y * 4.0);  // 1024x1024
```

Terrain color is still from the tile grid (block type determines material). But the visual height/slope comes from the continuous heightmap. This means:
- Ditch edges are smooth gradients, not stair-steps
- Berms rise gently
- Natural terrain has micro-variation

### Water surface

```wgsl
let water_h = bilinear_sample(water_tex, world_x * 2.0, world_y * 2.0);  // 512x512
let surface = elev + water_h;

if water_h > 0.01 {
    let depth = water_h;

    // Flow velocity for ripple direction
    let flux = bilinear_sample(flux_tex, world_x * 2.0, world_y * 2.0);
    let vel = vec2(flux.g - flux.a, flux.r - flux.b);  // east-west, north-south

    // Flow-aligned ripples
    let speed = length(vel);
    let flow_dir = select(vec2(0.0, 1.0), normalize(vel), speed > 0.001);
    let along = dot(vec2(world_x, world_y), flow_dir);
    let ripple = sin(along * 15.0 - time * speed * 3.0) * 0.02 * min(speed * 5.0, 1.0);

    // Static ripples for still water
    let still_ripple = sin(world_x * 11.0 + time * 1.3) * sin(world_y * 13.0 + time * 0.9) * 0.01;
    let final_ripple = mix(still_ripple, ripple, min(speed * 10.0, 1.0));

    // Depth-dependent color
    let shallow_col = vec3(0.15, 0.38, 0.55);
    let deep_col = vec3(0.06, 0.18, 0.40);
    var water_color = mix(shallow_col, deep_col, clamp(depth * 2.0, 0.0, 1.0));

    // Shore foam where depth approaches zero
    let foam = smoothstep(0.05, 0.0, depth) * 0.5;
    water_color = mix(water_color, vec3(0.8, 0.85, 0.9), foam);

    // Wet terrain above waterline (darkened, moisture)
    // ... (applied to adjacent non-water pixels)

    // Transparency: shallow water shows terrain through it
    let alpha = smoothstep(0.0, 0.4, depth);
    color = mix(color, water_color + final_ripple, clamp(alpha, 0.3, 0.95));
}
```

### Terrain slope shading

The elevation gradient creates natural slope shading:
```wgsl
let dx = bilinear_sample(elevation_tex, (world_x + 0.05) * 4.0, world_y * 4.0)
       - bilinear_sample(elevation_tex, (world_x - 0.05) * 4.0, world_y * 4.0);
let dy = bilinear_sample(elevation_tex, world_x * 4.0, (world_y + 0.05) * 4.0)
       - bilinear_sample(elevation_tex, world_x * 4.0, (world_y - 0.05) * 4.0);
let slope_shade = 1.0 - length(vec2(dx, dy)) * 2.0;  // darker on steep slopes
color *= clamp(slope_shade, 0.6, 1.0);
```

This makes ditch walls visibly darker (steep slope = shadow), berms have a lit side and shadow side, and natural terrain has subtle slope lighting.

## Defense Interaction

### Movement cost

Pathfinding samples average elevation at tile center. Depth below surface = movement penalty:

| Depth | Dry penalty | Wet penalty | Notes |
|-------|------------|-------------|-------|
| 0 to -0.2 | 1.2x | 1.5x | Slight depression |
| -0.2 to -0.5 | 1.5x | 2.5x | Ankle/knee deep |
| -0.5 to -1.0 | 2.0x | 4.0x + splash sound | Waist deep, alerts defenders |
| -1.0+ | 3.0x | Impassable | Deep moat, needs bridge |

### Trench as cover

A pleb standing in a trench has reduced Z-height based on the trench depth at their position:
```rust
let trench_depth = (original_elevation - current_elevation).max(0.0);
let effective_z = pleb.z_height() - trench_depth;
```

A pleb in a -0.8 trench: effective Z = 1.0 - 0.8 = 0.2. Nearly invisible to flat-trajectory bullets. Combined with the peek-fire system: the trench rim is the "wall" they peek over.

### Water as barrier

Enemies won't pathfind through deep water. Combined with walls at the only bridge → funneling. The moat forces enemies to approach from predictable directions where defenders have overlapping fire.

### Sound alert

Entities wading through water > 0.2 depth emit splash sounds through the sound sim. Faster movement = louder splash. Defenders hear enemies trying to cross the moat at night.

## Building interaction

When placing a building on sloped terrain:
```rust
// Sample elevation at all sub-cells under the building footprint
// Average them → target flat elevation
// Stamp all sub-cells to that average
// Excess dirt above average → dirt items (auto-leveling excavation)
// Deficit below average → requires dirt placement (filling)
```

This means buildings auto-level their foundation, and the player can see terrain being flattened during construction.

Walls block water flux at tile boundaries (same as existing wall-blocked fluid flow in the obstacle texture).

## Implementation order

1. **Elevation heightmap** — create 1024x1024 texture, generate from existing elevation, upload to GPU, render with bilinear sampling (visual only first)
2. **Slope shading** — derivative-based shading in raytrace shader
3. **Dig zones** — UI for marking dig areas, zone data structure
4. **CPU dig execution** — brush-based elevation modification, dirt generation
5. **GPU upload** — write modified elevation sub-region to GPU after each dig stroke
6. **Water textures** — create 512x512 water depth + flux textures
7. **Water compute shader** — pipe model, two passes
8. **Water rendering** — bilinear water depth in raytrace, flow-aligned ripples
9. **Groundwater seep** — connect to existing water table
10. **Rain/evaporation** — weather system drives water input/output
11. **Defense integration** — pathfinding cost, Z-height in trenches, splash sounds

## Files changed

| File | Changes |
|------|---------|
| `src/terrain.rs` | NEW — elevation heightmap, dig zone, brush ops, dirt resource |
| `src/shaders/water.wgsl` | NEW — GPU pipe model compute shader |
| `src/grid.rs` | Elevation generation at 1024x1024 from existing world gen |
| `src/gpu_init.rs` | Create elevation + water textures, pipeline, bind groups |
| `src/main.rs` | Store elevation CPU-side, dispatch water compute, upload dig changes |
| `src/shaders/raytrace.wgsl` | Bilinear elevation sampling, water rendering, slope shading |
| `src/simulation.rs` | Pleb dig task execution, dirt production, trench Z-height |
| `src/pleb.rs` | Dig activity, effective Z from trench depth |
| `src/ui.rs` | Dig zone tool in build menu, depth preset selector |
| `src/placement.rs` | Building auto-leveling on placement |
| `src/zones.rs` | DigZone struct, work task generation |

# Rayworld — Project Context (v39)

## What This Is

A colony survival simulation rendered entirely via GPU compute shader raytracing, inspired by Rimworld. Written in Rust with wgpu, runs native and WASM/WebGPU. The core differentiator is a real-time Navier-Stokes fluid simulation that integrates with the game world — smoke flows through doors, fire consumes O2 in sealed rooms, fans force airflow through walls.

## Architecture

**Single-crate, single-file Rust project** with WGSL compute shaders. All game logic in `src/main.rs` (~3750 lines). Shaders are separate `.wgsl` files in `src/`.

| File | Lines | Purpose |
|------|-------|---------|
| `main.rs` | 3749 | All game logic, GPU setup, render loop, UI, input, A* |
| `raytrace.wgsl` | 1744 | Per-pixel raytracing: materials, shadows, lighting, smoke overlay, pleb rendering |
| `fluid.wgsl` | 241 | NS velocity passes: curl, vorticity, divergence, gradient, advection, splat, fan/fire sources |
| `fluid_dye.wgsl` | 243 | Dye/gas advection at 512x512: smoke, O2, CO2, temperature channel, diffusion, accumulation |
| `fluid_pressure.wgsl` | 72 | Jacobi pressure solver with Neumann BCs |
| `lightmap.wgsl` | 122 | Light source seeding (fire flicker, electric lights, lamps) |
| `lightmap_propagate.wgsl` | 191 | Iterative flood-fill light propagation with wall/glass occlusion |
| `blit.wgsl` | 35 | Fullscreen triangle blit with bilinear upscale |

## Key Data Structures

### Block Grid (256x256, u32 per cell)
```
[type:8 | height:8 | flags:8 | roof_height:8]
Types: 0=air, 1=stone, 2=dirt, 3=water, 4=wall, 5=glass, 6=fireplace,
       7=electric_light, 8=tree, 9=bench, 10=standing_lamp, 11=table_lamp,
       12=fan, 13=compost
Flags: bit0=door, bit1=roof, bit2=open, bits3-4=segment/direction, bits5-6=rotation
```

### CameraUniform (Rust struct shared with all WGSL shaders)
44 f32 fields = 176 bytes. Contains: camera position/zoom, screen dimensions, grid size, time, sun parameters (precomputed), lighting tuning, lightmap viewport, fluid overlay mode, pleb position/angle/lights, temporal reprojection state. **Must match exactly** across `main.rs` and all 4 WGSL shader Camera structs.

### FluidParams (separate uniform for fluid shaders)
20 f32 fields = 80 bytes. Contains: sim/dye dimensions, dt, dissipation, vorticity, splat parameters, wind vector, smoke rate, fan speed.

### Dye Texture Channels (RGBA16Float, 512x512)
- **R**: smoke density (0-2, visual haze)
- **G**: O2 level (0-1, atmospheric=1.0, fire consumes)
- **B**: CO2 level (0-1.5, fire/compost produces)
- **A**: reserved for temperature (future)

## GPU Pipeline Per Frame

```
1. Lightmap (every 2 frames): 2 seed passes + 26 propagation passes (viewport-culled)
2. Fluid sim (every frame):
   - Curl → Vorticity → Splat → Divergence
   - Pressure clear → 35 Jacobi iterations (Neumann BCs, ping-pong)
   - Gradient subtract → Advect velocity (+ fire/fan/wind sources)
   - Advect dye 512x512 (smoke/O2/CO2, diffusion, accumulation, fire injection)
3. Raytrace: per-pixel compute shader (temporal reprojection early-out, shadow rays,
   proximity glow with lightmap gate, directional bleed, material rendering, pleb,
   border fog, fluid overlay)
4. Blit: fullscreen triangle with bilinear upscale
5. Egui: UI overlay (controls, build menu, overlays, debug, pleb menu)
```

## Fluid Simulation

Full Navier-Stokes (Stable Fluids / Stam 1999) at 256x256 with:
- Vorticity confinement (curl strength 35)
- Neumann BCs at walls (pressure builds up in sealed rooms)
- 35 Jacobi pressure iterations with 0.6 temporal damping
- Fire blocks: consume O2, produce CO2 + smoke, inject velocity
- Compost blocks: produce CO2 continuously (anaerobic)
- Fan blocks: force-set velocity (one-way valve, can't be overridden by pressure)
- Global wind: outdoor cells receive wind force, adjustable via UI
- Smoke diffusion (10%) + accumulation for room filling
- Edge dissipation + windward O2 injection
- Obstacle-aware dye advection (walls block smoke, bilinear sampling respects walls)

## Lighting System

- **Lightmap**: 512x512 (2x grid resolution), flood-fill propagation, 26 iterations
- **Proximity glow**: per-pixel 13x13 scan for nearby lights (gated by lightmap intensity)
- **Directional bleed**: light pools projected through windows/doors
- **Shadow rays**: per-pixel trace toward sun with glass tinting and tree dappling
- **Precomputed sun**: all trig on CPU, passed via CameraUniform
- **Day/night cycle**: 60-second full cycle, dawn/dusk color transitions

## Performance Optimizations

- Render at configurable resolution (0.15-1.0 of window, bilinear upscale)
- Conditional proximity glow (lightmap gate skips ~90% of 13x13 scans)
- Toggleable glow/bleed (UI buttons to disable expensive per-pixel features)
- Temporal reprojection (reuse previous frame when camera/time/fluid static)
- Force-refresh counter (5 frames after grid changes for lightmap propagation)
- Viewport-culled lightmap propagation
- Lightmap throttle (every 2 frames)

## Jeff (First Pleb)

- Continuous movement (not grid-aligned), 4-corner bounding box collision
- WASD direct control (always active), Q/E rotation (when selected)
- A* pathfinding with visible green path line
- Auto-opens doors (close after 2 seconds)
- Torch (T): warm fire glow, 6-tile radius, wall-occluded via `trace_glow_visibility`
- Headlamp (G): directional white cone, 10-tile radius, wall-occluded
- Selection ring (pulsing green), click-to-select, click-to-move
- Blueprint placement mode with walkability preview
- Controls help modal

## UI Layout

- **Top-right**: version + FPS
- **Top-left**: Controls window (time, zoom, lighting, foliage, fluid sim, wind, render quality)
- **Bottom-left**: Pleb menu (above) + Build menu (below, compact single-column)
- **Bottom-right**: Overlay bar (Off/Gases/Smoke/O2/CO2/Vel/Pres + Debug/Glow/Bleed toggles)
- **Bottom-right above bar**: Gas legend (when overlay active) + Wind compass

## Overlay Modes

| Mode | Value | Description |
|------|-------|-------------|
| Off | 0 | Normal rendering with subtle smoke/O2/CO2 effects |
| Gases | 1 | All gases with distinct colors (white smoke, blue O2 deficit, yellow-green CO2) |
| Smoke | 2 | Smoke density as black→red→yellow→white heat map |
| Velocity | 3 | Direction as hue, magnitude as brightness, per-block arrows |
| Pressure | 4 | ROYGBIV absolute pressure with bilinear interpolation |
| O2 | 5 | Blue (atmospheric) → red (depleted) |
| CO2 | 6 | Dark (none) → yellow-green (high) |

## Build Items

| Item | Type | Placement | Notes |
|------|------|-----------|-------|
| Fire | 6 | Ground | Consumes O2, produces CO2 + smoke + heat velocity |
| Bench | 9 | Ground | 3-tile, rotatable H/V |
| Fan | 12 | Walls only | 4 directions, forced airflow, one-way valve |
| Compost | 13 | Ground | Produces CO2 continuously |
| Ceiling Light | 7 | Ground | Electric light, height 0 |
| Floor Lamp | 10 | Ground | Height 2 |
| Table Lamp | 11 | On bench | Height 1 |

All placeable items removable by clicking.

## What's Planned Next (from SPEC.md/PLAN.md)

**Phase 2d**: Temperature field (dye.a channel), buoyancy, block thermal mass
**Phase 2e**: Extended gas system (Gas Texture 2: H2O, CH4, CO, H2), chemical reactions
**Phase 2f**: Phase transitions (water↔ice↔steam, dry ice↔CO2)
**Phase 3**: More plebs, needs system (hunger, warmth, rest), jobs
**Phase 4**: Resources, crafting, stockpiles
**Phase 5**: Weather, survival mechanics

## Key Conventions

- Workgroup size 8x8 for all compute shaders
- CameraUniform struct must match identically across Rust and all WGSL files
- FluidParams struct must match across Rust and all 3 fluid WGSL files
- Ping-pong: odd iteration counts so final result is in texture B
- Version in `VERSION` file (integer), bumped on every commit
- Version label in main.rs (`format!("v{} | {:.0} fps"`)

# Rayworld - Game Specification

## Vision

Rayworld is a colony survival simulation rendered entirely via raytracing, inspired by Rimworld's top-down management gameplay. It combines block-based world construction, real fluid mechanics simulation, and integrated physics into a cohesive, performance-first experience that runs natively and on the web.

The core differentiator: every system — rendering, fluids, physics, lighting, AI — is deeply integrated. Air flows through doors with proper velocity fields creating realistic eddies and turbulence, fire generates buoyant plumes with vorticity, smoke swirls through corridors following pressure gradients. The world is not a backdrop; it is a participant. The fluid simulation is not a game-y approximation — it is real Navier-Stokes fluid dynamics running in real time on the GPU.

## Core Pillars

1. **Real fluid mechanics.** The fluid simulation is the technical heart of the game. It solves the incompressible Navier-Stokes equations on a GPU-accelerated grid using the Stable Fluids method (Jos Stam) with vorticity confinement. Velocity, pressure, temperature, and gas composition are proper continuous fields that advect, diffuse, and interact. This is what makes Rayworld feel physically real, not approximated.

2. **Performance above all.** The simulation must be fast enough that the world feels alive at scale. Target: 60+ simulation ticks/sec at 512x512 in production. GPU compute shaders do the heavy lifting for both fluid sim and raytracing.

3. **Always raytraced.** No rasterization fallback. The visual identity is defined by raytraced lighting — volumetric fog that samples the real fluid density fields, soft shadows, light scattering through smoke and steam. The top-down perspective keeps ray complexity bounded.

4. **3D-ready architecture.** The world starts as a 2D block grid for the prototype, but the data model and simulation are designed from day one to support vertical block stacking (Z-layers). Walls have height. The fluid sim can extend to 3D. Multi-story buildings are a planned feature, not an afterthought.

5. **Integrated simulation.** Fluids, physics, lighting, and AI are not isolated systems. They read and write shared world state. A fire produces heat and CO2 that advects through the velocity field, creating buoyant vortical plumes that curl around obstacles, affecting pleb pathfinding and health, rendered as volumetric smoke by the raytracer.

6. **Emergent storytelling.** Like Rimworld, the game has no win condition. Drama emerges from systems interacting: a cold snap + broken heater + low wood = crisis. The player manages plebs through an unfolding story.

7. **Accessible scale.** Runs in a browser via WASM+WebGPU. The top-down 2.5D perspective and block-based world keep the rendering budget manageable even on integrated GPUs.

## Technology

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Memory safety, WASM target, strong ecosystem for games/compute |
| GPU API | wgpu (WebGPU) | Cross-platform (native + web), compute shader support for fluid sim + raytracing |
| WASM toolchain | wasm-pack / wasm-bindgen | Rust-to-web pipeline |
| Windowing | winit | Cross-platform, integrates with wgpu |
| ECS | Custom or hecs/bevy_ecs | Lightweight, cache-friendly entity management for plebs/items |
| Asset format | Custom voxel/SDF format | Imported from Blender via export script |
| Serialization | rkyv or bincode | Fast save/load of world state |

### Performance Targets (Prototype)

- 60 fps rendering at 1080p on integrated GPU (Intel Iris / Apple M-series)
- 60+ fluid sim ticks/sec on 128x128 sim grid (can render dye/density at higher resolution)
- < 2 second cold start in browser

### Performance Targets (Production)

- 60 fps rendering at 1080p on discrete GPU (GTX 1660 tier)
- 30 fps rendering at 1080p on integrated GPU
- 60+ fluid sim ticks/sec on 512x512 sim grid (or 256x256x8 for 3D)
- Simulation tick rate decoupled from render frame rate

## World Model

### Block Grid

The primary world is a 2D grid of blocks (tiles), each representing roughly 1m x 1m. Each block has:

- **Terrain type** (soil, rock, water, void)
- **Structure** (wall, floor, door, none)
- **Height** (integer, 0-N blocks tall — used for raytracing occlusion and future 3D stacking)
- **Contents** (items, furniture)
- **Lighting state** (illumination level, color)

Note: gas/fluid state is NOT stored per-block. It lives in the fluid simulation grid, which operates at its own resolution (see Fluid Grid below).

### Sub-block Grid

Certain objects (furniture, decorations, small items) can be placed on a finer sub-grid within a block. Resolution: **4x4 per block** (2D for now).

This allows:
- Furniture placement with more precision than 1-tile snapping
- Visual variety within tiles
- Small item positioning (e.g., dropped resources)

Sub-blocks are a rendering and placement concern. They do NOT participate in fluid simulation.

### Vertical Dimension (3D Stacking)

The long-term goal is full 3D block stacking:
- Blocks can be stacked vertically (Z-layers)
- Fluid simulation extends to 3D when Z-layers are present
- Multi-story buildings, basements, elevation changes

For the prototype, the world is a single Z-layer with variable block heights. But the data model uses (x, y, z) coordinates internally from day one so that adding Z-layers is an extension, not a rewrite.

```
Prototype:  world[x][y]        — single layer, blocks have height attribute
Production: world[x][y][z]     — full 3D grid, fluid sim runs in 3D
```

### Fluid Grid

The fluid simulation operates on its own grid, which is decoupled from both the block grid and the rendering resolution. This is a key architectural insight from PavelDoGreat's implementation — the sim can run at a coarse resolution (e.g., 128x128) while density/dye is rendered at a much higher resolution (e.g., 1024x1024) via bilinear interpolation.

**Sim grid fields** (all f32, stored as GPU textures/storage buffers):
- **Velocity** (vec2 for 2D, vec3 for 3D) — the flow field
- **Pressure** (scalar) — enforces incompressibility
- **Divergence** (scalar, scratch) — used during pressure solve
- **Curl** (scalar, scratch) — used for vorticity confinement
- **Temperature** (scalar) — advected by velocity, drives buoyancy

**Density fields** (can run at higher resolution than sim):
- Per-gas density: O2, CO2, smoke, water vapor, etc.
- Each advected independently through the same velocity field
- Rendered at display resolution for visual fidelity

**Obstacle field**: derived from block grid. Walls = solid cells, doors = dynamic (open/closed), floors = open. Updated when blocks change.

## Systems

### Fluid Mechanics System (P0 - Required for Prototype)

This is the core technical system. It solves the incompressible Navier-Stokes equations on a 2D Eulerian grid, running entirely on GPU compute shaders. The implementation follows the Stable Fluids method (Jos Stam, 1999) with enhancements from GPU Gems (Mark Harris) and PavelDoGreat's WebGL implementation.

#### Algorithm (Per Simulation Tick)

```
1. COMPUTE CURL of velocity field
2. APPLY VORTICITY CONFINEMENT (amplify curls to counteract numerical dissipation)
3. COMPUTE DIVERGENCE of velocity field
4. CLEAR PRESSURE (with configurable damping, not full zero — preserves some temporal coherence)
5. JACOBI PRESSURE SOLVE (20 iterations default, tunable)
6. SUBTRACT PRESSURE GRADIENT from velocity (project to divergence-free)
7. ADVECT velocity through itself (semi-Lagrangian)
8. ADVECT temperature through velocity
9. ADVECT each density field (O2, CO2, smoke, etc.) through velocity
10. APPLY BUOYANCY (temperature + density → vertical force)
11. APPLY EXTERNAL FORCES (fire, plebs, wind, player interaction)
12. ENFORCE BOUNDARY CONDITIONS (obstacle field from block grid)
```

Each step is a GPU compute shader dispatch over the full grid. Double-buffered (ping-pong) textures prevent read/write hazards.

#### Advection (Semi-Lagrangian)

For each cell, trace backward through the velocity field to find where the fluid "came from", and sample that value with bilinear interpolation. This is unconditionally stable regardless of timestep (Stam's key insight). Dissipation is applied per-field to control how quickly quantities fade.

```wgsl
// Advection compute shader (pseudocode)
let pos = cell_coords_to_uv(global_id.xy);
let vel = textureSample(velocity_field, pos).xy;
let source_pos = pos - vel * dt * texel_size;
let value = bilinear_sample(source_field, source_pos);
let decay = 1.0 + dissipation * dt;
output[global_id.xy] = value / decay;
```

Key tuning parameters:
- **Velocity dissipation**: 0.2 (how fast velocity fades — low = long-lasting currents)
- **Density dissipation**: 1.0 per gas type (smoke fades faster than CO2)
- **Temperature dissipation**: ~0.5 (heat lingers but doesn't persist forever)

#### Vorticity Confinement

Numerical dissipation in grid-based methods kills small-scale vortices. Vorticity confinement adds a corrective force that amplifies existing curl, preserving the "swirly" look that makes fluid sim visually compelling. This is what makes smoke look realistic rather than blobby.

```wgsl
// Step 1: Compute curl (scalar in 2D = dVy/dx - dVx/dy)
let L = velocity[x-1, y].y;
let R = velocity[x+1, y].y;
let T = velocity[x, y+1].x;
let B = velocity[x, y-1].x;
curl[x, y] = 0.5 * (R - L - T + B);

// Step 2: Apply vorticity force (push velocity along curl gradient)
let curl_L = abs(curl[x-1, y]);
let curl_R = abs(curl[x+1, y]);
let curl_T = abs(curl[x, y+1]);
let curl_B = abs(curl[x, y-1]);
let curl_C = curl[x, y];
var force = 0.5 * vec2(abs(curl_T) - abs(curl_B), abs(curl_R) - abs(curl_L));
force = normalize(force + 0.0001) * curl_strength * curl_C;
velocity[x, y] += force * dt;
```

**Curl strength** (default ~30): controls how much vorticity is preserved. Higher = more dramatic swirls.

#### Pressure Solve (Jacobi Iteration)

The pressure Poisson equation is solved iteratively. Each iteration reads neighbor pressures:

```wgsl
let pL = pressure[x-1, y];
let pR = pressure[x+1, y];
let pB = pressure[x, y-1];
let pT = pressure[x, y+1];
let div = divergence[x, y];
pressure_out[x, y] = (pL + pR + pB + pT - div) * 0.25;
```

**Pressure iterations** (default 20): more = better incompressibility, at GPU cost. 20 is a good balance. Each iteration is a full-grid dispatch, making this the most expensive step (~50% of total fluid sim cost).

**Pressure clear value** (default 0.8): before each solve, the old pressure field is multiplied by this value rather than cleared to zero. This preserves temporal coherence and helps convergence. 0.0 = full clear each frame, 1.0 = no clear.

#### Boundary Conditions

Obstacles (walls) are handled per-shader by checking the obstacle field:
- Solid cells get zero velocity after each step
- Pressure solve uses reflected values at solid boundaries (no-slip)
- Doors are dynamic obstacles: open door = open cell, closed door = solid cell
- World edges can be open (outflow), closed (wall), or periodic (wrap)

The obstacle field is a GPU texture derived from the block grid. It only needs re-uploading when blocks change (build/destroy), not every tick.

#### Multi-Gas Support

Multiple density fields (O2, CO2, smoke, water vapor, etc.) are each advected independently through the shared velocity field. They have independent:
- **Dissipation rates** (smoke fades fast, CO2 lingers)
- **Sources** (fire → CO2 + smoke, plebs → CO2, plants → O2)
- **Sinks** (fire consumes O2, plebs consume O2)
- **Buoyancy contribution** (hot smoke rises, cold dense gas sinks)

Each gas type can optionally run at a different resolution from the velocity sim. Visually important gases (smoke) benefit from high-res density; gameplay-relevant gases (O2/CO2) can run at sim resolution.

#### Temperature and Buoyancy

Temperature is a scalar field advected by velocity. It drives buoyancy forces: hot cells push velocity upward (in the top-down view this manifests as outward expansion from heat sources), cold cells push downward. Combined with vorticity confinement, this creates natural convection — rising plumes that curl and eddy.

```wgsl
let temp_delta = temperature[pos] - ambient_temp;
let buoyancy = temp_delta * buoyancy_coefficient;
let smoke_weight = density_smoke[pos] * smoke_weight_coefficient;
velocity[pos].y += (buoyancy - smoke_weight) * dt;
```

#### Sim / Render Resolution Decoupling

Following PavelDoGreat's architecture, the simulation runs at a coarser resolution than the visual output:

| Layer | Prototype | Production |
|-------|-----------|------------|
| Velocity sim | 128x128 | 256x256 - 512x512 |
| Pressure/divergence/curl | same as velocity | same as velocity |
| Density (visual: smoke, dye) | 512x512 | 1024x1024 |
| Density (gameplay: O2, CO2) | same as velocity | same as velocity |
| Temperature | same as velocity | same as velocity |

Density fields are advected using bilinear-interpolated velocity from the coarser grid. This gives high visual fidelity at low compute cost.

#### Future: Hybrid MPM (Material Point Method)

For liquid water and deformable materials, a Lagrangian particle system (MPM, as in the Floom reference) can be layered on top of the Eulerian grid. Particles carry material properties and transfer momentum to/from the grid each tick (particle-to-grid, grid solve, grid-to-particle). This is post-prototype scope but architecturally compatible — MPM naturally uses the same grid as the Eulerian solver.

Use cases: flowing water, lava, mud, snow, blood.

#### Future: 3D Extension

When Z-layers are added, the fluid sim extends from 2D to 3D:
- Velocity becomes vec3
- Curl becomes vec3 (full vorticity vector)
- Pressure solve operates on 6-connected neighbors instead of 4
- Jacobi iteration: `(pL + pR + pB + pT + pDown + pUp - div) / 6.0`
- Cost scales linearly with Z-layers

At 256x256x8 = 524K cells, still very feasible. At 512x512x16 = 4.2M cells, may need reduced iteration count or multigrid solver.

### Rendering (P0 - Required for Prototype)

- **Software raytracer running on GPU compute shaders** (wgpu compute pipeline)
- Top-down camera with slight perspective / fisheye (configurable)
- Per-pixel ray march against the block grid + sub-block detail
- Lighting: direct light from sources (sun, torches, fire) with soft shadows
- **Volumetric rendering**: ray marches sample fluid density fields (smoke, steam) for volumetric attenuation and scattering. The fluid sim IS the volumetric data source.
- **Post-processing**: bloom on emissive surfaces (fire, lava), sunrays through smoke (as in PavelDoGreat's implementation)
- Simple material system: diffuse color + emission for prototype

### Physics (P1 - Post-Prototype)

Scope for v1 is minimal:
- Projectile trajectories (arrows, thrown objects) — simple ballistic curves
- Structural integrity — walls/roofs collapse if unsupported (grid-based check, not continuous)
- Fluid-coupled particles: objects in fluid feel drag from the velocity field

This is NOT a general rigid body engine. Keep scope tight.

### Pleb AI (P0 - Required for Prototype)

- Needs-based behavior: each pleb has needs (hunger, warmth, rest, safety)
- Job queue: player assigns priorities, plebs pick highest-priority available job
- Pathfinding: A* on the block grid, with fluid state as cost modifiers (plebs avoid smoke, extreme cold, low O2 — all sampled from fluid grid)
- Skills: each pleb has skill levels affecting work speed/quality
- Health: temperature exposure, oxygen deprivation, injury — all derived from fluid grid state at pleb's position

### Storyteller / Event System (P1 - Post-Prototype)

- Random event generator with difficulty scaling
- Weather events (cold snaps, heat waves, rain, wind) — these inject forces and sources into the fluid sim
- Raid events, wanderer joins, trade caravans
- Modeled after Rimworld's storyteller concept

### Resource / Crafting System (P0 - Required for Prototype)

- Resources: wood, stone, food, fuel (minimum viable set)
- Gathering: plebs chop trees (wood), mine rocks (stone), forage (food)
- Crafting: build walls, floors, doors, campfire, basic furniture
- Stockpiles: designated zones for resource storage

### UI / Player Interaction (P0 - Required for Prototype)

- Mouse-driven: select plebs, designate zones, place buildings
- Overlay modes: velocity field (arrows/streamlines), temperature map, gas composition, pressure
- Pleb info panel: needs, skills, health, current task
- Build menu: list of constructable items
- Time controls: pause, 1x, 2x, 3x speed
- **Fluid interaction**: click/drag to inject velocity impulses and density (for testing and fun — like the inspiration demos)

### Asset Pipeline (P1 - Post-Prototype)

- Blender export script: converts simple low-poly models to voxelized format
- Voxel resolution matches sub-block grid
- Assets stored as compact voxel arrays with material indices
- Runtime: assets are stamped into the world at sub-block positions

### Save / Load (P0 - Required for Prototype)

- Full world state serialization: block grid, pleb state, resource state
- Fluid state: velocity + pressure + temperature + density fields
  - Option A: serialize full state (large but exact resume)
  - Option B: serialize sources/sinks only, let fluid re-equilibrate on load (smaller, slight visual discontinuity)
- Fast binary format (rkyv or bincode)
- Autosave on interval

### Audio (P2 - Future)

- Ambient soundscape driven by world state (wind velocity from fluid sim, fire crackling, pleb chatter)
- UI feedback sounds
- Not in prototype scope

### Multiplayer (P2 - Future / Maybe Never)

- Not in scope for v1. Design as single-player only.
- If pursued later: lockstep simulation with deterministic ticks. GPU fluid sim determinism is non-trivial (floating point ordering).

### Modding (P2 - Future)

- Data-driven content (items, materials, recipes, gas types) loaded from files
- Scripting layer (Lua or WASI-based) for behavior modification
- Not in prototype scope, but data-driven design from day one makes this easier later

## Open Questions

1. **Camera perspective**: Exactly how much perspective / fisheye? Needs prototyping to find what feels right.
2. **Fluid sim resolution vs block grid**: 1:1? 0.5:1? The sim grid should probably be coarser than the block grid (e.g., 128x128 sim for a 256x256 block world). Needs benchmarking.
3. **Density render resolution**: How high can we push the density/dye textures while maintaining 60fps? PavelDoGreat runs dye at 1024x1024 on WebGL — we should be able to match or exceed this on WebGPU compute.
4. **Jacobi iteration count**: 20 is a good default. Trade-off between visual quality of incompressibility and GPU cost. Should be tunable at runtime (settings menu).
5. **Vorticity strength**: Default ~30. Too low = blobby smoke. Too high = unrealistic turbulence. Needs visual tuning per gas type.
6. **3D fluid sim feasibility**: At what Z-layer count does the pressure solve become too expensive for 60 ticks/sec? Needs benchmarking. Multigrid solver may be needed for 16+ layers.
7. **ECS choice**: Custom ECS vs hecs vs bevy_ecs? Prototype with hecs, migrate if needed.
8. **WebGPU readiness**: Browser support for WebGPU compute shaders is still rolling out. Fallback: WebGL2 fragment-shader-based fluid sim (like PavelDoGreat's approach — proven to work).
9. **Art style**: How stylized? Rimworld is flat/simple. Raytracing + volumetric fluid enables much richer visuals — where's the line?
10. **Hybrid MPM timing**: When to add particle-based fluids (water, lava)? Needs the Eulerian grid working and stable first.
11. **Sub-block resolution**: Is 4x4 sufficient or do we need 8x8? Depends on visual fidelity testing.
12. **Fluid save strategy**: Full state serialization vs re-equilibration on load?

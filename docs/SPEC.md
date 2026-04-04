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

Temperature is stored in the dye texture A channel (Rgba16Float, actual Celsius values). It is advected by velocity, diffuses through air, and is blocked by walls. Fire injects ~300°C; outdoor ambient varies with time of day (5°C night, 25°C midday).

Temperature drives buoyancy: hot cells expand outward (in the top-down view), creating convection currents. Combined with vorticity confinement, this produces natural plume behavior.

```wgsl
let temp_delta = temperature[pos] - ambient_temp;
let buoyancy = temp_delta * buoyancy_coefficient;
velocity[pos].y += buoyancy * dt;
```

#### Multi-Gas Architecture

Gas species are packed into RGBA16Float textures, 4 species per texture. All textures share the same velocity field and are advected independently. Adding new species costs one advection dispatch per 4 gases.

**Gas Texture 1** (dye, 512x512 — visual + core gameplay):
- R: smoke density (visual haze particles)
- G: O2 (oxygen, atmospheric = 1.0)
- B: CO2 (carbon dioxide)
- A: air temperature (°C)

**Gas Texture 2** (256x256 — extended chemistry):
- R: H2O vapor (steam, humidity)
- G: CH4 (methane — flammable)
- B: CO (carbon monoxide — toxic, flammable)
- A: H2 (hydrogen — explosive)

**Gas Texture 3+** (future, same pattern):
- SO2, NH3, noble gases, biological agents, etc.

Each gas has independent:
- **Dissipation**: smoke fades fast, CO2 lingers, O2/N2 are conserved
- **Sources/sinks**: fire → CO2 + H2O + heat; plebs → CO2; plants → O2; decay → CH4
- **Density**: affects buoyancy (CO2 sinks, H2 rises, steam rises)
- **Toxicity**: CO and CO2 at high concentration damage plebs
- **Flammability**: CH4, H2, CO ignite above threshold temperatures

#### Chemical Reactions

A post-advection compute pass checks reaction conditions per cell and applies mass-action kinetics. Reactions consume reactants, produce products, and inject/absorb heat.

| Reaction | Equation | Ignition Temp | Heat |
|----------|----------|---------------|------|
| Methane combustion | CH4 + 2O2 → CO2 + 2H2O | >580°C | Exothermic |
| CO combustion | 2CO + O2 → 2CO2 | >600°C | Exothermic |
| Hydrogen combustion | 2H2 + O2 → 2H2O | >500°C | Very exothermic |
| Wood/coal burning | (block) + O2 → CO2 + smoke + heat | >250°C | Exothermic |
| Water gas-shift | CO + H2O ⇌ CO2 + H2 | >400°C | Mildly exo |

Reaction rate: `rate = k * [A] * [B] * dt` where k = 0 below ignition temperature, proportional to temperature above it (simplified Arrhenius).

Exothermic reactions inject heat into the temperature field (dye.a). Endothermic reactions absorb heat. Chain reactions emerge naturally: a methane leak near a fire ignites, producing heat that ignites more methane → explosion propagates through the gas cloud.

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

### Pleb AI (P0 - Implemented)

- 7 needs: hunger, thirst, rest, warmth, oxygen, mood, stress. Health damage from unmet needs.
- Crisis system: auto-eat/sleep when critically low, overrides player commands
- 6 skills: shooting, melee, crafting, farming, medical, construction (1-10 scale)
- 4-priority work system: haul, farm, build, craft. Per-pleb priority toggles.
- Pathfinding: A* with wall awareness, door passability, water cost gradient, terrain elevation
- Activities: idle, walking, sleeping, harvesting, eating, hauling, farming, building, crafting, digging, filling, drinking, butchering, staggering, mental breaks
- Combat: ranged (pistol with magazine/reload), melee (axe/pick/shovel), grenades, crouching, peek-fire, suppression, bleeding
- Morale: S-curve stress, vulnerability spiral, leader aura recovery, panic contagion
- Ranks: Green → Trained → Veteran → Hardened (combat experience tracking)
- Leaders: command shouts (Rally/Advance/FallBack), morale aura
- Equipment: auto-equip best weapon, equipped items protected from hauling
- Need emotes: thought bubbles at hunger/thirst/rest/warmth thresholds with sound
- Per-pleb event log (capped at 50 entries, shown in character sheet)
- Hunt command: right-click fauna to stalk and shoot

### Weather System (P1 - Implemented)

- 4 states: Clear, Cloudy, LightRain, HeavyRain with probabilistic transitions (30-90s)
- Rain: wind-angled multi-layer streaks with parallax, ground splash ripples, atmospheric haze
- Lightning strikes during heavy rain (random outdoor tile, 120dB thunder sound, power surge)
- Per-tile wetness tracking with evaporation (sun-dependent)
- Rain boosts water table, affects crop growth
- Drought periods: no rain, boosted sun intensity

### Fauna System (P1 - Implemented)

- Data-driven creature definitions (creatures.toml): health, speed, damage, behavior flags
- Hostile: Duskweavers (nocturnal pack hunters, flee light), Hollowcalls (immobile sound entities)
- Passive: Dusthares (skittish, hop-based movement, Browse/Scatter AI)
- Worldgen spawn (10-15 dusthares), slow reproduction (90-150s if 2+ alive, cap 20)
- Hunting: right-click → Hunt command, pleb follows + precision aims
- Butchering: right-click corpse → Butcher (4s), yields raw meat
- Creature rendering: GPU buffer (32 max), per-species color/eyes, hop animation
- Hover outlines on creatures, sprite blocks (trees, bushes, rocks)

### Storyteller / Event System (P2 - Future)

- Random event generator with difficulty scaling
- Raid events, wanderer joins, trade caravans
- Modeled after Rimworld's storyteller concept

### Resource / Crafting System (P0 - Required for Prototype)

- Resources: wood, stone, clay, fiber, sticks, logs, planks, rope, berries, raw meat, cooked meat
- Gathering: plebs chop trees (sticks+fiber), mine rocks, forage berries, hunt fauna
- Crafting: data-driven recipes (recipes.toml). Stations: workbench, saw horse, kiln, hand
- Storage: crate blocks for item storage, storage zones for ground items
- Items: data-driven (items.toml) with icons, categories, nutrition, weapon stats, liquid capacity
- Equipment: plebs auto-equip weapons/tools, equipped items protected from hauling

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

### Audio (P1 - Implemented)

- Procedural spatial audio: 8 sound patterns (impulse, sine, noise, thud, slash, gunshot, bullet_impact, explosion)
- Spatialized with stereo panning and distance attenuation (60-tile range)
- Rain ambience: continuous filtered noise layered (wash + patter + individual drops)
- Thunder on lightning strikes
- Need emote sounds (soft noise when hunger/thirst thresholds cross)
- UI click sounds

### Multiplayer (P2 - Future / Maybe Never)

- Not in scope for v1. Design as single-player only.
- If pursued later: lockstep simulation with deterministic ticks. GPU fluid sim determinism is non-trivial (floating point ordering).

### Modding (P2 - Future)

- Data-driven content (items, materials, recipes, gas types) loaded from files
- Scripting layer (Lua or WASI-based) for behavior modification
- Not in prototype scope, but data-driven design from day one makes this easier later

### Thermodynamics System (P0 - Required for Prototype)

Two-domain temperature model: air temperature (fluid-carried) and block temperature (material-stored).

#### Air Temperature

Stored in the dye texture A channel (Rgba16Float, actual Celsius values). Advected by velocity, diffuses through air, blocked by walls.

- **Sources**: fire (~300°C), hot blocks radiating
- **Sinks**: cold outdoor ambient, ice, cold blocks absorbing
- **Ambient**: varies with time of day (5°C night, 25°C midday)
- **Visualization**: temperature overlay (blue=cold, white=mild, red=hot)
- **Buoyancy**: hot air creates outward expansion (velocity force proportional to temperature delta)

#### Block Temperature & Thermal Mass

Each block material has thermal properties:

| Material | Heat Capacity | Conductivity | Notes |
|----------|--------------|--------------|-------|
| Air | ~0 (uses fluid) | N/A | Temperature carried by wind |
| Stone/Wall | 4.0 | 0.002 | Slow to heat, slow to cool |
| Water | 8.0 | 0.01 | Huge thermal buffer |
| Ice | 4.0 | 0.01 | Melts at 0°C |
| Wood/Dirt | 2.0 | 0.003 | Moderate storage |
| Glass | 1.5 | 0.02 | Conducts heat fast |
| Metal (fan) | 1.0 | 0.05 | Conducts quickly |

Blocks exchange heat with adjacent air cells per frame. Heat conducts slowly through solid walls.

#### Phase Transitions

| Transition | Trigger | Result |
|------------|---------|--------|
| Water → Ice | block temp < 0°C | Becomes solid obstacle, light blue visual |
| Ice → Water | block temp > 0°C | Becomes liquid, fluid-passable |
| Water → Steam | block temp > 100°C | Block loses mass, steam injected into fluid |
| Steam → Water | air temp < 100°C + high humidity | Condensation deposits water |
| Dry Ice → CO2 | block temp > -78°C | Solid CO2 sublimes to gas |
| CO2 → Dry Ice | air temp < -78°C + high CO2 | Reverse sublimation |

Phase transitions run on CPU (scan affected blocks each frame, check temperatures).

---

## Implementation Status (v186+)

### Completed

**Phase 0: Foundation** ✅
- [x] Rust project with wgpu, winit, egui
- [x] 256x256 block grid with 30 block types
- [x] Compute shader raytracer (top-down, per-pixel)
- [x] Camera pan/zoom controls
- [x] WASM build target (Trunk), GitHub Pages deploy
- [x] Modular codebase: 14 Rust modules + 8 WGSL shaders

**Phase 1: World & Lighting** ✅
- [x] 30 block types via data-driven material table (GpuMaterial with 24 properties each)
- [x] 5 wall materials (wood, steel, sandstone, granite, limestone), 3 floor types, insulated walls
- [x] Block height and oblique south-face projection (material-driven)
- [x] Directional sun with shadow ray marching, foliage dappling
- [x] Point lights: fireplace, electric light, standing lamp, table lamp (per-material properties)
- [x] Day/night cycle with dawn/dusk color transitions, time-of-day buttons
- [x] GPU lightmap: 512x512 (2x res) with flood-fill propagation (26 iterations, viewport-culled)
- [x] Proximity glow with line-of-sight tracing (wall-occluded, height-aware)
- [x] Directional light bleed through windows/doors
- [x] Tree sprites (8 conifer variants), bush sprites (16 variants), rock sprites (32 variants), noise-clustered forests
- [x] Build menu with drag-to-place, destroy tool, roofing, windows, doors
- [x] sRGB gamma correction (consistent native/web appearance)

**Phase 2a: Core Fluid Solver** ✅
- [x] GPU Navier-Stokes (Stable Fluids) at 256x256
- [x] Curl, vorticity confinement, divergence, Jacobi pressure (35 iterations, Neumann BCs)
- [x] Gradient subtract, semi-Lagrangian advection
- [x] Dye field at 512x512 with obstacle-aware bilinear sampling

**Phase 2b: World Integration** ✅
- [x] Fire blocks: O2-dependent combustion, produce CO2 + smoke + heat velocity
- [x] Compost blocks: anaerobic CO2 production
- [x] Wall fan (type 12): forced directional airflow, one-way valve, adjustable speed
- [x] Global wind with UI sliders + compass indicator
- [x] Smoke diffusion + accumulation, edge dissipation, windward O2 injection

**Phase 2c: Rendering & Debug** ✅
- [x] 8 overlay modes: Off, Gases, Smoke, Velocity (with arrows), Pressure (ROYGBIV), O2, CO2, Temp, Heat Flow
- [x] Debug tooltip with block info + gas values (native) or block info only (WASM)
- [x] 20-tile border fog-of-war

**Phase 2d: Temperature & Thermodynamics** ✅
- [x] Air temperature in dye.a channel (Celsius values, advected by fluid)
- [x] Fire injects ~300°C (O2-dependent), outdoor ambient varies 5-25°C with day/night
- [x] Gradient-based buoyancy (radial heat expansion, not fixed direction)
- [x] Smoke-weight coupling (dense smoke sinks, opposes thermal lift)
- [x] Block temperature buffer (256x256 f32) with per-frame thermal exchange
- [x] Material thermal properties: heat capacity, conductivity, solar absorption
- [x] Solar heating of outdoor sunlit blocks
- [x] Neumann BCs for temperature at walls (no heat sink)
- [x] Indoor cells retain heat (zero dissipation when roofed)
- [x] Door opening pressure release (velocity burst from hot rooms)
- [x] Temperature overlay + Heat Flow overlay (velocity colored by temperature)
- [x] Heat shimmer visual near fire, cold blue tint

**Phase 2e: Piping System** ✅
- [x] 6 pipe components: Pipe, Pump, Tank, Valve, Outlet, Inlet
- [x] CPU-side pressure network simulation (pressure equalization, gas transport)
- [x] Auto-connecting pipe rendering (straight, corner, T, cross from neighbors)
- [x] Valve toggle (click to open/close), Pump speed slider
- [x] Tank with fill gauge, inlet/outlet directional chevrons
- [x] Pipe overlay mode

**Phase 3: Plebs** ✅
- [x] Multi-pleb system (up to 16 colonists, Vec<Pleb>)
- [x] Randomized appearance: skin tone, hair color/style, shirt, pants
- [x] In-world sprite matches portrait (shirt body, skin head, hair)
- [x] Click-to-select, right-click-to-move A*, group selection
- [x] Auto-open doors (close after 2 seconds)
- [x] Torch and headlight (3 beam modes: wide/normal/focused) per pleb
- [x] Colonist bar with portraits at top of screen
- [x] GpuPleb storage buffer for multi-pleb GPU rendering
- [x] 7 needs: hunger, thirst, rest, warmth, oxygen, mood, stress
- [x] Health system: damage from suffocation, cold, fire, combat, starvation, dehydration
- [x] Crisis-driven AI: auto-eat/sleep when needs critical
- [x] Work priority system (haul/farm/build/craft, per-pleb 0-3 priorities)
- [x] Backstories + personality traits (data-driven)
- [x] Non-linear morale: S-curve stress, vulnerability spiral, panic contagion
- [x] Combat: ranged, melee, grenades, crouching, peek-fire, suppression
- [x] Ranks (Green→Trained→Veteran→Hardened), leaders, command shouts
- [x] BG2-style character window: equipment | portrait | stats, tabbed bottom (Gear/Log/Modifiers)
- [x] Need-based thought bubbles + status labels with priority-based colored pills
- [x] Equipment system: equip/unequip via right-click, haul protection
- [x] Hunting: right-click fauna → Hunt command, follow + precision aim
- [x] Butchering: right-click corpse → Butcher activity (4s) → raw meat

**Phase 3b: Physics** ✅
- [x] PhysicsBody with 3D position (x,y,z), velocity, rotation
- [x] Gravity (25 tiles/sec²), bounce, wall collision
- [x] Wood boxes: pushable by plebs, throwable (F key)
- [x] Wind and fan forces affect physics bodies
- [x] Cannon block (type 29): directional, 360° rotation, click to fire
- [x] Cannonball: ballistic arc, kinetic energy impact damage, block destruction
- [x] Cannonball shadow + trajectory visualization

**Performance Optimizations** ✅
- [x] Configurable render resolution (0.15-1.0 of window)
- [x] Temporal reprojection with force-refresh system
- [x] Conditional proximity glow (lightmap gate)
- [x] Toggleable glow/bleed/temporal via UI
- [x] Lightmap throttling (every 2 frames, forced on grid changes)
- [x] Precomputed sun parameters on CPU
- [x] 271 unit tests across 4 modules

**Phase 3c: Weather & Water** ✅
- [x] Weather state machine (Clear/Cloudy/LightRain/HeavyRain)
- [x] Wind-angled rain with parallax layers, ground splashes, atmospheric haze
- [x] Lightning strikes during heavy rain with thunder sound
- [x] Per-tile wetness, evaporation, water table boost from rain
- [x] GPU water physics (512x512 ping-pong, symmetric flow, percolation drain)
- [x] Per-pixel water rendering with bilinear elevation sampling
- [x] Water-aware pathfinding (quadratic cost gradient, deep water blocked)
- [x] Sub-tile terrain elevation (1024x1024 heightmap), digging, berms

**Phase 3d: Fauna & Food** (Partial)
- [x] Creature system: data-driven (creatures.toml), GPU rendering, bleeding, corpses
- [x] Duskweavers (nocturnal pack hunters), Hollowcalls (sound entities)
- [x] Dusthares (passive fauna, Browse/Scatter AI, hop animation, worldgen spawn)
- [x] Hunt command, butchering, raw/cooked meat items
- [ ] Cooking work task (carry raw meat to campfire → cooked meat)
- [ ] Berry bush finite supply + regrow
- [ ] Snare trapping, fishing
- [ ] Mudcrawler, Thornback creatures
- [ ] Cooking combinations (Zelda-style)
- [ ] Food preservation (salt, smoking, drying)

**Phase 3e: Audio** ✅
- [x] Procedural spatial audio (8 patterns, stereo panning, distance falloff)
- [x] Rain ambience (continuous layered noise)
- [x] Need emote sounds

### Not Yet Started

**Phase 2 remaining:**
- [ ] Water phase transitions (freeze/evaporate)
- [ ] Chemical reactions (methane + fire = explosion)
- [ ] Extended gas system (H2O, CH4, CO, H2)

**Phase 4: Content & Survival**
- [ ] Save/load (world state serialization)
- [ ] Storyteller / event system
- [ ] Trade caravans, wanderer joins
- [ ] Tech tree / research

---

## Open Questions

1. ~~**Fluid sim resolution vs block grid**~~: Resolved — 1:1 at 256x256, dye at 512x512.
2. ~~**Jacobi iteration count**~~: Resolved — 35 iterations, Neumann BCs, 0.6 temporal damping.
3. ~~**Vorticity strength**~~: Resolved — 35.0 default.
4. **3D fluid sim feasibility**: Not yet tested. Future scope.
5. **ECS choice**: Not yet needed. Single-file architecture for now.
6. **WebGPU readiness**: Builds with Trunk for WASM. Testing needed.
7. **Art style**: Evolving. Raytraced with procedural materials, top-down oblique.
8. **Hybrid MPM timing**: After temperature/phase transitions are working.
9. **Block temperature storage**: Use separate CPU array or GPU buffer? Bits 24-31 of grid are used for roof height.
10. **Phase transition performance**: CPU-side block scanning each frame — acceptable at 256x256?

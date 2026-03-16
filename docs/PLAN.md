# Rayworld - Development Plan

This document defines the phased roadmap from zero to playable prototype to production. Each phase has concrete deliverables, acceptance criteria, and estimated effort. Phases are sequential — each builds on the previous.

---

## Phase 0: Foundation (Weeks 1-2)

**Goal:** Runnable window with a rendered block grid. Prove the tech stack works end-to-end, native and web.

### Deliverables

- [ ] Rust project scaffolding: cargo workspace with crates for `core` (simulation), `render` (wgpu), `game` (glue + UI)
- [ ] winit window creation, wgpu device/surface setup
- [ ] Hardcoded 20x20 block grid in memory (struct with terrain type, wall bool)
- [ ] Compute shader raytracer: cast rays from top-down camera, hit block grid, output flat-shaded colors
- [ ] Camera controls: pan with mouse drag, zoom with scroll
- [ ] WASM build target: `wasm-pack build` produces a working web version
- [ ] CI: GitHub Actions builds native (Linux/macOS/Windows) + WASM on every push

### Acceptance Criteria

- Native window shows a colored grid rendered via raytracing
- Same thing runs in a browser via WASM+WebGPU
- 60 fps on both targets for a 20x20 grid
- Build is automated

### Key Decisions to Make

- Cargo workspace structure (propose: `crates/core`, `crates/render`, `crates/game`)
- wgpu version and feature flags
- WASM bundler (wasm-pack vs trunk)

---

## Phase 1: World & Lighting (Weeks 3-5)

**Goal:** A world that looks like something. Walls cast shadows, floors have texture, there's a day/night concept.

### Deliverables

- [ ] Expand block model: terrain types (grass, dirt, stone, water), structures (wall, floor, door)
- [ ] World generation: simple noise-based terrain for prototype maps (100x50)
- [ ] Block height: walls are tall, floors are ground level — raytracer respects height for occlusion
- [ ] Direct lighting: single sun light source, rays test occlusion for shadows
- [ ] Point lights: placeable light sources (torches), soft falloff
- [ ] Day/night cycle: sun angle changes over time, ambient light shifts
- [ ] Simple material system: diffuse color per block type, emission for light sources
- [ ] Optimized ray traversal: DDA grid traversal (not brute force), early termination

### Acceptance Criteria

- 100x50 map renders at 60 fps native, 30+ fps web
- Walls cast visible shadows from the sun
- Torches illuminate surrounding area with falloff
- Day visually transitions to night and back

---

## Phase 2: Fluid Mechanics (Weeks 6-10)

**Goal:** A real Navier-Stokes fluid simulation running on the GPU. Smoke, heat, and gas flow visibly and physically through the world.

This is the most technically ambitious phase and the core differentiator of the project. Extra time is allocated. The approach follows the Stable Fluids method (Stam 1999) with GPU compute, informed by PavelDoGreat's WebGL implementation and the GPU Gems article.

### Phase 2a: Core Solver (Weeks 6-7)

**Goal:** Standalone fluid sim that looks like the PavelDoGreat demo.

- [ ] GPU buffer setup: double-buffered textures/storage buffers for velocity (RG32F), pressure (R32F), divergence (R32F), curl (R32F)
- [ ] Advection shader: semi-Lagrangian backtracing with bilinear interpolation and configurable dissipation
- [ ] Divergence shader: compute divergence of velocity field
- [ ] Pressure solver: Jacobi iteration (20 iterations, configurable), with pressure clear/damping
- [ ] Gradient subtract shader: project velocity to divergence-free
- [ ] Curl shader: compute scalar curl of velocity field
- [ ] Vorticity confinement shader: amplify existing curls to preserve swirl structure
- [ ] Splat shader: inject velocity + density impulses at a point (Gaussian falloff)
- [ ] Mouse/touch interaction: click-drag to inject velocity and colored dye (like the inspiration demos)
- [ ] Basic density field: single "dye" density advected through velocity, rendered as colored output
- [ ] Configurable parameters exposed in debug UI: sim resolution, pressure iterations, curl strength, dissipation rates

### Acceptance Criteria (2a)

- Fluid sim runs standalone at 60fps on integrated GPU at 128x128 sim resolution
- Click-drag creates swirling, curling fluid motion that looks comparable to the PavelDoGreat demo
- Vorticity confinement visibly preserves small-scale eddies
- Fluid wraps around or reflects off screen boundaries correctly

### Phase 2b: World Integration (Weeks 8-9)

**Goal:** Fluid sim respects the block grid. Walls are obstacles, doors are dynamic boundaries.

- [ ] Obstacle field: GPU texture derived from block grid (solid/open per cell)
- [ ] Boundary enforcement in all shaders: solid cells get zero velocity, pressure solve respects solid neighbors
- [ ] Dynamic obstacles: doors toggle between solid and open, obstacle field updates on block change
- [ ] Partial permeability: doors can be partially open (fractional obstacle value)
- [ ] Temperature field: scalar advected by velocity, with source/sink support
- [ ] Buoyancy shader: temperature differential drives vertical velocity, smoke density applies downward force
- [ ] Fire source: block state that injects heat + velocity + smoke density into fluid sim
- [ ] Ambient temperature: outdoor cells trend toward ambient, modulated by time of day

### Acceptance Criteria (2b)

- Place walls in the world → fluid flows around them with visible eddies behind obstacles
- Open a door → fluid rushes through the gap with realistic acceleration
- Light a fire in an enclosed room → hot smoke rises and swirls, filling the room
- Open a door to a smoky room → smoke pours out, fresh air flows in, visible vortices at the doorframe

### Phase 2c: Multi-Gas & Rendering (Weeks 9-10)

**Goal:** Multiple gas types, volumetric rendering, overlays.

- [ ] Multi-gas density fields: O2, CO2, smoke — each advected independently through shared velocity field
- [ ] Per-gas dissipation rates: smoke fades faster than CO2
- [ ] Sources/sinks: fire (produces CO2 + smoke, consumes O2), plebs (consume O2, produce CO2), plants (consume CO2, produce O2)
- [ ] Separate sim vs density resolution: sim at 128x128, visual density (smoke) at 512x512+
- [ ] Volumetric raytracer integration: ray march samples smoke density field for attenuation/scattering
- [ ] Post-processing: bloom on emissive surfaces (fire), inspired by PavelDoGreat's bloom pipeline
- [ ] Debug overlays: velocity field (arrow/streamline visualization), temperature heatmap, O2/CO2/smoke concentration, pressure field
- [ ] Performance profiling: identify bottleneck (likely pressure solve), establish baseline ticks/sec

### Acceptance Criteria (2c)

- Fire produces visible smoke AND invisible CO2 spread. O2 depletes in enclosed space. Fire extinguishes when O2 drops below threshold.
- Smoke is volumetrically visible in the raytraced view as haze/fog
- Debug overlays clearly show all fluid fields
- 60+ ticks/sec at 128x128 sim resolution on integrated GPU
- Visual smoke at 512x512 density resolution looks smooth and detailed

### Technical Reference

The fluid sim pipeline per tick (in dispatch order):

```
┌─────────────────────────────────────────────────────┐
│ 1. Compute curl of velocity                         │ ← 1 dispatch
│ 2. Apply vorticity confinement                      │ ← 1 dispatch
│ 3. Compute divergence                               │ ← 1 dispatch
│ 4. Clear/damp pressure                              │ ← 1 dispatch
│ 5. Jacobi pressure solve                            │ ← 20 dispatches (!)
│ 6. Subtract pressure gradient                       │ ← 1 dispatch
│ 7. Advect velocity through itself                   │ ← 1 dispatch
│ 8. Advect temperature                               │ ← 1 dispatch
│ 9. Advect density fields (smoke, O2, CO2, etc.)     │ ← N dispatches (1 per gas)
│ 10. Apply buoyancy                                  │ ← 1 dispatch
│ 11. Apply external forces (fire, wind, etc.)        │ ← 1 dispatch
│ 12. Enforce boundary conditions                     │ ← 1 dispatch
└─────────────────────────────────────────────────────┘
Total: ~30+ dispatches per tick at 128x128 = trivially fast on GPU
```

Key tuning parameters (defaults from PavelDoGreat, will need game-specific tuning):

| Parameter | Default | Range | Effect |
|-----------|---------|-------|--------|
| Sim resolution | 128 | 64-512 | Grid cells. Higher = more detail, more GPU cost |
| Pressure iterations | 20 | 10-50 | More = better incompressibility. 20 is usually enough |
| Pressure clear | 0.8 | 0.0-1.0 | Temporal damping. 0 = full clear, 1 = no clear |
| Velocity dissipation | 0.2 | 0.0-4.0 | How fast flow fades. Low = persistent currents |
| Density dissipation (smoke) | 1.0 | 0.0-4.0 | How fast smoke fades |
| Density dissipation (CO2) | 0.1 | 0.0-1.0 | CO2 should linger much longer |
| Curl strength | 30 | 0-50 | Vorticity confinement. Higher = more swirls |
| Buoyancy coefficient | tunable | — | How strongly temperature drives velocity |

---

## Phase 3: Plebs - Core AI (Weeks 11-14)

**Goal:** Plebs exist, move, have needs, and interact with the fluid simulation. The game becomes interactive.

### Deliverables

- [ ] Pleb entity: position, name, needs (hunger, warmth, rest), skills, health
- [ ] Rendering: plebs rendered as simple voxel sprites in the raytracer
- [ ] Pathfinding: A* on block grid, recalculates on world change
- [ ] Needs system: needs decay over time, plebs seek to fulfill lowest need
- [ ] Fluid interaction — temperature: pleb warmth need affected by temperature field sampled at pleb position
- [ ] Fluid interaction — oxygen: low O2 at pleb's position causes health damage
- [ ] Fluid interaction — smoke: high smoke density impairs pleb vision/movement speed
- [ ] Pleb as fluid source: each pleb injects small CO2 source + consumes O2 at their grid cell
- [ ] Pathfinding cost modifiers: smoke density, temperature extremes, low O2 all increase path cost
- [ ] Basic jobs: idle, move-to, sleep (on ground for now)
- [ ] Player interaction: click to select pleb, see info panel, right-click to command move
- [ ] Spawn 3-5 plebs on map start

### Acceptance Criteria

- Plebs wander, pathfind around obstacles
- Plebs near fire get warm (temperature field), plebs far from fire get cold
- Plebs in smoky/low-O2 rooms take health damage
- Plebs actively avoid heavily-smoked areas when pathfinding
- Player can select and command plebs
- Pleb info panel shows needs, health, and local gas conditions

---

## Phase 4: Resources & Building (Weeks 15-18)

**Goal:** The core gameplay loop: gather resources, build structures, survive. Building changes the fluid sim.

### Deliverables

- [ ] Resource types: wood (from trees), stone (from rocks), food (from berry bushes)
- [ ] Trees and rocks: world objects that can be designated for harvesting
- [ ] Harvesting job: pleb walks to target, harvests over time (skill-dependent), resource drops
- [ ] Inventory: plebs carry resources, haul to stockpile zones
- [ ] Stockpile zones: player designates areas, plebs haul matching resources there
- [ ] Build system: player designates construction (wall, floor, door, campfire, bedroll)
- [ ] **Obstacle field update on build**: placing/removing a wall updates the fluid obstacle texture → fluid flow changes in real time
- [ ] Construction job: pleb carries required resources to site, builds over time
- [ ] Campfire: produces heat + light + smoke via fluid sources, consumes wood over time
- [ ] Bedroll: plebs sleep here to fulfill rest need
- [ ] Build menu UI: list of buildable items with resource costs
- [ ] Designation UI: zone painting for stockpiles, harvest orders

### Acceptance Criteria

- Player designates tree for chopping → pleb chops it → wood appears → pleb hauls to stockpile
- Player places campfire blueprint → pleb brings wood → campfire is built → it produces heat and light and smoke
- Player builds walls to enclose a room → fluid sim immediately respects new boundaries
- Player places bedroll → pleb sleeps on it at night
- Full survival loop: chop wood → build campfire → stay warm at night → forage food → don't die

---

## Phase 5: Weather & Survival (Weeks 19-21)

**Goal:** The world pushes back. Weather drives the fluid sim, survival becomes a challenge.

### Deliverables

- [ ] Ambient temperature: varies by time of day (cold at night, warm midday) — drives the fluid temperature field
- [ ] Weather system: clear, cloudy, rain, snow, wind — selected by simple state machine
- [ ] **Wind**: applies directional velocity impulses at map edges into the fluid sim. Smoke blows downwind. Buildings create wind shadows.
- [ ] Rain: adds water vapor density, reduces outdoor temperature, visual effect in raytracer
- [ ] Cold snap event: ambient temperature drops severely → fluid temperature field drops → plebs freeze
- [ ] Heat wave event: ambient temperature rises
- [ ] Hypothermia: prolonged cold exposure (from fluid temperature at pleb position) causes health damage, eventually death
- [ ] Starvation: prolonged hunger causes health damage, eventually death
- [ ] Suffocation: prolonged low O2 (from fluid O2 field) causes death
- [ ] Death: pleb dies, body remains as object
- [ ] Win condition for prototype: survive N days

### Acceptance Criteria

- Night is colder than day — fluid temperature field drops, plebs need shelter
- Wind blows smoke sideways — visible in fluid sim, affects where you build campfires
- Cold snap makes the temperature field drop severely — plebs huddle near fires
- Rain cools outdoor cells via the fluid sim
- A pleb can die from exposure, suffocation, or starvation if player mismanages
- Player can "win" by surviving a set number of days

---

## Phase 6: Polish & Ship Prototype (Weeks 22-24)

**Goal:** A playable, shareable prototype that demonstrates the core vision.

### Deliverables

- [ ] Save/load: full world state + fluid state to binary file
- [ ] Fluid save strategy: serialize velocity + density fields (or mark as "re-equilibrate on load" for smaller saves)
- [ ] Autosave: every N minutes
- [ ] Main menu: new game, load game, settings
- [ ] Settings: resolution, sim resolution, render quality, pressure iterations, simulation speed
- [ ] Fluid sim parameter tuning: find the sweet spot for all dissipation/curl/buoyancy values
- [ ] Tutorial hints: minimal text prompts for first-time players
- [ ] Performance profiling and optimization pass (especially pressure solve)
- [ ] Bug fixing and edge case handling
- [ ] Web deployment: hosted playable build (itch.io or custom page)
- [ ] Native release builds: Linux, macOS, Windows

### Acceptance Criteria

- New player can figure out the core loop within 5 minutes
- Game runs stable for 30+ minutes without crashes
- Save, quit, reload — world state is preserved (fluid state resumes or re-equilibrates)
- Web version loads in < 5 seconds, runs at 30+ fps with fluid sim active
- Native version runs at 60 fps with fluid sim active
- The fluid sim is visually impressive enough to be the "wow factor" in a demo

---

## Post-Prototype Roadmap (Not Scoped in Detail)

Each gets its own detailed plan when the time comes.

### Tier 1 — Near Term
- **3D block stacking (Z-layers)**: extend block grid to 3D, extend fluid sim to 3D (vec3 velocity, 6-neighbor pressure solve)
- **Storyteller / event system**: Rimworld-style AI director generating events based on colony state
- **More building types**: walls from different materials, roofs, furniture
- **Crafting system**: workbenches, recipes, intermediate materials

### Tier 2 — Medium Term
- **MPM hybrid fluid sim**: particle-based water/lava/mud layered on top of Eulerian grid (Floom-inspired)
- **Combat**: hostile entities, weapons, armor, defensive structures
- **Blender asset pipeline**: export script, voxelization, runtime loading
- **Advanced gas interactions**: chemical reactions, combustion chains, explosive gas mixtures
- **Pleb social system**: relationships, mood, mental breaks

### Tier 3 — Long Term
- **Modding support**: data-driven content loading, scripting API
- **Audio**: ambient soundscape driven by fluid velocity/density, UI sounds
- **Multiplayer**: lockstep deterministic simulation (research required — GPU fluid determinism is hard)

---

## Technical Architecture Notes

### Crate Structure

```
rayworld/
  crates/
    core/          # World model, simulation logic, tick management. No rendering deps.
    fluid/         # Fluid simulation: NS solver, gas fields, obstacle sync. Depends on core.
    render/        # wgpu setup, compute shaders, camera, overlays. Depends on core + fluid.
    game/          # Game loop, UI, input, ECS glue, save/load. Depends on all above.
    common/        # Shared types, math, config. No deps.
  assets/
    shaders/       # WGSL compute/render shaders
      fluid/       # Advection, pressure, vorticity, divergence, gradient, splat, boundary
      raytrace/    # Ray march, lighting, volumetric sampling
      post/        # Bloom, sunrays, overlays
    textures/      # Dithering noise, etc.
    voxels/        # Voxel model assets
  docs/            # This file, SPEC.md, design docs, fluid_mechanics/
  web/             # HTML shell, JS glue for WASM build
```

Note: `fluid/` is a separate crate from `core/` because the fluid simulation is a substantial, self-contained system with its own GPU pipeline. It reads obstacle state from `core` and writes density/temperature/velocity fields that `core` and `render` both consume.

### Simulation / Render Separation

The simulation (`core` + `fluid`) runs on a fixed tick rate, independent of frame rate. The renderer (`render`) interpolates between sim states for smooth visuals.

```
Game Loop:
  accumulator += frame_delta
  while accumulator >= TICK_DURATION:
    core.tick()          // block grid, pleb AI, jobs, needs
    fluid.tick()         // NS solve, advection, sources/sinks
    accumulator -= TICK_DURATION
  render(core.state, fluid.state, interpolation_factor)
```

The fluid sim can optionally run at a different tick rate than the game sim if needed (e.g., fluid at 120Hz, game at 60Hz) — the solver is stable at any timestep thanks to semi-Lagrangian advection.

### GPU Buffer Layout

The fluid sim uses GPU storage buffers (or textures) in ping-pong configuration:

```
Velocity A    (RG32F, sim_width x sim_height)
Velocity B    (RG32F, sim_width x sim_height)
Pressure A    (R32F,  sim_width x sim_height)
Pressure B    (R32F,  sim_width x sim_height)
Divergence    (R32F,  sim_width x sim_height)
Curl          (R32F,  sim_width x sim_height)
Temperature A (R32F,  sim_width x sim_height)
Temperature B (R32F,  sim_width x sim_height)
Obstacles     (R8,    sim_width x sim_height)  — from block grid

Per gas type:
  Density A   (R32F,  density_width x density_height)  — can be higher res
  Density B   (R32F,  density_width x density_height)
```

Memory at 128x128 sim, 512x512 density, 3 gas types:
- Velocity: 2 x 128x128 x 8 bytes = 256 KB
- Pressure: 2 x 128x128 x 4 bytes = 128 KB
- Divergence + Curl: 2 x 128x128 x 4 bytes = 128 KB
- Temperature: 2 x 128x128 x 4 bytes = 128 KB
- Obstacles: 128x128 x 1 byte = 16 KB
- Density (3 gases): 3 x 2 x 512x512 x 4 bytes = 6 MB
- **Total: ~7 MB** — trivially small

### Block Grid Data Layout

Struct-of-arrays (SoA) for cache-friendly iteration on CPU:

```rust
struct World {
    width: u32,
    height: u32,
    depth: u32,            // Z-layers, 1 for prototype
    terrain: Vec<TerrainType>,
    structure: Vec<StructureType>,
    height_map: Vec<u8>,   // block height for raytracing
    light: Vec<f32>,       // baked or per-frame lighting
}
```

The fluid fields live in GPU memory, not in the World struct. The World struct generates the obstacle texture that the fluid sim consumes.

### Ray Traversal

For the top-down raytracer, each pixel maps to a ray from the camera through the block grid. Use 2D DDA to step through tiles:

1. Project ray onto the 2D block grid
2. Step tile-by-tile using DDA
3. At each tile, check block height against ray height
4. On hit: shade pixel (material color * lighting)
5. **Along the ray, accumulate smoke density by sampling the fluid density texture** — this gives volumetric fog for free
6. Apply bloom/post-processing

---

## Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| WebGPU compute not supported in target browsers | Can't ship web version | Medium | Test early (Phase 0). Fallback: WebGL2 fragment-shader fluid sim (proven by PavelDoGreat). |
| Fluid sim too slow at 512x512 | Can't hit 60 ticks/sec in production | Low | Reduce pressure iterations, use coarser sim grid, GPU compute is fast. Profile at Phase 2. |
| Raytracer + fluid sampling too slow | Frame rate drops | Medium | Reduce density sample count per ray, lower density resolution, temporal reprojection. |
| 3D fluid extension too expensive | Can't do multi-story with fluid | Medium | Limit Z-layers (4-8), use 2D sim per floor with inter-floor coupling, multigrid solver. |
| Vorticity confinement looks wrong at game scale | Unrealistic fluid motion | Low | Extensive parameter tuning in Phase 2a. Compare against reference demos. |
| Scope creep on physics | Delays prototype | High | Hard constraint: no rigid body physics in prototype. Fluid sim is the physics. |
| Art style undefined | Inconsistent visuals | Medium | Define style guide before Phase 4 (when assets matter). |
| Single developer bottleneck | Slow progress | High | Prioritize ruthlessly. Ship Phase 6 before any post-prototype work. |

---

## Immediate Next Steps

1. `cargo init` the workspace with the crate structure above
2. Get a winit + wgpu window open with a colored quad (compute shader output)
3. Define the `World` struct in `crates/core`
4. Write the first WGSL compute shader: top-down ray march against a flat grid
5. Get the WASM build working in a browser
6. **Study PavelDoGreat's `script.js` in detail** — the entire fluid pipeline is in one file, MIT licensed. Map each shader to its WGSL equivalent.

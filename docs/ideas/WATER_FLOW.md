# Realistic Flowing Water

## The Vision

Water that flows downhill, pools in depressions, responds to player-built structures, and creates real decisions about where to settle and how to survive. Not decorative — functional. Finite volume that rain fills and drought drains.

## Current State

The game already has:
- **Water table** — height map generated during world gen, rendered as blue tint below terrain elevation
- **Elevation** — per-tile height in terrain_data
- **Fluid sim** — GPU Navier-Stokes for gas/smoke/temperature (256×256 velocity, 512×512 dye)
- **Obstacle texture** — 512×512 sub-cell blocking from thin walls
- **Thermal sim** — per-tile temperature

None of these model actual flowing water with volume. The water table is static — a paint layer, not a simulation.

## Approach: GPU Pipe Model

The pipe model is the standard for 2D water in colony sims (Dwarf Fortress, Oxygen Not Included, Cities: Skylines). It's physically accurate, trivially parallelizable on GPU, and naturally integrates with terrain and obstacles.

### Data

Two GPU textures at grid resolution (256×256):

```
water_tex:  R32Float   — water depth above terrain per cell (0 = dry)
flux_tex:   Rgba32Float — outflow rate in 4 directions (R=North, G=East, B=South, A=West)
```

Surface level at any cell = `terrain_height[cell] + water_height[cell]`

### Algorithm (GPU compute, per frame)

**Pass 1: Compute flux** (outflow from each cell)
```wgsl
// For each direction, accelerate/decelerate flow based on height difference
for dir in [N, E, S, W]:
    neighbor_surface = terrain[neighbor] + water[neighbor]
    my_surface = terrain[me] + water[me]
    delta = my_surface - neighbor_surface
    flux[dir] = max(0.0, old_flux[dir] + gravity * dt * delta)
    flux[dir] *= 0.998  // friction damping

// Scale total outflow to never exceed available water (volume conservation)
total_out = (flux.r + flux.g + flux.b + flux.a) * dt
if total_out > water[me]:
    flux *= water[me] / total_out
```

**Pass 2: Update water height** (apply fluxes)
```wgsl
// Inflow from all 4 neighbors - outflow from this cell
inflow = flux_from_north_neighbor.b   // their South outflow
       + flux_from_east_neighbor.a    // their West outflow
       + flux_from_south_neighbor.r   // their North outflow
       + flux_from_west_neighbor.g    // their East outflow

outflow = flux[me].r + flux[me].g + flux[me].b + flux[me].a

water[me] += (inflow - outflow) * dt
water[me] = max(0.0, water[me])
```

**Pass 3 (optional): Velocity field** (for visual effects)
```wgsl
// Average flux to get velocity at cell center
vel.x = (flux_west_in - flux_east_out + flux_east_in - flux_west_out) / (2.0 * avg_water)
vel.y = (flux_north_in - flux_south_out + flux_south_in - flux_north_out) / (2.0 * avg_water)
```

Total: **2-3 compute dispatches per frame**, ~0.1ms on modern GPU. Trivial.

### Why Pipe Model Over Alternatives

| Approach | Pros | Cons |
|----------|------|------|
| **Pipe model** | Simple, GPU-parallel, volume-conserving, handles obstacles naturally | No turbulence, simplified momentum |
| Shallow Water Equations | More physically accurate waves | Complex, numerical instability, overkill for colony sim |
| Cellular automaton | Very simple | No momentum, water "teleports", no smooth flow |
| Particle (SPH) | Most realistic | Extremely expensive, complex, 1000s of particles |
| Extend gas fluid sim | Reuses existing code | Gas sim has no gravity pooling, no free surface |

The pipe model gives 90% of the visual and gameplay realism at 10% of the complexity.

## Rendering

### Water Surface (in raytrace.wgsl)

```
if water_height > 0.01:
    depth = water_height

    // Color: deeper = darker blue, more opaque
    water_color = mix(vec3(0.3, 0.5, 0.7), vec3(0.05, 0.15, 0.35), clamp(depth * 2.0, 0, 1))
    water_alpha = clamp(depth * 3.0, 0.3, 0.9)

    // Caustics: animated noise ripple
    caustic = sin(world_x * 8.0 + time * 2.0) * sin(world_y * 8.0 + time * 1.7) * 0.1

    // Flow lines: directional streaks based on velocity
    flow_dir = normalize(velocity)
    flow_phase = dot(vec2(world_x, world_y), flow_dir) * 4.0 + time * 3.0
    flow_line = sin(flow_phase) * 0.05 * length(velocity)

    // Shore foam: white fringe at water edge
    if depth < 0.1:
        foam = (1.0 - depth / 0.1) * 0.3
        water_color = mix(water_color, vec3(0.9), foam)

    // Blend onto terrain
    color = mix(color, water_color + caustic + flow_line, water_alpha)
```

### Visual Features
- **Depth shading**: shallow = light blue/translucent, deep = dark blue/opaque
- **Caustics**: animated sine-based ripple pattern
- **Flow direction**: faint streaks in the direction of water velocity
- **Shore foam**: white fringe where water meets land
- **Sky reflection**: water tinted by sky color (sun angle, day/night)
- **Transparency**: underwater terrain visible through shallow water

## Gameplay Integration

### Player Structures

| Structure | Effect on Water |
|-----------|----------------|
| **Wall/dam** | Blocks water flow (flux = 0 through wall cells) |
| **Channel** | Dug trench (lower terrain) → water flows through |
| **Well** | Extracts water from underground (reduces local water_height) |
| **Pump** | Moves water uphill (pipe system integration) |
| **Water wheel** | Generates power from flowing water velocity |
| **Irrigation ditch** | Channels water to farm zones (crop yield bonus) |
| **Moat** | Defensive water channel, slows/blocks enemies |
| **Bridge** | Walkable over water (thin wall at elevation) |

### Weather

```
Rain:     water_height += rain_rate * dt  (uniform across map, scaled by weather intensity)
Snow:     water frozen (no flow), melts → water when temperature rises
Drought:  evaporation_rate increases with temperature → water shrinks
Flood:    heavy rain + poor drainage = colony flooding
```

The existing weather system drives rain. The thermal sim drives evaporation. Water connects them into a cycle:

```
Rain → Water accumulates → Flows downhill → Pools → Evaporates (heat) → Repeat
```

### Colony Survival

- **Drinking water**: Colonists need water. Well placement matters. Running out = dehydration crisis.
- **Irrigation**: Farms near water grow faster. Channels bring water to distant fields.
- **Fire fighting**: Water source near buildings enables bucket brigades.
- **Flooding**: Building in a depression without drainage = disaster during heavy rain.
- **Drought**: Water reserves matter. Ponds shrink. Wells go dry.

### Creature Interaction

- Duskweavers avoid water (can't swim — blocks their pathfinding)
- Thermogasts are drawn to warm water (hot springs)
- Water creates natural defensive barriers

## Terrain Interaction

### Digging

When a player digs terrain (lowers elevation), water flows into the hole:
```
Before: terrain=5, water=0, neighbor_terrain=3, neighbor_water=2
After dig: terrain=2, water=0 → water flows in from neighbors → pools
```

This is automatic — no special code. The pipe model handles it because the surface level changed.

### Erosion (Long-Term)

Over game-weeks, flowing water erodes soft terrain:
```
erosion_rate = water_velocity * terrain_softness * dt_slow
terrain_height -= erosion_rate
// Deposited downstream (sedimentation)
```

This creates natural rivers over time. Can expose buried mineral deposits (connects to DN-010).

## Finite Water

**Water has volume.** This is the key design decision.

- Rain adds water. Evaporation removes it. Pumps move it. Digging redirects it.
- If you drain a pond for irrigation, it stays drained until rain refills it.
- Seasonal cycle: wet season fills reserves, dry season tests them.
- Where you settle relative to water sources is a permanent, meaningful choice.

From PHILOSOPHY.md: "The things you can't build." You can't build a river. You can build around one.

## Implementation Phases

## Rain, Pooling, and the Water Cycle

### Rain → Ground → Flow → Pool → Evaporate

Rain isn't decoration. It's the primary water input. Every raindrop adds volume.

```
Each frame during rain:
    for each tile:
        water_height[tile] += rain_rate * dt

        // Terrain absorption: permeable soil absorbs some rain before pooling
        let absorption = terrain_permeability[tile] * dt  // clay=low, sand=high, rock=zero
        water_height[tile] = max(0.0, water_height[tile] - absorption)
```

The pipe model then moves this water downhill automatically. No extra code — the flux computation handles it. Rain falls everywhere, water flows to the lowest points, pools accumulate.

### Rain Intensity Levels

The existing `WeatherState` drives rain rate:

| Weather | rain_rate (tiles/sec) | Effect |
|---------|----------------------|--------|
| Clear | 0.0 | Evaporation only |
| Cloudy | 0.0 | No rain, reduced evaporation |
| Light Rain | 0.0002 | Gentle accumulation, puddles in low spots |
| Heavy Rain | 0.001 | Fast accumulation, flooding risk |

At heavy rain, a 1-tile-deep depression fills in ~17 minutes of game time. A flat plain gets a thin sheet of water everywhere, which flows toward any depression.

### Pooling Mechanics

Pools form automatically from the pipe model — water flows downhill and stops when it reaches a depression with no lower outlet. Key behaviors:

**Natural pools**: Low points in the elevation map collect water. The deeper the depression, the larger the pool. Terrain generation can seed natural ponds at elevation minima.

**Player-created pools**: Dig a hole → water flows in from surrounding terrain. Dig a channel to a river → fills with water. Build a dam across a slope → pool forms behind it.

**Overflow**: When a pool rises above the lowest rim, it spills over and flows downhill. This is automatic from the pipe model — flux resumes when surface level exceeds the neighbor.

**Saturation**: After prolonged rain, the ground saturates. Absorption drops to zero. Water sheets across the surface. Low areas flood first. The colony's drainage infrastructure (or lack of it) determines who floods.

### Terrain Permeability

Different terrain types absorb water at different rates before it pools:

| Terrain | Permeability | Notes |
|---------|-------------|-------|
| Sandy soil | High (0.003) | Rain soaks in fast, rarely pools |
| Grassland | Medium (0.001) | Some pooling in heavy rain |
| Clay | Low (0.0003) | Water sits on surface, pools quickly |
| Rock | Zero (0.0) | All rain runs off immediately |
| Peat/marsh | Very low (0.0001) | Already saturated, pools immediately |
| Tilled farm | Medium (0.001) | Loosened soil absorbs well |
| Floor (built) | Zero (0.0) | Impermeable — must drain actively |

Rocky terrain near mountains creates fast runoff → flash floods downhill. Clay plains become muddy pools. Sandy areas absorb everything. Built floors (stone, wood) are impermeable — a roofless room fills with water in rain.

### Puddles and Wet Ground

Before water reaches visible pooling depth, the ground gets wet:

```wgsl
// In terrain rendering (raytrace.wgsl):
let moisture = min(water_height * 20.0, 1.0);  // 0 = dry, 1 = saturated
if moisture > 0.01 {
    // Darken terrain (wet look)
    color *= 1.0 - moisture * 0.25;
    // Slight glossy reflection (specular on wet ground)
    let wet_spec = pow(max(dot(normal, sun_half), 0.0), 16.0) * moisture * 0.15;
    color += sun_color * wet_spec;
}
```

After rain stops, moisture lingers — water_height slowly drops from evaporation and absorption, so the ground stays dark/wet for a while before drying. Puddles shrink from the edges. This is free from the pipe model.

### Evaporation

Water leaves the system through evaporation, driven by the thermal simulation:

```wgsl
// In water flow compute shader:
let temp = block_temps[tile];
let evap_rate = max(0.0, (temp - 5.0) * 0.00001);  // faster above 5°C
let wind_factor = 1.0 + wind_speed * 0.1;  // wind accelerates evaporation
water_height[tile] -= evap_rate * wind_factor * dt;
water_height[tile] = max(0.0, water_height[tile]);
```

Hot day + wind = fast evaporation. Cold night = almost none. This creates the seasonal cycle: wet season fills ponds, dry season drains them. A colony that doesn't store water faces drought.

### Flooding

When rain exceeds drainage capacity, tiles flood. Flooding is just "water_height > threshold on a tile that shouldn't have water":

**Detection**: any tile with water_height > 0.1 that has a building/floor/wall → flooding event.

**Effects**:
- Items on flooded tiles get damaged/destroyed
- Crops drown (water_height > 0.3 for > 1 hour kills crops)
- Colonists slowed in shallow water, can't traverse deep water
- Electrical systems short out (wire + water = danger)
- Campfires extinguished

**Prevention**:
- Drainage channels around colony perimeter
- Build on high ground
- Roofs keep rain off interiors (existing roof system)
- Pumps remove floodwater

**Recovery**: water drains naturally once rain stops (flows to lower ground). Mopping up remaining puddles requires bucket/pump. Damaged items need repair.

### Rain Visual Effects (raytrace.wgsl)

```wgsl
if camera.rain_intensity > 0.01 {
    // Splashes on water surfaces
    if water_depth > 0.02 {
        let splash_seed = floor(world_x * 3.0) * 127.1 + floor(world_y * 3.0) * 311.7;
        let splash_phase = fract(camera.time * 4.0 + fract(sin(splash_seed) * 43758.5));
        if splash_phase < 0.1 {
            // Brief white ring at random positions
            let ring = abs(splash_phase * 10.0 - 0.5);
            if ring > 0.3 && ring < 0.5 {
                color = mix(color, vec3(0.8), 0.3 * camera.rain_intensity);
            }
        }
    }

    // Ripple rings on water (concentric circles from rain drops)
    let rain_ripple = sin(length(vec2(
        fract(world_x * 2.0) - 0.5,
        fract(world_y * 2.0) - 0.5
    )) * 30.0 - camera.time * 8.0);
    if water_depth > 0.01 {
        color += vec3(0.03) * max(rain_ripple, 0.0) * camera.rain_intensity;
    }
}
```

Rain on water: expanding ripple rings from splash points. Rain on ground: darkening (moisture). Rain on roofs: no water accumulation (existing roof system provides cover).

### The Full Water Cycle in One Day

```
Dawn:     Morning dew (light moisture on ground). Ponds at normal level.
Morning:  Sun rises, evaporation begins. Puddles from yesterday's rain shrink.
Noon:     Peak evaporation. Ground dries. Shallow pools disappear.
Afternoon: Clouds build (weather system). Evaporation slows.
Evening:  Rain begins. Water accumulates. Channels fill. Pools rise.
Night:    Heavy rain. Low areas flood. Colonists shelter indoors.
Midnight: Rain peaks. Colony drainage tested. Overflow into buildings?
Pre-dawn: Rain eases. Flood waters begin receding. Damage assessment.
```

Every part of this cycle emerges from the pipe model + rain input + evaporation output. No scripted events — pure simulation.

### Phase 1: Static → Dynamic Water
- Create `water_tex` (R32Float, 256×256) initialized from existing water table
- Create `flux_tex` (Rgba32Float, 256×256) initialized to zero
- Compute shader: 2 passes (flux + update) per frame
- Obstacle integration: walls block flux (check wall_data + block grid)
- Render: replace static water table rendering with dynamic water_height sampling
- **Files**: gpu_init.rs, main.rs, new shader `water_flow.wgsl`, raytrace.wgsl

### Phase 2: Interaction
- Digging lowers terrain → water responds automatically
- Wall placement blocks water flux
- Well block type: slowly reduces local water_height (extraction)
- Pathfinding: water cells are unwalkable (depth > 0.3)
- **Files**: placement.rs, pleb.rs, simulation.rs

### Phase 3: Weather
- Rain adds water uniformly (weather.rs drives rate)
- Evaporation removes water based on thermal sim temperature
- Seasonal variation: rain_rate modulated by time of year
- **Files**: simulation.rs, water_flow.wgsl

### Phase 4: Visual Polish
- Shore foam, caustics, flow direction lines
- Depth-based opacity and color
- Underwater terrain visible through shallow water
- Waterfall rendering at height discontinuities
- **Files**: raytrace.wgsl

### Phase 5: Advanced Gameplay
- Irrigation zones (farm yield bonus near water)
- Water wheel power generation
- Moat as defensive structure
- Bucket brigade fire fighting
- Erosion over long timescales

## Performance Budget

| Resource | Size | Notes |
|----------|------|-------|
| water_tex | 256KB | R32Float, 256×256 |
| flux_tex | 1MB | Rgba32Float, 256×256 |
| Compute | 2-3 dispatches/frame | 32×32 workgroups, ~0.1ms |
| Total GPU memory | ~1.3MB | Trivial |

Can run at half-rate (every other frame) for 50% compute savings with no visible difference — water flows slowly enough that 30Hz updates look identical to 60Hz.

## Water Shader — Making It Look Real

Water from above reads as water through layered visual cues. Each one is cheap (a few sine calls), and only runs on water pixels (~20% of screen). Priority order — start with 1-3 and it already looks great.

### 1. Depth Transparency (Essential — This IS Water)

Beer's law absorption: shallow water shows the terrain through it, deep water is opaque. The single most important effect.

```wgsl
let visibility = exp(-depth * 3.0);  // 0 at depth=∞, 1 at depth=0
let underwater_terrain = terrain_color * visibility;
let surface_color = depth_tint(depth);  // shallow=blue-green, deep=dark blue
color = mix(underwater_terrain, surface_color, 1.0 - visibility);
```

Color palette varies by depth AND time of day:
- Shallow: `(0.35, 0.55, 0.50)` — blue-green, terrain visible
- Medium: `(0.15, 0.30, 0.50)` — blue
- Deep: `(0.04, 0.08, 0.18)` — near black
- Dawn/dusk: warm orange-gold sky reflection on surface
- Night: very dark, only torch reflections

### 2. Shore Edge (Essential — Defines the Boundary)

The transition from water to land needs wet darkening + animated foam:

```wgsl
if depth < 0.15 {
    // Wet sand: darken terrain near water
    let wet = 1.0 - depth / 0.15;
    terrain_color *= 1.0 - wet * 0.3;

    // Foam line: wavy white fringe
    let foam_wave = sin(world_x * 12.0 + time * 2.0) * 0.02;
    let foam_center = 0.05 + foam_wave;
    let foam = smoothstep(0.0, 0.03, depth - foam_center + 0.03)
             * smoothstep(0.0, 0.03, foam_center + 0.03 - depth);
    color = mix(color, vec3(0.9, 0.92, 0.95), foam * 0.4);
}
```

The foam line undulates with a sine wave, creating the organic look of waves lapping at the shore.

### 3. Caustics (High Impact — "This Water Moves")

The dancing light pattern on the bottom — the classic pool-floor effect. Multiple overlapping sine waves at different angles:

```wgsl
fn caustics(wx: f32, wy: f32, t: f32) -> f32 {
    var c = 0.0;
    for (var i = 0; i < 3; i++) {
        let fi = f32(i);
        let freq = 5.0 + fi * 3.5;
        let dir = vec2(cos(fi * 2.1 + 0.5), sin(fi * 2.1 + 0.5));
        let phase = dot(vec2(wx, wy), dir) * freq + t * (1.5 + fi * 0.4);
        c += sin(phase);
    }
    return pow(max(c / 3.0, 0.0), 2.0);  // sharpen peaks
}

// Applied to underwater terrain:
underwater_terrain += caustic * sun_intensity * visibility * 0.25;
```

9 sine calls total. Only runs on water pixels, only during daytime. Creates beautiful animated light patterns that respond to the sun.

### 4. Specular Sun Highlight (Surface Glint)

Bright spots where sun reflects off the rippled surface:

```wgsl
// Surface normal from ripple gradients
let nx = sin(wx * 5.0 + time * 1.3) * cos(wy * 4.0 + time * 0.9) * 0.3;
let ny = cos(wx * 4.0 - time * 1.1) * sin(wy * 6.0 + time * 1.4) * 0.3;

// Specular: how aligned is reflected sun with camera (looking down)?
let reflect_z = 1.0 - nx * nx - ny * ny;  // reflected ray Z component
let sun_contrib = nx * sun_dir_x + ny * sun_dir_y;  // alignment with sun
let spec = pow(max(reflect_z - sun_contrib * 0.3, 0.0), 32.0);

color += sun_color * spec * sun_intensity * 0.5;
```

Creates moving bright sparkles on the water surface. Gorgeous at low sun angles (dawn/dusk), subtle at noon (sun straight down).

### 5. Ripple Refraction (Underwater Wobble)

The terrain visible through shallow water appears distorted by the surface ripples:

```wgsl
let ripple_x = sin(wx * 3.0 + time * 1.2) * sin(wy * 4.0 + time * 0.8);
let ripple_y = cos(wx * 5.0 - time * 1.5) * sin(wy * 3.0 + time * 1.1);
let distort = vec2(ripple_x, ripple_y) * 0.02 * visibility;  // less in deep water

// Sample terrain at distorted position
let refracted_terrain = sample_terrain(wx + distort.x, wy + distort.y);
```

The underwater world gently wobbles. Stronger in shallows (where you can see the bottom), vanishes in deep water.

### 6. Flow Direction (Streaks)

When the pipe model provides a velocity field, directional streaks show where water is going:

```wgsl
let flow_dir = normalize(velocity);
let flow_speed = length(velocity);

// Directional noise aligned to flow
let along = dot(vec2(wx, wy), flow_dir) * 6.0 + time * flow_speed * 4.0;
let streak = pow(sin(along) * 0.5 + 0.5, 6.0);  // sharp bright lines

// Only visible in moving water
let flow_vis = smoothstep(0.1, 0.5, flow_speed);
color += vec3(0.15) * streak * flow_vis;

// Foam in fast-flowing areas
let foam = smoothstep(0.8, 2.0, flow_speed) * 0.3;
let foam_noise = fract(sin(wx * 127.1 + wy * 311.7 + time * 5.0) * 43758.5);
color = mix(color, vec3(0.85), foam * foam_noise);
```

Still water: no streaks. Gentle flow: faint directional lines. Fast current: bright streaks + foam.

### 7. Night Torch Reflection (Atmospheric)

Pleb torches/headlights reflect off nearby water:

```wgsl
// For each nearby pleb with torch:
let dist = length(vec2(wx - pleb.x, wy - pleb.y));
if dist < 4.0 && water_depth > 0.02 {
    let ripple = sin(dist * 8.0 - time * 3.0) * 0.3 + 0.7;  // concentric ripples
    let atten = 1.0 / (1.0 + dist * 0.5 + dist * dist * 0.1);
    let reflection = torch_color * atten * ripple * 0.3;
    color += reflection;
}
```

A warm glow spreading across the water surface near torches, broken up by ripple rings. Beautiful at night. The existing torch light code already provides position and color — just extend it to tint water surfaces.

### The Complete Pixel Pipeline

```
if water_depth > 0.005:
    1. Sample terrain at refraction-distorted position
    2. Add caustics (sun × visibility)
    3. Apply Beer's law absorption
    4. Compute surface color (depth + sky reflection)
    5. Add specular sun highlight
    6. Add flow streaks (if velocity > 0)
    7. Composite underwater + surface
    8. Add shore foam at edges
    9. Add torch reflections at night
```

~30-40 GPU instructions per water pixel. Comparable to tree rendering. Skip steps 2+5 at night. Skip step 6 without velocity data (Phase 1). Each step is independent and can be tuned or disabled.

### Day/Night Water Color Cycle

```
Noon:     Sky-blue reflection, bright caustics, strong specular
Afternoon: Warm gold creeping into reflection, caustics dimming
Sunset:   Orange-gold surface, deep blue shadows, dramatic specular
Dusk:     Dark blue-grey, faint last caustics
Night:    Near-black, only torch reflections, moonlight shimmer
Dawn:     Pink-gold on surface, caustics returning, mist wisps
```

The existing sun color/intensity/direction data drives all of this. No new state needed — just sample `camera.sun_color_r/g/b`, `camera.sun_intensity`, `camera.sun_dir_x/y`, `camera.sun_elevation`.

## Deeper Angles

### Groundwater — The Hidden Layer

The water table already exists as a height map. Surface water and groundwater should connect:

- **Rain → absorption → water table rises.** Permeable soil passes water downward. Heavy rain raises the local water table over days.
- **Springs**: Where the water table intersects terrain surface, water emerges naturally. These are permanent water sources that flow from the ground without rain. Springs appear at the base of hills/cliffs where underground water meets the surface. Players discover springs by exploring (connects to DN-010).
- **Wells tap groundwater, not surface water.** A well's yield depends on the water table depth at that tile. Shallow table = productive well. Deep table = slow or dry.
- **Dig below water table → flooding from below.** Digging a mine shaft or basement below the water table causes water to seep in from the sides. Different from surface flooding — slower but relentless. Pumps needed to keep the mine dry.
- **Marshland**: Where the water table sits at the surface permanently. Ground is always wet, always slow, always muddy. Building here requires raised foundations.

Two-layer model: surface water (pipe model, visible, fast) + groundwater (water table, invisible, slow, wells access it). Rain connects them: surface → absorption → groundwater → springs → surface.

### Ice and Freezing

When the thermal sim reads below 0°C at a water tile:

- **Water freezes.** Flux drops to zero. Ice is a solid surface — walkable! Colonists can cross frozen rivers in winter.
- **Ice is slippery.** Movement speed bonus on ice but a chance of stumble/fall (brief stagger animation).
- **Ice harvesting.** Colonists can cut ice blocks for drinking water in winter when wells freeze over.
- **Spring thaw.** Temperature rises → ice melts over hours → sudden water release. All the winter snow/ice becomes water at once → spring flooding. The colony's drainage must handle the surge.
- **Pipes freeze.** Exposed pipes (existing pipe system) in cold areas freeze and potentially burst. Insulated pipes (BT_INSULATED) resist this. Burst pipes spray water until repaired.
- **Visual**: Frozen water renders as pale blue-white semi-reflective surface. Cracking patterns as temperature rises above -2°C. Melt puddles forming at edges.

### Mud and Terrain Deformation

Wet terrain changes properties:

- **Mud.** Wet soil below a threshold becomes mud. Movement speed -40%. Building speed -20%. Visible as darker, shinier terrain.
- **Tracks.** Colonists walking through mud leave footprints (terrain compaction system already exists). Heavy items dragged through mud leave trails. Tracks accumulate into paths — paths dry faster (compacted = less absorption = water runs off).
- **Drying.** Mud dries back to solid when water_height drops below threshold. Takes time — a muddy field stays muddy for hours after rain stops.
- **Quicksand.** Deep saturated sand with high water content. Rare terrain hazard near certain water features. Colonists that walk into it get stuck (need rescue). Detectable by terrain color — slightly different than normal sand.

### Water as Weapon and Obstacle

**Defensive:**
- **Moat.** Dig a trench around the colony, divert water into it. Enemies can't cross deep water. A drawbridge (future door variant) lets friendlies pass.
- **Flood trap.** Build a dam, fill the reservoir. When enemies approach, destroy the dam → flash flood. Water damage + movement denial + drama.
- **Electric moat.** Wire + water = anything in the pool takes electrical damage. Requires power and wire placed adjacent to water. Terrifying.

**Hazards:**
- **Drowning.** Colonists/enemies in water deeper than 0.5 tiles take damage over time. Can swim briefly but fatigue sets in. Deep water is lethal without a bridge.
- **Wading.** Water depth 0.1-0.5: movement at 50% speed. Attacks -20% accuracy (footing). Crossing a river under fire is suicide.
- **Current.** Fast-flowing water pushes entities downstream. A colonist crossing a fast river drifts sideways. Fall into a strong current → swept away.

### Water Sound

The existing GPU sound propagation models water acoustically:

- **Flowing water.** Fast-flow tiles inject continuous sound (sine pattern, low frequency). Sound carries through the map. You hear a river before you see it.
- **Rain.** During rain, distributed sound sources across the map. The sound sim carries it physically — rain on a roof sounds different from rain on open ground (the roof blocks direct sound, you hear it filtered through walls).
- **Waterfall.** Where terrain drops sharply with flowing water — loud broadband noise. The sound carries far and reflects off cliffs.
- **Dripping.** In buildings with damaged/missing roofs during rain: periodic impulse sound sources. The classic "drip... drip..." that tells you the roof leaks.
- **Splashing.** When a colonist or physics body enters water — impulse sound. When walking through shallow water — periodic splash sounds.

All of these use the existing `SoundSource` system + GPU wave equation. No new sound infrastructure.

### Water Color from Environment

Water takes on the character of its surroundings:

| Bottom terrain | Water appearance |
|---------------|-----------------|
| Sand | Clear blue-green, visible bottom |
| Clay | Murky brown-yellow |
| Rock | Crystal clear, deep blue |
| Peat/marsh | Dark brown-green |
| Built floor | Sharp-edged, urban look |

Additional color effects:
- **Stagnant water** (zero velocity for >1 day): green algae tint that builds slowly
- **Blood** (combat near water): red tint that advects with flow and dissipates over minutes
- **Silt** (erosion products): brown turbidity in fast-flowing water near eroded terrain
- **Thermal**: Hot water (near campfire/kiln) shimmers more. Cold water is clearer.

The dye texture from the fluid sim could carry water color information — pigment advected by the velocity field. Blood dropped in a river would flow downstream, spread, and fade. This uses the same Navier-Stokes sim already running for gas.

### Wind and Waves

The existing wind system (direction + magnitude from weather) affects water surface:

- **Wind fetch.** Large open water bodies develop waves proportional to wind speed × distance across the water (fetch). A small pond barely ripples. A large lake has real waves.
- **Directional waves.** Waves align with wind direction. Windward shores get wave impact (foam, spray). Leeward shores are calm.
- **Storm waves.** During heavy rain + high wind, waves can breach low barriers. A moat wall that's safe in calm weather overflows in a storm.
- **Visual**: Wave amplitude modulates the ripple shader. Calm water = gentle sine ripple. Storm = large amplitude, shorter wavelength, foam everywhere.

```wgsl
let fetch = estimate_fetch(world_x, world_y, wind_dir);  // distance to shore upwind
let wave_amplitude = wind_speed * 0.01 * min(fetch, 10.0);  // bigger with more fetch
let wave = sin(dot(pos, wind_dir) * 4.0 - time * 3.0) * wave_amplitude;
```

### Ecological Impact

Water shapes the living world:

- **Vegetation.** Tiles near water (within 3 tiles) get a growth bonus. Trees grow taller. Grass is greener. Visible in the terrain detail shader.
- **Crops.** Farms near water or with irrigation channels yield more. Farms far from water need manual watering (colonist activity) or crops wilt.
- **Animals.** Duskweavers avoid water (can't swim). But prey animals (future) gather at water to drink — hunting ground.
- **Fish.** Water tiles with depth >0.3 can support fish. Fishing rod (craftable) + water = food source. Connects to the food progression system.
- **Algae.** Still water grows algae over days. Green tint. Reduces water quality (colonists won't drink it without filtering). Flowing water stays clean.
- **Reeds/cattails.** Shallow water edges grow reeds over time (visual + harvestable for fiber). The shore becomes visually alive.

### Mist, Dust, and Atmospheric Particles

The game already has a fluid sim that advects smoke, temperature, and gas. Water introduces three new atmospheric phenomena that plug directly into it.

#### Mist / Fog

Mist is water vapor in the air. It forms when conditions are right:

**Generation:**
- Water surfaces evaporate into the air → dye texture gets a "moisture" component
- Warm water + cold air = rapid mist (morning fog over a river)
- Rain aftermath: ground moisture evaporates as sun warms → ground fog
- Waterfalls: impact mist rises from the base, advected by wind

**The fluid sim carries mist physically.** It's just another dye channel — advected by velocity, diffused, blocked by walls. Mist from a river drifts downwind. Mist in a valley pools (cold air sinks, holds moisture). Morning sun burns off mist from the top down.

```
Mist generation rate:
    if water_height > 0.01 and air_temp > water_temp:
        mist_rate = (air_temp - water_temp) * 0.001 * water_surface_area

    // Inject into dye texture at water tile
    dye.mist += mist_rate * dt
```

**Rendering:** Mist is a semi-transparent white overlay with soft edges:
```wgsl
let mist_density = sample_dye(world_x, world_y).mist;
if mist_density > 0.01 {
    let mist_color = mix(vec3(0.7, 0.72, 0.75), vec3(0.85, 0.87, 0.9), sun_intensity);
    let mist_alpha = clamp(mist_density * 2.0, 0.0, 0.7);
    color = mix(color, mist_color, mist_alpha);
    // Mist scatters light: slight glow near light sources
    color += light_color * mist_density * 0.1;
}
```

**Gameplay effects:**
- Vision range reduced in mist (fog of war radius shrinks proportionally)
- Enemies can approach unseen through mist → ambush risk
- Sound still propagates (you hear what you can't see)
- Mist clings to valleys — high ground has better visibility
- Campfire/torch visible as a glow through mist (warm halo)

**The atmospheric moments:** Dawn over a river valley. Mist rises from the water, fills the low ground. Your colony on the hillside looks down into white. The torches of the night watch glow as fuzzy halos. Then the sun comes up and the mist burns away from east to west. This emerges from simulation, not scripting.

#### Dust

Dust is the dry counterpart to mist. It forms when there's no water:

**Generation:**
- Wind over dry terrain (water_height == 0, low humidity) kicks up dust
- Stronger wind = more dust. Threshold: wind > 5 tiles/sec on dry ground
- Digging/construction creates dust plumes (impulse into dye texture)
- Explosions create dust clouds
- Dry terrain types produce more dust: sand > clay > rock > grass

```
if water_height < 0.001 and wind_speed > 5.0:
    dust_rate = (wind_speed - 5.0) * terrain_dustiness * 0.002
    // Inject into fluid sim dye channel
    dye.dust += dust_rate * dt
```

**The fluid sim advects dust** just like smoke. Dust blows downwind, piles against walls, enters buildings through open doors, gets pushed by fans. A dust storm (weather event) injects massive amounts across the map.

**Rendering:** Dust is warm-tinted opacity:
```wgsl
let dust_density = sample_dye(world_x, world_y).dust;
if dust_density > 0.01 {
    let dust_color = vec3(0.65, 0.55, 0.40);  // sandy tan
    let dust_alpha = clamp(dust_density * 1.5, 0.0, 0.6);
    color = mix(color, dust_color, dust_alpha);
    // Dust in sunlight: warm glow (particles catch light)
    color += sun_color * dust_density * sun_intensity * 0.15;
}
```

**Gameplay effects:**
- Vision reduced (like mist, but warm-tinted)
- Breathing: colonists in heavy dust need face covering or take health damage (connects to breathing/O2 system)
- Equipment degrades faster in dusty environments
- Crops damaged by heavy dust
- Dust storm event: map-wide dust injection, everything slows down

**Western atmosphere:** Dust is THE visual motif of the frontier. A rider approaching the colony kicks up a dust trail. Wind sweeps dust across the plains. A gunfight raises dust with every bullet impact. The frontier isn't clean — it's gritty.

#### Steam

Where water meets extreme heat:

- Kiln/smelter near water → steam plume
- Lava (future) meeting water → massive steam
- Hot springs → perpetual gentle steam
- Pouring water on fire → steam burst (fire fighting)

Steam is just high-temperature mist. It rises (buoyant in the fluid sim — hot gas rises) and dissipates as it cools. Rendering: brighter than mist, more opaque, faster-moving.

```
if water_height > 0.01 and block_temp > 80.0:
    steam_rate = (block_temp - 80.0) * 0.005
    dye.mist += steam_rate * dt  // steam IS mist, just from heat
    dye.temp += steam_rate * 20.0  // carries heat upward
```

#### Dye Texture Channel Budget

The existing dye texture is Rgba32Float: R=smoke, G=O2, B=CO2, A=temperature. To add mist and dust without a new texture, options:

1. **Pack into existing channels.** Smoke (R) could carry both smoke AND dust (they're visually similar — particles in air). Distinguish by temperature: hot smoke vs cold dust.

2. **Second dye texture.** A new Rgba32Float texture for water-related advection: R=mist, G=dust, B=dissolved_color (blood/silt), A=moisture. Same resolution (512×512), same advection shader, negligible cost.

3. **Repurpose.** If O2/CO2 aren't heavily used, pack mist into one.

Option 2 is cleanest — a dedicated atmospheric particle texture. The advection shader already exists and handles ping-pong; adding a second texture to the same dispatch costs almost nothing.

#### The Unified Atmosphere

All of these — smoke, mist, dust, steam — are particles in air carried by the same velocity field:

```
         ┌──────────┐
Wind ───→│          │
         │  Fluid   │──→ Advect all particles together
Rain ───→│  Sim     │    (same velocity field)
Heat ───→│          │
         └──────────┘
              ↓
    ┌────┬────┬────┬────┐
    │Smoke│Mist│Dust│Steam│
    └────┴────┴────┴────┘
    Fire  Water Dry  Heat+
                     Water
```

They interact:
- Rain suppresses dust (water + dust → mud, dust_density drops in rain)
- Fire near water creates steam (replaces mist with hotter, denser version)
- Wind carries all of them in the same direction
- Walls block all of them equally (existing obstacle texture)
- Fans push all of them (existing fan block interaction)

The visual result: a living atmosphere that responds to everything happening in the game. A colony after a rainstorm: mist rising from puddles, steam from the cooling campfire, dust settling as the ground gets wet. A colony during drought: dust everywhere, dry cracked ground, no mist, air shimmers with heat.

### Historical Water Features

From PHILOSOPHY.md — "the map as memory":

- **Dry riverbed.** A winding depression with smooth stone bottom. Visible in terrain generation. Was a river before the climate shifted. Build your well here — the water table is close.
- **Flood plain.** Flat terrain with rich soil deposits from ancient floods. Great farmland but will flood again in heavy rain.
- **Ancient dam.** Stone structure spanning a valley (discoverable ruin from DN-010). Partially broken. Repair it → instant reservoir.
- **Fossil spring.** A dried-up spring marked by mineral deposits. Dig here → tap the underground water source. The planet's geology tells you where water was.

## Connection to Other Systems

| System | Integration |
|--------|-----------|
| **Terrain/elevation** | Surface = terrain + water_height. Digging redirects water. |
| **Thermal sim** | Temperature drives evaporation rate. Hot springs. |
| **Weather** | Rain adds water volume. Drought reduces it. |
| **Fluid sim (gas)** | Water surface blocks gas flow. Steam from hot water. |
| **Creature system** | Water blocks creature pathfinding. Thermogasts seek warm water. |
| **Building** | Dams block flow. Channels direct it. Wells extract. Pumps transport. |
| **Discovery layer** | Erosion exposes buried deposits over time. |
| **Needs system** | Colonists need drinking water. Dehydration if no access. |
| **Agriculture** | Irrigation boosts crop yield. Drought kills crops. |

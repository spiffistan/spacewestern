# Tall Grass — Concealment, Fire Fuel, and the Frontier Perimeter

## The Idea

Grass that grows tall enough to hide in. Not decoration — a gameplay material that creates decisions: clear it for safety, leave it for ambush potential, burn it to deny cover, harvest it for fiber.

## What It Does

1. **Obscures visibility** — entities inside tall grass are hidden from outside; vision rays degrade passing through it
2. **Slows movement** — wading through grass is slower than open ground
3. **Grows back** — cut grass returns over days on suitable terrain with moisture
4. **Burns fast** — tall grass is the most flammable thing on the map; wildfire races through it
5. **Yields fiber** — cutting grass produces crafting material (ties into early-game fiber loop)
6. **Shelters creatures** — duskweavers stalk through it, glintcrawlers nest in it
7. **Enables stealth** — drafted colonists in grass are hidden; enemies approach unseen

## Data Model

Simplest approach: per-tile `grass_height` as a float (0.0 = bare, 1.0 = full tall grass). Could pack into terrain_data spare bits (quantized to 4 bits = 16 levels) or a separate lightweight buffer.

```rust
// Option A: pack into terrain_data (bits 20-23 are available)
let grass_level = (terrain_data[idx] >> 20) & 0xF;  // 0-15 → 0.0-1.0

// Option B: separate buffer
grass_height: Vec<u8>,  // 256×256 = 64KB, 0-255 mapped to 0.0-1.0
```

## Growth

Grass grows on suitable terrain when conditions are right:

```
Each game-day tick:
    for each tile:
        if terrain is GRASS, LOAM, or PEAT:
            if moisture > 0.2 (from water system):
                grass_height += 0.1   // ~10 days to full height
                grass_height = min(grass_height, 1.0)

        // Rocky, sandy, clay, built floors: no growth
        // Near water: 1.5× growth rate
        // Shaded by trees: 0.7× growth rate (less sun)
```

**Seasonal**: in a future seasonal system, grass grows in spring/summer, browns in autumn, dies in winter. Regrows from stubble in spring.

## Cutting

A work task (manual or zone-based):

- Colonist walks to grass tile, cuts for ~2 seconds
- Grass_height drops to 0.05 (stubble)
- Yields 1-3 fiber (existing item)
- Grows back from stubble faster than from bare (root system intact)

A "clear zone" (like growing/storage zones) designates an area that colonists keep mowed. The colony perimeter becomes a maintained cleared strip.

## Fire

Tall grass is tinder:

```
if grass_height > 0.3 and fire_adjacent:
    ignition_chance = grass_height * 0.8   // catches easily
    fire_spread_rate = grass_height * 5.0  // spreads FAST
    burn_time = grass_height * 0.3         // burns out quickly (thin fuel)
```

A wildfire through a grass field moves 3-5× faster than through buildings. The fire front is visible — a line of flame racing downwind. Anything in its path takes damage.

**Controlled burns**: Deliberately set fire to clear a field. The frontier technique. Light the upwind edge, the fire sweeps downwind and burns out. Clear a huge area in minutes. But: wind shifts mid-burn → fire turns toward your colony.

**Fire breaks**: Cleared paths stop grass fires. A 2-tile-wide cleared strip around the colony is basic fire safety. If you didn't clear it... one lightning strike and the colony is surrounded by flame.

## Visibility and Stealth

### Fog of War Integration (fog.rs)

Vision rays passing through tall grass tiles lose intensity:

```rust
// In shadowcast visibility computation:
if grass_height_at(ray_x, ray_y) > 0.4 {
    let density = grass_height * 0.6;  // 60% opacity at full height
    visibility *= 1.0 - density;
    if visibility < 0.05 { break; }
}
```

Effect: you can see 1 tile into grass dimly, 2 tiles barely, 3+ tiles not at all. A field of tall grass is opaque at distance.

### Entity Concealment

An entity (pleb, enemy, creature) standing in tall grass is hidden:

```rust
// When checking if entity is visible:
let in_grass = grass_height_at(entity.x, entity.y) > 0.5;
if in_grass {
    // Only visible if observer is within 2 tiles
    if observer_distance > 2.0 {
        visible = false;
    }
}
```

Colonists in grass: name label hidden, body partially obscured in shader, not targetable by enemies beyond 2 tiles.

### Vision FROM Grass

An entity inside tall grass has reduced vision:

```rust
let vision_radius = if in_tall_grass {
    base_radius * 0.4  // can only see nearby
} else {
    base_radius
};
```

Hiding in grass is a tradeoff: you're hidden but nearly blind. You hear (sound sim) but can't see.

## Shader Rendering

### Grass Blades (raytrace.wgsl)

Procedural grass overlay on terrain tiles with grass_height > 0.1:

```wgsl
if grass_h > 0.1 {
    let wind_push = sin(world_x * 2.5 + camera.time * camera.wind_magnitude * 0.8)
                  * camera.wind_magnitude * 0.04;

    // Multiple blade layers at different frequencies
    var grass_alpha = 0.0;
    var grass_col = vec3(0.0);

    for (var layer = 0; layer < 3; layer++) {
        let freq = 3.0 + f32(layer) * 2.5;
        let phase = f32(layer) * 1.7;
        let blade_x = fract(world_x * freq + phase + wind_push * f32(layer + 1)) - 0.5;
        let blade_y = fract(world_y * freq + phase * 0.7) - 0.5;

        // Blade: thin angled line, swaying
        let sway = sin(camera.time * 1.5 + world_x * 3.0 + f32(layer)) * 0.15 * grass_h;
        let blade_dist = abs(blade_x - blade_y * sway);

        if blade_dist < 0.05 {
            let root_to_tip = blade_y + 0.5;  // 0=root, 1=tip
            let layer_col = mix(
                vec3(0.22, 0.32, 0.12),  // dark root
                vec3(0.50, 0.58, 0.25),  // bright tip
                root_to_tip
            );
            // Sun catch: blades leaning toward sun are brighter
            let sun_catch = max(0.0, sway * camera.sun_dir_x + 0.3) * camera.sun_intensity * 0.3;
            grass_col = mix(grass_col, layer_col + sun_catch, 0.5);
            grass_alpha = max(grass_alpha, grass_h * (1.0 - blade_dist / 0.05));
        }
    }

    color = mix(color, grass_col, grass_alpha * 0.7);
}
```

Three overlapping blade layers at different frequencies create a dense, organic look. Each blade:
- Sways with the existing wind system (direction + magnitude)
- Has root-to-tip color gradient (dark green → bright yellow-green)
- Catches sunlight when leaning toward the sun
- Density scales with grass_height

### Entity Occlusion

After rendering a pleb/creature body, if they're in tall grass, overlay grass on their lower body:

```wgsl
// After pleb body rendering:
if drew_pleb && grass_at_pleb > 0.5 {
    // Re-render grass blades over the lower portion of the pleb
    let occlusion_line = grass_at_pleb * 0.5;  // grass covers bottom 50% at full height
    if pleb_local_y > -occlusion_line {
        // Blend grass color over pleb's legs
        color = mix(color, grass_blade_color, 0.6);
    }
}
```

The pleb's legs disappear into the grass. Head and upper body still visible from above. Creates the visual of wading through a field.

### At Night

Tall grass at night is nearly invisible — just a slightly different darkness. Torchlight catches the blade tips:

```wgsl
if grass_h > 0.3 && torch_light > 0.01 {
    // Torch catches grass tips: warm orange on green
    let tip_catch = grass_alpha * torch_light * 0.3;
    color += vec3(0.4, 0.3, 0.1) * tip_catch;
}
```

A torch-bearing colonist walking through grass at night: the grass around them lights up in warm orange, flickering. Beyond the torch radius: black void. Something could be two tiles away and invisible.

## Creature Interaction

### Duskweavers Love Grass

From ALIEN_FAUNA.md: duskweavers are pack scavengers that stalk and steal. Tall grass is their preferred terrain:

```rust
// In duskweaver pathfinding cost:
let grass_bonus = if grass_height_at(x, y) > 0.5 { -3 } else { 0 };
// Negative cost = PREFER this tile
```

Duskweavers approach the colony through grass fields, invisible until they're close. Clearing grass around the perimeter forces them into the open where torch-bearing guards spot them.

### Glintcrawlers Nest in Grass

From ALIEN_FAUNA.md: glintcrawlers are ambush hazards in tall grass and rocky terrain. Grass above 0.7 height can spawn glintcrawler nests during world generation or over time. Walking into one without looking → warning crackle → bite.

### The Ecology

Clearing ALL grass around the colony makes you safe from grass-dwelling creatures but:
- No fiber harvest (need to go further for it)
- No ambush potential (can't hide your own troops)
- Fire doesn't stop at your cleared perimeter — it stops at the grass edge, which is now far away
- The cleared area looks barren (aesthetic)

The optimal strategy: maintain a cleared perimeter, but leave strategic grass patches for hunting, ambush positions, and fiber harvest. Manage, don't eliminate.

## The Perimeter Game

This creates a fundamental early-game loop:

```
Day 1:  Colony surrounded by tall grass. Can barely see beyond 3 tiles.
Day 3:  Start clearing a perimeter. Fiber stockpile grows.
Day 5:  Perimeter cleared to 5 tiles. Can see duskweavers approaching.
Day 8:  First grass fire (lightning or campfire spark).
        Uncleared side burns. Colony nearly catches fire.
Day 10: Full perimeter cleared. Fire break maintained.
Day 15: Grass growing back at the edges. Need to re-mow.
        Or: assign a clear zone and colonists maintain it automatically.
```

The grass isn't an enemy. It's the frontier. You push it back, it pushes in. You manage the boundary. That boundary IS your colony's edge — visible on the map as the line between manicured safety and wild unknown.

## Implementation Phases

### Phase 1: Data + Growth + Cutting
- Add grass_height to terrain_data (4 bits) or separate buffer
- Growth tick in simulation (terrain type + moisture check)
- Cut action (yields fiber, reduces height)
- Clear zone designation
- **Files**: grid.rs, simulation.rs, zones.rs, placement.rs

### Phase 2: Rendering
- Procedural grass blades in raytrace.wgsl
- Wind sway from existing wind system
- Sun-catch highlights
- Entity partial occlusion
- **Files**: raytrace.wgsl

### Phase 3: Visibility
- Fog of war: grass reduces vision ray intensity
- Entity concealment: hidden in grass beyond 2 tiles
- Reduced vision from inside grass
- **Files**: fog.rs, simulation.rs

### Phase 4: Fire + Creatures
- Grass accelerates fire spread (fire.rs)
- Duskweaver grass preference in pathfinding
- Glintcrawler nests in tall grass
- **Files**: fire.rs, simulation.rs, creatures.rs

## Connections

| System | Integration |
|--------|-----------|
| **Fiber/crafting** | Cutting grass = primary early-game fiber source |
| **Fire** | Grass is the most flammable terrain; fire breaks are essential |
| **Fog of war** | Grass degrades vision rays; concealment mechanic |
| **Combat** | Ambush from grass; enemies approach unseen |
| **Creatures** | Duskweavers prefer grass; glintcrawlers nest in it |
| **Wind** | Grass sways with wind; fire spreads downwind through grass |
| **Water** | Moisture drives growth; dry grass burns easier |
| **Terrain** | Only grows on suitable soil; rocky terrain stays bare |
| **Pathfinding** | Slight movement penalty; creatures may prefer it |

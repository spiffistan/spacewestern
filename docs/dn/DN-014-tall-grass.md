# DN-013: Tall Grass — Terrain Vegetation as Gameplay

## Status: Proposed

## Problem

The world has vegetation rendering (grass blades from `terrain_data` bits 4-8) but it's purely cosmetic. High vegetation looks slightly different but doesn't affect gameplay. There's no concealment, no fire fuel mechanic, no fiber harvesting from the environment, and no reason to clear the colony perimeter.

## Solution

Promote vegetation density above a threshold into a gameplay-meaningful "tall grass" state. No new data layer — the existing `terrain_data` vegetation field (0-31) gains gameplay meaning at high values.

## Vegetation Thresholds

```
 0-5:   Bare/sparse — rocky, sandy, heavily trafficked ground
 6-15:  Short grass — cosmetic, no gameplay effect (current behavior)
16-22:  Medium grass — slight movement penalty, partial concealment
23-31:  Tall grass — significant concealment, fire hazard, harvestable
```

These aren't new terrain types — they're states of the existing vegetation density on grassland/loam/peat terrain.

## Map Generation

### Where Tall Grass Grows

In `generate_terrain_with_params`, vegetation density is already assigned. Extend it so suitable terrain gets higher initial values:

```rust
let base_veg = match terrain_type {
    TERRAIN_GRASS => 12 + (moisture * 15.0) as u32,  // 12-27 depending on moisture
    TERRAIN_LOAM  => 10 + (moisture * 18.0) as u32,  // 10-28 (richest growth)
    TERRAIN_PEAT  => 8 + (moisture * 12.0) as u32,   // 8-20 (boggy, moderate)
    TERRAIN_MARSH => 15 + (moisture * 10.0) as u32,  // 15-25 (reeds)
    TERRAIN_CHALKY => 4 + (moisture * 8.0) as u32,   // 4-12 (sparse scrub)
    TERRAIN_CLAY  => 6 + (moisture * 10.0) as u32,   // 6-16 (moderate)
    TERRAIN_ROCKY | TERRAIN_GRAVEL => (moisture * 4.0) as u32, // 0-4 (minimal)
    _ => 0,
};
// Apply noise variation (patchy, not uniform)
let noise = fbm(x, y, 0.3) * 8.0;
let final_veg = (base_veg as f32 + noise).clamp(0.0, 31.0) as u32;
```

This produces:
- **Near water**: lush tall grass (25-31) on grassland/loam
- **Dry areas**: short sparse grass (5-12)
- **Rocky/gravel**: near-bare (0-4)
- **Patchy**: noise ensures natural irregular coverage

### Spawn Clearing

The spawn area (center of map) gets reduced vegetation:

```rust
let dist_from_spawn = distance_to_center(x, y);
if dist_from_spawn < 10.0 {
    final_veg = (final_veg as f32 * (dist_from_spawn / 10.0)).max(3.0) as u32;
}
```

The colony starts in a natural clearing. Tall grass begins ~10 tiles out.

### Natural Features

- **Game trails**: Narrow paths of low vegetation winding through tall grass (compaction = 15+). Generated as random walks between points of interest.
- **Clearings**: Occasional circular patches of short grass in dense fields (noise-based). Good building sites.
- **Thickets**: Extra-dense patches (veg 28-31) near water. Hardest to see into, richest fiber harvest.

## Dynamics

### Growth

Each game-day, vegetation increases on suitable terrain:

```rust
fn tick_grass_growth(terrain: &mut [u32], water_table: &[f32], temperature: f32) {
    for idx in 0..terrain.len() {
        let tt = terrain_type(terrain[idx]);
        let veg = (terrain[idx] >> 4) & 0x1F;
        let compaction = terrain_compaction(terrain[idx]);

        // Only grass-supporting terrain grows
        if !matches!(tt, TERRAIN_GRASS | TERRAIN_LOAM | TERRAIN_PEAT | TERRAIN_MARSH | TERRAIN_CLAY) {
            continue;
        }

        // Growth rate depends on: moisture, temperature, compaction
        let moisture = water_table.get(idx).copied().unwrap_or(0.0);
        let moisture_factor = (moisture + 0.5).clamp(0.0, 1.0);
        let temp_factor = ((temperature - 5.0) / 20.0).clamp(0.0, 1.0); // grows 5-25°C
        let compact_factor = 1.0 - (compaction as f32 / 31.0) * 0.9; // traffic kills growth
        let max_veg = match tt {
            TERRAIN_LOAM => 28,
            TERRAIN_GRASS => 25,
            TERRAIN_MARSH => 22,
            TERRAIN_PEAT => 20,
            TERRAIN_CLAY => 16,
            _ => 8,
        };

        if veg < max_veg && temp_factor > 0.1 {
            let growth = (moisture_factor * temp_factor * compact_factor * 0.3) as u32; // ~1-2 per day
            let new_veg = (veg + growth.max(1)).min(max_veg);
            terrain[idx] = (terrain[idx] & !0x1F0) | (new_veg << 4);
        }
    }
}
```

Growth rate: ~1-3 veg levels per game-day in good conditions. From stubble (5) to tall grass (23): ~6-18 days. Compacted paths stay short. Cold weather stops growth. Drought slows it.

### Drying

When temperature is high and moisture is low, tall grass dries out:

```rust
// Dry grass: vegetation stays high but a "dry" flag activates
let is_dry = veg > 15 && moisture < 0.1 && temperature > 30.0;
```

Dry grass:
- Visual: golden-brown instead of green (shader color shift)
- Flammability: 2× more likely to catch fire
- Harvest: yields "dry fiber" (same item, or could be "hay" for future livestock)
- Still provides concealment (dry or green, it's tall)

The drying state could be stored in the roughness bits (13-14) which have spare range, or derived in the shader from moisture + temperature.

### Cutting / Harvesting

A colonist assigned to cut grass (work zone or manual task):

```
Action: "Cut grass" at tile (x, y)
Duration: 2 seconds (1s with scythe tool)
Result:
  - vegetation drops to 5 (stubble)
  - Yields 1-3 fiber (existing ITEM_FIBER)
  - If dry: yields 1-2 "hay" (future item for animal feed / insulation)
```

**Clear zone**: A zone type (like growing/storage zones) that auto-assigns colonists to mow any tile where vegetation > threshold. The colony perimeter stays maintained.

### Trampling

Walking through tall grass reduces vegetation:

```rust
// In movement code, when pleb moves through a tile:
let veg = (terrain[idx] >> 4) & 0x1F;
if veg > 10 {
    let new_veg = veg - 1;
    terrain[idx] = (terrain[idx] & !0x1F0) | (new_veg << 4);
}
// Also add compaction (existing system):
terrain_add_compaction(&mut terrain[idx], 1);
```

Frequently walked tiles naturally become paths. A single crossing barely dents tall grass. Regular traffic creates visible trails.

### Fire

Grass is the most flammable terrain. Integration with existing `fire.rs`:

```rust
// In fire spread check:
let veg = (terrain[idx] >> 4) & 0x1F;
let grass_fuel = veg as f32 / 31.0;
let dry_bonus = if is_dry { 2.0 } else { 1.0 };

ignition_chance *= 1.0 + grass_fuel * 3.0 * dry_bonus;  // tall dry grass: 7× ignition
fire_spread_rate *= 1.0 + grass_fuel * 4.0 * dry_bonus;  // spreads 5× faster in dry tall grass

// After burning:
terrain[idx] = (terrain[idx] & !0x1F0) | (0 << 4);  // vegetation = 0 (burned bare)
// Existing fire system handles scorched earth rendering
```

A grass fire moves FAST — 4-5× faster than building fires. Wind direction matters enormously. Downwind of a grass fire = danger zone.

### Regrowth After Fire

Burned ground (veg=0) regrows from scratch. Takes 15-25 game-days to return to tall grass. Burned areas are visible for days as dark scorched earth (existing `is_scorched_dirt` in shader).

## Visibility

### Fog of War (fog.rs)

Vision rays passing through tall grass lose intensity:

```rust
fn vision_blocked_by_grass(terrain: &[u32], x: i32, y: i32) -> f32 {
    let idx = (y as u32 * GRID_W + x as u32) as usize;
    if idx >= terrain.len() { return 0.0; }
    let veg = ((terrain[idx] >> 4) & 0x1F) as f32 / 31.0;
    if veg > 0.5 {
        (veg - 0.5) * 1.2  // 0-0.6 opacity (tall grass blocks up to 60% per tile)
    } else {
        0.0  // short grass: no blocking
    }
}
```

In the shadowcast loop, multiply visibility by `(1.0 - grass_opacity)` at each tile. Effect: 1 tile of tall grass = see dimly. 2 tiles = barely. 3+ tiles = blind.

### Concealment

An entity in grass with veg > 20 is concealed:
- Not visible to others beyond 2 tiles
- Name label hidden (already have this pattern in fog code)
- Aiming cone from enemies doesn't lock onto concealed targets

### Vision From Inside Grass

```rust
let in_tall_grass = (terrain[pleb_tile] >> 4) & 0x1F > 20;
let effective_vision = if in_tall_grass {
    base_vision_radius * 0.4
} else {
    base_vision_radius
};
```

## Shader Rendering

### Tall Grass Visual Enhancement

The existing grass rendering (lines 640-710 of raytrace.wgsl) already draws blade detail. For veg > 20 ("tall grass"), enhance:

```wgsl
let is_tall_grass = t_veg > 0.65;  // veg 20+/31

if is_tall_grass {
    // Taller blades with more sway
    blade_max = 0.45 + t_veg * 0.15;  // taller
    blade_width = 0.04;  // thinner (individual strands)

    // Wind sway: more pronounced in tall grass
    let sway = sin(camera.time * 1.2 + world_x * 2.5 + world_y * 1.8)
             * camera.wind_magnitude * 0.08 * t_veg;
    blade_angle += sway;

    // Depth layers: render 2 additional blade layers for density
    // (the existing system does 1 layer; tall grass adds overlapping layers)

    // Dry grass color shift (temperature + moisture based)
    if camera.sun_intensity > 0.7 && moisture < 0.15 {
        grass_col = mix(grass_col, vec3(0.55, 0.48, 0.25), 0.4);  // golden-brown
    }

    // Sun catch on blade tips (specular highlight when wind bends blades toward sun)
    let sun_catch = max(0.0, dot(vec2(sway, 0.3), vec2(camera.sun_dir_x, camera.sun_dir_y)));
    blade_col += sun_color * sun_catch * 0.2;
}
```

### Entity Occlusion in Tall Grass

After rendering a pleb/creature, if they're standing in tall grass, overlay grass on their lower body:

```wgsl
if drew_pleb && t_veg_at_pleb > 0.65 {
    let occlusion = (t_veg_at_pleb - 0.5) * 2.0;  // 0-1 how much legs are hidden
    let grass_line = s * 0.3 * occlusion;  // grass covers lower portion
    if ly < grass_line {
        // Blend grass blade color over pleb's lower body
        let blend = (grass_line - ly) / (grass_line + 0.01);
        color = mix(color, grass_blade_color_at_pleb, clamp(blend, 0.0, 0.6));
    }
}
```

Plebs wading through tall grass: legs disappear into the green. Head and shoulders visible. The "wading through a field" look.

### Night Rendering

At night, tall grass is darker than short grass (denser material absorbs more light). Torch-light catches blade tips:

```wgsl
if is_tall_grass && torch_nearby {
    let tip_catch = blade_t * torch_intensity * 0.25;
    blade_col += vec3(0.35, 0.25, 0.10) * tip_catch;  // warm orange on green
}
```

## Gameplay Integration

### MapGen Preview

The map generator preview should show tall grass as a distinct visual. Dense vegetation areas rendered as darker green patches:

```rust
// In draw_map_gen_screen terrain preview:
let veg = (terrain_data[idx] >> 4) & 0x1F;
if veg > 20 {
    // Overlay darker green for tall grass areas
    let grass_tint = egui::Color32::from_rgba_unmultiplied(30, 60, 20, (veg * 4) as u8);
    painter.rect_filled(px_rect, 0.0, grass_tint);
}
```

Players can see tall grass distribution before starting. Settling near dense grass = fire risk + creature habitat but also fiber abundance.

### MapGen Parameters

Add a grass density slider to terrain params:

```rust
pub struct TerrainParams {
    // ... existing ...
    pub grass_density: f32,  // 0.0-1.0 multiplier on vegetation generation
}
```

Low grass = more open frontier (less concealment, less fire risk, less fiber).
High grass = dense savanna (more hiding spots, more danger, more resources).

### What You Can Do With Grass

| Action | Tool | Time | Yield | Effect |
|--------|------|------|-------|--------|
| **Cut** | None/hands | 3s | 1 fiber | Veg → 5 |
| **Cut** | Scythe (future) | 1s | 2-3 fiber | Veg → 5 |
| **Burn** | Torch/fire | Instant | Nothing | Veg → 0, fire spreads |
| **Trample** | Walking | Passive | Nothing | Veg -= 1 per crossing |
| **Clear zone** | Zone tool | Auto | Ongoing fiber | Maintained low veg |

### Early Game Loop

```
Day 1:   Colony in a natural clearing, tall grass 10 tiles out in all directions.
         Can barely see what's out there. Duskweavers unseen in the grass.

Day 2-3: Start cutting grass for fiber (needed for rope → tools → buildings).
         Perimeter slowly clears. Vision improves.

Day 5:   2-tile cleared ring. Can see approaching threats. First duskweaver
         spotted at the grass edge at dusk. "That's what was rustling out there."

Day 8:   Lightning during thunderstorm. Grass catches fire northeast of colony.
         Fire races toward colony. Cleared perimeter stops it. Close call.

Day 10:  Full perimeter cleared. Fire break maintained. Tall grass deliberately
         left on the south approach as a trap — enemies will wade through it
         slowly while drafted colonists shoot from the cleared zone.
```

## Files to Modify

| File | Changes |
|------|---------|
| `src/grid.rs` | Vegetation thresholds, growth parameters, mapgen vegetation formula |
| `src/simulation.rs` | Growth tick, cutting action, trampling, fire interaction |
| `src/fog.rs` | Vision ray degradation through tall grass |
| `src/shaders/raytrace.wgsl` | Enhanced tall grass rendering, entity occlusion, drying color |
| `src/zones.rs` | Clear zone type |
| `src/fire.rs` | Grass fuel factor in fire spread |
| `src/pleb.rs` | Movement speed penalty in tall grass |
| `src/ui.rs` | MapGen preview overlay, clear zone tool, grass density slider |

## Verification

1. MapGen shows tall grass as distinct darker patches near water
2. Grass density slider in MapGen adjusts coverage
3. In-game: tall grass visible as dense swaying blades (wind-reactive)
4. Walking through tall grass: movement slowed, grass trampled
5. Cutting grass: yields fiber, reduces to stubble
6. Fire: grass burns fast, spreads with wind, leaves scorched earth
7. Regrowth: stubble → short → tall over ~2 weeks
8. Visibility: can't see entities 3+ tiles into tall grass
9. Entities in tall grass: partially hidden, name label suppressed
10. Dry conditions: grass turns golden-brown, burns even faster

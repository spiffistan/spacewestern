# DN-010: Discovery Layer — Hidden Map Features & Exploration

## Status: Proposed

## Problem

The map is what you see. Trees, rocks, dirt — everything is visible from the start. There's nothing to find, no reason to explore beyond the colony perimeter, and no connection between the creatures that appear at night and the terrain they inhabit during the day. Duskweavers spawn from nowhere and vanish into nothing.

The game has rich physics systems (thermal, fluid, sound, lighting) but no discovery mechanic that rewards curiosity or creates "I found something" moments.

## Solution: A Fourth Data Layer

Add `discovery_data: Vec<u32>` — one u32 per tile, alongside the existing three layers (grid_data, wall_data, terrain_data). This encodes hidden features that the player reveals through exploration, tools, and observation.

```
┌─────────────────────────────────────────────────┐
│ Layer 4: DISCOVERY (dense grid, per tile)       │
│   Hidden features below the surface.            │
│   Revealed by tools, digging, observation.      │
│   Storage: discovery_data[idx] — u32 per tile   │
│     bits 0-7:  feature type (0=none)            │
│     bits 8-15: feature data (subtype/quantity)  │
│     bit 16:    discovered (player knows it's    │
│                here but hasn't excavated)        │
│     bit 17:    excavated (fully dug up)          │
│     bits 18-23: depth (dig time remaining)       │
│     bits 24-31: reserved                         │
├─────────────────────────────────────────────────┤
│ Layer 3: BLOCKS  (grid_data — u32 per tile)     │
│ Layer 2: WALLS   (wall_data — u16 per tile)     │
│ Layer 1: TERRAIN (terrain_data — u32 per tile)  │
└─────────────────────────────────────────────────┘
```

## Feature Types

### Creature Dens

| Type | ID | Surface Hint | Discovery | Contents |
|------|----|-------------|-----------|----------|
| Duskweaver burrow | 1 | Disturbed earth (small dark circle, mound) | Follow creatures at night, or spot subtle terrain sign by day | Stolen food, creature pelts |
| Glintcrawler nest | 2 | Faint crackling near rocky terrain | Walk too close (warning sound), or clear surrounding brush | Venom sacs (medicine crafting) |

**Duskweaver burrows** are the key change: duskweavers spawn FROM their burrow at dusk and return TO it at dawn, instead of appearing/vanishing at map edges. This means:

- Following a fleeing duskweaver reveals the burrow location
- Destroying a burrow permanently removes that pack's spawn point
- The burrow contains everything the pack has stolen
- New burrows can appear over weeks in unexplored areas (the frontier is never fully tamed)

### Mineral Deposits

| Type | ID | Surface Hint | Discovery | Contents |
|------|----|-------------|-----------|----------|
| Iron deposit | 10 | Reddish-brown terrain tint | Metal detector (strong ping) | Iron ore — tools, construction |
| Copper deposit | 11 | Greenish terrain tint | Metal detector (medium ping) | Copper — wiring, electronics |
| Rare ore | 12 | No surface hint | Metal detector (faint, deep ping) | Late-game crafting materials |

Mineral deposits are clustered by geological noise (same technique as terrain generation). Surface hints are subtle — a slightly different terrain color that's easy to miss unless you're looking. The metal detector makes them unambiguous.

### Buried Artifacts

| Type | ID | Surface Hint | Discovery | Contents |
|------|----|-------------|-----------|----------|
| Scrap cache | 20 | Scorched earth near crash site | Metal detector, or dig near spawn | Starting bonus: tools, seeds, scrap |
| Alien fragment | 21 | None | Metal detector only (rare, deep) | Technology fragment — blueprint unlock |
| Settler remains | 22 | Stone cairn, partial foundation | Visual observation | Lore text + equipment |

Connects to the ruins/archaeology concept from GAMEPLAY_SYSTEMS.md. Alien fragments are the rarest discovery — finding one unlocks a recipe you can't get any other way. The lore fragments piece together what happened on this planet before the colonists arrived.

### Natural Features

| Type | ID | Surface Hint | Discovery | Contents |
|------|----|-------------|-----------|----------|
| Hot spring | 30 | Steam wisps (fluid sim) | Walk near it, feel warmth (thermal sim) | Mood + healing when visited |
| Underground water | 31 | Greener vegetation | Water divining tool, or lucky well placement | Optimal well location |
| Cave entrance | 32 | Rocky outcrop, dark shadow | Visual observation | Shelter, danger, deep resources |
| Fertile patch | 33 | Taller grass, richer color | Farming skill / observation | 2x crop yield |

Underground water already has data — the `water_table` buffer. Tying well placement to a discoverable survey mechanic gives meaning to an existing system.

## The Metal Detector

A craftable tool that creates a distinct exploration gameplay loop.

**Crafting:** Workbench recipe — wire + battery + scrap metal.

**Usage:** Equip to a colonist → survey mode. They walk slowly. The detector is a **sound source** that pings at a rate proportional to proximity to buried metal. The existing GPU sound propagation carries the ping physically — it reflects off walls, attenuates with distance. Another colonist nearby hears the pings too.

**Feedback tiers:**
- No ping: nothing here
- Slow ping (1/sec): something within 5 tiles
- Fast ping (3/sec): within 2 tiles
- Continuous tone: standing on it

**Tone variation:** Low pitch = iron (common). Medium = copper. High = artifact (rare). The player learns to distinguish tones by ear.

**Implementation:** The detector doesn't need a new system. It's a colonist activity (`PlebActivity::Surveying`) that checks `discovery_data` in a radius each tick and pushes `SoundSource` events with varying frequency. The shader could pulse the colonist's headlight in sync.

## Surface Hints — Shader Rendering

Undiscovered features with surface hints modify terrain rendering subtly:

```wgsl
// In terrain detail section of raytrace.wgsl
let disc = discovery_data[tile_idx];
let disc_type = disc & 0xFFu;
let discovered = (disc & 0x10000u) != 0u;
let excavated = (disc & 0x20000u) != 0u;

if disc_type > 0u && !excavated {
    if disc_type == 1u { // Duskweaver burrow
        // Small dark circle, disturbed earth mound
        let bc = length(vec2(fx - 0.5, fy - 0.5));
        if bc < 0.2 {
            color = mix(color, vec3(0.15, 0.12, 0.10), 0.6);
        }
    } else if disc_type >= 10u && disc_type <= 12u && discovered {
        // Mineral deposit: colored marker (only after discovery)
        let marker_pulse = sin(camera.time * 2.0) * 0.1 + 0.9;
        if disc_type == 10u { color = mix(color, vec3(0.6, 0.25, 0.15), 0.15 * marker_pulse); }
        if disc_type == 11u { color = mix(color, vec3(0.2, 0.5, 0.3), 0.15 * marker_pulse); }
    }
}
```

Burrows are always slightly visible (you can find them by looking). Mineral deposits only show their marker after discovery (detector found them). Artifacts have no surface sign at all.

## World Generation

Hidden features are placed in `generate_world()` alongside terrain:

```
Burrows:     3-5 per map, near edges, in grass/rocky terrain, >30 tiles from spawn
Minerals:    Noise-based clusters, 10-20 deposits, biased toward rocky terrain
Artifacts:   2-4 per map, completely random placement, no clustering
Hot springs: 0-2 per map, near water table peaks
Fertile:     Noise-based patches near water, 5-10% of farmable tiles
```

## Exploration Loop

The discovery system creates a gameplay arc that runs alongside the building/survival loops:

1. **Night 1:** Duskweavers steal food. Player learns they exist.
2. **Night 2:** Player watches fleeing duskweavers. They go northeast.
3. **Day 3:** Player scouts northeast. Finds disturbed earth — a burrow. Digs it up. Stolen berries recovered.
4. **Day 5:** Player crafts metal detector. Walks the perimeter. Hears a ping near the old riverbed.
5. **Day 6:** Excavation. Iron deposit. Better tools unlocked.
6. **Day 10:** Deep rare ping near the crash site. An alien fragment. What does it do?

Each discovery is a story. Each story teaches a mechanic. The map accumulates history.

## Connection to Design Philosophy

From PHILOSOPHY.md:

- **The map as memory:** Discovery sites become landmarks. Cleared burrows, mined deposits, excavated ruins — the map tells the colony's exploration story.
- **Scarcity as teacher:** The first stolen-food incident teaches about burrows. The first mineral find teaches about surveying. No tutorials needed.
- **Permanence:** Destroying a burrow permanently changes the ecosystem. Mining a deposit depletes it. These are irreversible choices.
- **The things you can't build:** Hot springs, fertile patches, mineral veins — the best colony sites have natural features worth building around. WHERE you settle matters.

## Implementation Phases

### Phase 1: Burrows
- Add `discovery_data: Vec<u32>` to App
- Place 3-5 burrows during world gen
- Render burrows in shader (small dark circles)
- Duskweavers spawn from/return to burrows instead of map edges
- Dig action on burrow tile destroys it, drops stolen items
- **Files:** main.rs, grid.rs, simulation.rs, raytrace.wgsl

### Phase 2: Metal Detector + Minerals
- Add mineral deposits to world gen (noise-based clusters)
- Metal detector recipe + `PlebActivity::Surveying`
- Detector sound pings via SoundSource
- Excavation task (like building, with progress bar)
- Surface hint rendering in shader
- **Files:** simulation.rs, raytrace.wgsl, grid.rs, pleb.rs, types.rs

### Phase 3: Artifacts + Lore
- Alien fragments: rare buried items
- Blueprint unlock mechanic (finding artifacts grants recipes)
- Lore text fragments (narrative snippets from the card system)
- Settler remains with equipment drops
- **Files:** simulation.rs, types.rs, ui.rs, creatures.toml

### Phase 4: Natural Features
- Hot springs (thermal sim integration — warm tile, mood bonus)
- Fertile soil patches (crop yield multiplier from terrain_data)
- Cave entrances (future multi-level hook)
- **Files:** grid.rs, simulation.rs, raytrace.wgsl, zones.rs

## Risks

**Performance:** An extra 256KB buffer (256×256×4) is trivial. The shader check adds one texture read per pixel for the discovery layer, gated behind a `disc_type > 0` early exit.

**Complexity creep:** Phase 1 (burrows) is small and self-contained. Each subsequent phase is independent. Don't need all four — even just burrows significantly improves the creature system.

**Metal detector balance:** Too easy = trivializes exploration. Too hard = nobody uses it. The ping-rate feedback is key — it gives information without giving answers. You still have to walk there and dig.

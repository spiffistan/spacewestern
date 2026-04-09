# DN-023: Sub-Tile Mining and Geology

**Status:** Draft
**Depends on:** DN-016 (terrain/elevation), DN-022 (skill scale)
**Related:** Stone Lab debug tool, procedural stone rendering

## Problem

RimWorld mining is "click tile → wait → tile disappears → resources appear." There's no spatial reasoning, no discovery, no skill beyond speed. Every tile of granite is identical. Mining is a solved chore after hour one.

We want mining that rewards attention, creates interesting spaces, and makes the geology of the world legible and strategic.

## Core Principle: Mining Carves, Not Removes

When you mine rock, the rock stays. Mined-out sub-cells become empty space. The remaining rock retains its shape. A partially-mined boulder looks chiseled. A partially-mined cliff has a carved notch. The tile never "disappears" — it's sculpted.

This means mining is a form of architecture. A granite cliff becomes your workshop. A sandstone face becomes a quarry. You're not removing the world, you're reshaping it.

## Sub-Tile Mining Grid

Each rock tile, when mining begins, generates an **8×8 grid** of sub-cells (64 cells per tile). Each sub-cell has:

```rust
struct MiningCell {
    material: u8,    // 0=host rock, 1=iron, 2=copper, 3=flint, 4=crystal, 5=void, 6=coal
    hardness: u8,    // 0=mined out, 1-10=intact
}

struct MiningGrid {
    cells: [[MiningCell; 8]; 8],
    rock_type: u8,    // host rock (granite, sandstone, etc.)
}

// Sparse: only tiles being mined or adjacent to mining
mining_grids: HashMap<(i32, i32), MiningGrid>
```

The grid is generated deterministically from the tile's position seed. Same tile always has the same veins.

## Mineral Veins

Veins are generated from high-frequency noise continuous across tile boundaries. A vein that starts in one tile predictably continues into the next. The player (and their geologist) can learn to follow veins.

| Mineral | Shape | Width | Host rock | Color in cut face |
|---------|-------|-------|-----------|------------------|
| Iron oxide | Shallow bands | 2-3 cells | Sandstone, granite | Rusty red-brown |
| Copper | Deep narrow lines | 1 cell | Basalt, granite | Green-blue patina |
| Flint | Nodules (clusters) | 2-4 cells | Chalk, limestone | Dark glossy black |
| Crystal | Pockets (void + crystal) | 1-2 cells | Any deep rock | Sparkling clear/purple |
| Coal | Thick seams | 3-4 cells | Sedimentary | Matte black |

## Directional Mining

The miner stands **adjacent** to the rock and works inward from their side. Mining from the south reveals south-facing cross-sections. The player sees veins in the cut face and decides where to dig next.

Where you stand to mine matters. To efficiently extract a vein running east-west, mine from the north or south (cross-cutting). Following the vein lengthwise wastes effort on host rock.

## Mining Speed

Each sub-cell takes time proportional to hardness and inversely proportional to tool quality × skill:

| Tool | Time per sub-cell | Full tile (64 cells) |
|------|------------------|---------------------|
| Bare hands | ~8 game-seconds | ~8 minutes |
| Stone pick | ~3 game-seconds | ~3 minutes |
| Flint pick | ~2 game-seconds | ~2 minutes |
| Iron pick | ~1 game-second | ~1 minute |

This is deliberately slow. Mining is a commitment, not a click.

## Structural Behavior (Rock Type Dependent)

Rock type determines what happens to the carved space:

| Rock | Structural integrity | Behavior |
|------|---------------------|----------|
| Chalk | Very low | Caves in when >30% removed. Creates rubble, not space |
| Sandstone | Moderate | Small cavities hold. Wide spans crumble over time |
| Limestone | Moderate-high | Natural cave tendencies. Water erosion creates voids |
| Granite | High | Holds almost any shape. Ideal for chambers and tunnels |
| Basalt | Very high | Never caves. Perfect rooms — but painfully slow to mine |

Chalk is strip-mined for flint (fast, messy). Granite is sculpted into permanent architecture (slow, magnificent).

## Geology Skill

A new skill domain: **Geology** (or "Prospecting"). Affects what the player can learn about rock.

- **0-3:** "It's rock." No vein indicators. Slow. Might mine through ore without recognizing it.
- **4-6:** Can identify rock types from surface. Recognizes veins in cut faces. "This looks like limestone — flint likely."
- **7-9:** Surface examination reveals vein directions. Subtle color hints visible on unmined rock. "Iron vein runs northeast, about 3 tiles."
- **10:** Full mineral awareness. Sees through stone. Never wastes a dig.

### The "Examine" Action

Right-click rock → "Examine" (time cost, uses geology skill). Results vary by skill level. Information is per-pleb — the geologist's knowledge doesn't automatically transfer.

Gameplay: send your geologist to examine, then send your miner to dig where indicated. Division of labor.

## Shader Rendering

The raytrace shader reads the mining grid:
- Each pixel maps to a sub-cell (8×8 per tile)
- Mined cells: dark ground/void
- Intact cells: procedural stone (from Stone Lab presets)
- Cut boundary: shows cross-section with vein colors
- Geology skill level could affect vein visibility (fade out for low-skill plebs in fog-of-war?)

A partially-mined rock shows chisel marks, exposed faces, colored vein streaks along the cut — the player reads the geology visually as they dig.

## What Drops

| Material | When mined | Item |
|----------|-----------|------|
| Host rock | Always | Stone blocks (building material) |
| Iron oxide | When iron sub-cell mined | Iron ore nugget |
| Copper | When copper sub-cell mined | Copper ore nugget |
| Flint | When flint sub-cell mined | Flint nodule (tool-making) |
| Crystal | When crystal sub-cell mined | Crystal specimen (trade/morale) |
| Coal | When coal sub-cell mined | Coal (fuel) |
| Void | When void exposed | Nothing — but creates space |

## Implementation Order

1. MiningGrid struct + deterministic vein generation from noise
2. Sub-cell mining mechanic (chip from facing edge)
3. Shader rendering (mined/unmined/vein boundary)
4. Resource drops per material type
5. Rock type structural behavior (collapse for soft rock only)
6. Geology skill + examine action
7. Underground tunneling (future: vertical mining, support pillars)

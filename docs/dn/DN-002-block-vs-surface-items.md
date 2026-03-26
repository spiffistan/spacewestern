# DN-002: Block Types vs Surface Items

## Status: Proposed

## Problem

Every "thing" in the world is currently a block type (BT_*) occupying a full tile. Adding a new block type requires touching ~8 files: BT_* constant in grid.rs, wgsl_block_constants(), blocks.toml entry, NUM_MATERIALS bump in materials.rs, shader clamp updates in 4 WGSL files, plus integration across fire/fog/placement/simulation.

This works for structural elements but creates two problems:

1. **A tile can only be one block type.** You can't have a bench with a cup on it, a floor with a dropped tool, or a table with food. The moment you want two things on one tile, the model breaks.

2. **The 256-type ceiling.** Block type is 8 bits (bits 0-7 of the u32). Currently at 62 types. Plenty for structural/functional blocks, but if every decoration, food item, and tool gets a BT_*, the space fills up and each addition costs significant integration work.

## Proposed: Three-Layer Model

| Layer | Purpose | Storage | Examples |
|-------|---------|---------|----------|
| **Block** (tile grid) | Structural things that define the tile | `grid_data[idx]` — one u32 per tile | Walls, floors, doors, furniture, equipment, trees |
| **Surface item** (entity) | Small things resting on/near blocks | `Vec<SurfaceItem>` with position + item | Cup on table, tool on workbench, candle, sign, plank stack |
| **Carried/stored** (inventory) | Items inside containers or on plebs | `PlebInventory` / `CrateInventory` | Everything being carried, crate contents |

## What Should Be a Block Type

Things that **define the tile's physics and function:**

- Walkable/impassable (affects pathfinding)
- Has height (casts shadows, blocks vision)
- Blocks/conducts fluid, heat, electricity
- Is a functional station (plebs interact with the tile position)
- Is structural (walls, floors, doors, pipes, wires)

## What Should NOT Be a Block Type

Things that **sit on top of** blocks without changing the tile's behavior:

- Dropped resources (already handled by `ground_items`)
- Decorations (mugs, plates, books, candles, paintings)
- Items on furniture surfaces (tools on workbench, food on table)
- Small props (bucket by well, plank stack near saw horse)
- Signs with text content
- Anything you'd want coexisting on the same tile as something else

## Existing System: ground_items

`GroundItem` already implements half of this — items with world positions that render in-world and can be picked up. Currently framed as "dropped loot waiting to be hauled." Extending this into a proper surface item layer is the natural path.

## Proposed: SurfaceItem

```rust
struct SurfaceItem {
    x: f32, y: f32,          // world position
    stack: ItemStack,         // what it is (uses existing item system)
    placed: bool,             // intentionally placed (decoration) vs dropped (loot)
}
```

Key differences from current `GroundItem`:
- `placed = true` items are not auto-hauled — they're intentional decorations
- Rendering uses item-specific sprites/shapes (not just generic dots)
- Click interaction: pick up, move, use
- Can coexist with any block type on the same tile

## Rendering

Surface items render in the raytrace shader after blocks, before plebs. Small sprites or procedural shapes based on item type. The item registry already has icons — these could drive in-world rendering.

## When to Build This

Not yet. The block system handles all current needs. Surface items become important when:
- The game needs visual richness ("lived in" colony feeling)
- Decorations and aesthetics become a gameplay element
- Players want to place items on furniture
- The block type count starts feeling constrained

## Decision Rule

When adding a new "thing" to the game, ask:
1. Does it affect pathfinding, vision, fluid, heat, or electricity? → **Block type**
2. Do plebs interact with the tile as a station? → **Block type**
3. Does it have structural height? → **Block type**
4. Is it a small object sitting on/near another thing? → **Surface item** (future)
5. Is it carried or stored? → **Inventory item**

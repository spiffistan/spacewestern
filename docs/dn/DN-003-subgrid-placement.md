# DN-003: Sub-Grid Placement Snapping

## Status: Proposed

## Problem

All block placement snaps to whole-tile positions. A 3×3 item in a 4-tile-wide space is flush against one edge — it can't be centered. Multi-tile objects look misaligned when the object size doesn't match the available space.

## Solution

A 4×4 sub-grid within each tile for placement snapping. The cursor can snap to quarter-tile positions, allowing finer alignment of multi-tile objects.

### Example

A 3-block-wide object placed in a 4-block-wide room:

```
Tile grid:      |  1  |  2  |  3  |  4  |
Without subgrid: [OBJ][OBJ][OBJ][   ]        ← flush left
With subgrid:    [ [OBJ][OBJ][OBJ] ]          ← centered
                  ^2 sub-cells margin each side^
```

3 blocks = 12 sub-cells. 4 blocks = 16 sub-cells. 2 sub-cells margin on each side.

### What This Is

- Finer cursor snapping (1/4 tile increments) during placement
- A rendering offset so objects draw at their sub-grid position
- A small offset value stored per placed multi-tile object

### What This Is NOT

- Not a per-tile data grid for small items (see DN-002 for that)
- Not a new rendering system — objects render the same, just shifted
- Not a physics/pathfinding change — collision still operates on tile boundaries

## Ownership

A sub-grid-placed object is owned by exactly one tile, regardless of visual overlap.

**Ownership rule: the top-left tile of the item's bounding box owns it.** If an item perfectly straddles a tile boundary, top-left wins. Deterministic, no ambiguity — same convention as screen coordinates and existing multi-tile items (beds, benches).

A chair at tile (5, 3) with sub-offset (2, 0) visually bleeds into tile (6, 3), but (5, 3) owns it. Block data, interaction, removal — all keyed to the owning tile.

This matches how trees already work: canopy sprites spill into neighboring tiles visually, but the tree block lives at one position.

### Click resolution

When the player clicks tile (6, 3) and visually hits the chair, the click handler checks neighboring tiles for sub-grid items whose visual extent overlaps into the clicked tile. Small lookup — at most 4 cardinal neighbors.

## Data

Multi-tile objects that use sub-grid placement store a sub-tile offset:

```
sub_offset_x: u8,  // 0-3 (quarter-tile offset)
sub_offset_y: u8,  // 0-3
```

This could be packed into the block flags or stored alongside the placement data. Two bits per axis = 4 bits total.

## Rendering

The shader applies the sub-offset when rendering the object. For a block at tile (bx, by) with sub-offset (sx, sy):

```
render_x = f32(bx) + f32(sx) * 0.25
render_y = f32(by) + f32(sy) * 0.25
```

The object's procedural render function uses this shifted origin instead of the tile center.

## Pawn Interaction

Some sub-grid items have interaction points — a chair is sat in, a workbench is worked at from the front. Pawns need to reach the right sub-grid position.

**Don't sub-grid the pathfinder.** A* operates on the 256×256 tile grid (65K nodes). A 4× sub-grid would be 1024×1024 (1M nodes) — 16× pathfinding cost for every pleb, every request. Not worth it.

Instead: **tile-level path, sub-grid final approach.**

1. A* paths to the tile (or adjacent walkable tile), same as today
2. On arrival, the pleb walks the final fractional distance to the sub-grid target using direct linear movement — no pathfinding needed within a single tile
3. The pleb's `(x, y)` float position already supports sub-tile precision; plebs smoothly interpolate between tiles. Final target becomes `(tile_x + sub_x * 0.25, tile_y + sub_y * 0.25)` instead of `(tile_x + 0.5, tile_y + 0.5)`

### Interaction points per item type

Not every item is interacted with at its center:

| Item | Interaction point |
|------|------------------|
| Chair | Center (pawn sits in it) |
| Workbench | Adjacent tile, facing the bench's sub-grid edge |
| Table | Adjacent tile nearest the pawn's approach |
| Bed | Head-end tile of the bed |

The interaction point is defined per block type, offset by the item's sub-grid position. Existing `adjacent_walkable()` logic applies — just with a sub-grid-adjusted target position for the final approach.

## Scope

This is a placement UX feature. The sub-grid exists only for positioning — it doesn't store items, affect pathfinding, or change tile physics. Pathfinding remains tile-level; sub-grid precision is handled in the final-approach step. Future extensions (sub-tile item storage, decoration placement) could build on the same 4×4 grid, but are not part of this proposal.

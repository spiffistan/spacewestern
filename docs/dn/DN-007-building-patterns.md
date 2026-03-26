# DN-007: Allowed Building Patterns With Thin Walls

## Status: Analysis

## Current System Summary

Each wall tile stores in its flags byte:
- **bits 3-4**: primary edge direction (0=N, 1=E, 2=S, 3=W)
- **bits 5-6**: thickness (0=full/4 sub-cells, 1→3, 2→2, 3→1 sub-cell)
- **bit 2**: corner flag (also covers next clockwise edge: N→N+E, E→E+S, etc.)

The tile is 1×1 in world units, divided into a 4×4 sub-grid conceptually. A thickness-1 wall on the north edge occupies the top 25% of the tile. The rest is open space (for floor/furniture).

### Key functions:
- `pixel_is_wall(fx, fy, flags)` — is this sub-pixel inside the wall area?
- `edge_blocked(grid, ax, ay, bx, by)` — is movement between adjacent tiles blocked?
- `has_wall_on_edge(flags, edge)` — does this block have a wall on the given edge?
- `edge_covers_pixel(fx, fy, edge, wall_frac)` — does this edge's wall cover this pixel?

### Building placement:
- **Drag a rectangle** → hollow rect, each tile gets the correct edge (top row=N, bottom=S, left=W, right=E, corners get L-shape)
- **Drag a line** → all tiles get the same edge (horizontal=N or S based on rotation, vertical=E or W)
- **Single tile** → edge from current rotation (Q/E to cycle)
- **Thickness** → controlled by +/- keys, applies to all placed walls

## Patterns That Work

### Simple Rectangle Room
```
Drag from (2,2) to (6,5):
  (2,2)NW  (3,2)N   (4,2)N   (5,2)N   (6,2)NE
  (2,3)W                                (6,3)E
  (2,4)W                                (6,4)E
  (2,5)SW  (3,5)S   (4,5)S   (5,5)S   (6,5)SE
```
Each corner tile has the corner flag set. Interior is open. Room is sealed for roof detection. ✅

### L-Shaped Rooms
Two overlapping rectangles. Where they share a wall segment, the wall is just one tile with one edge. The second rectangle's wall on that edge overwrites or coexists. ✅ (but needs careful handling of overlapping placements)

### Corridors
Horizontal: drag a 1-wide horizontal line → wall on N edge. Drag another line 2 tiles south → wall on S edge. Creates a corridor between them. ✅

### Shared Walls Between Rooms
Room A has east wall, Room B is immediately east. Only Room A's east wall is needed — `edge_blocked()` checks BOTH tiles and blocks if EITHER has a wall on the shared edge. ✅

### Interior Partitions
Place a single line of thin walls inside a room to divide it. Works because walls can go on any edge of any tile. ✅

## Patterns That Are Problematic

### Problem 1: Two Walls on the Same Edge
If tile (3,3) has a wall on its East edge, and tile (4,3) has a wall on its West edge — they're occupying the SAME physical boundary between those two tiles. This is wasteful (double resources) and visually confusing (do they render as two separate walls with a gap between them?).

**Current behavior:** Both walls render independently. A thickness-1 wall on the east edge of (3,3) renders as a strip on the right 25% of that tile. A thickness-1 wall on the west edge of (4,3) renders as a strip on the left 25% of THAT tile. Visually they appear as two walls with a thin gap between them.

**Recommendation:** When placing a wall on an edge, check if the adjacent tile already has a wall on the mirrored edge. If so, either:
- **Block placement** with a "wall already exists" message
- **Merge**: skip the placement since the edge is already blocked
- **Allow it** but render them as one thicker wall (complex)

### Problem 2: Corner Sealing
The corner flag creates an L-shape (primary edge + next clockwise). But what about the opposite corner? A room needs walls on ALL four corners. The top-left corner is NW, coded as "primary=W, corner flag" (W→W+N). The top-right is NE = "primary=N, corner flag" (N→N+E).

**Current behavior:** `thin_wall_edge_for_rect()` correctly assigns corner edges based on position in the drag rectangle. ✅

**But:** If you build walls piecemeal (drag north wall, then drag east wall separately), the corner tile might get overwritten. The first drag sets it to N edge. The second drag sets it to E edge. The N edge is lost — corner is unsealed!

**Recommendation:** When placing a wall on a tile that already has a wall on a different edge, UPGRADE to a corner instead of overwriting:
```
Existing: N edge → Place E edge → Result: N+E corner (primary=N, corner flag)
```

### Problem 3: T-Junctions
Three walls meeting at a point. A tile can only store ONE primary edge + one optional corner (next clockwise). A T-junction needs THREE edges (e.g., N+E+S). The current encoding can't represent this.

**Options:**
- **Disallow T-junctions** — simplest. Force the player to offset one wall by one tile. Architecturally this means no interior walls meeting exterior walls at the same point.
- **Full-tile wall at junctions** — if three or more edges are needed, upgrade to a full-tile (thickness 4) wall at that junction. Visually acceptable (it's a pillar/column).
- **Expand encoding** — use a bitmask instead of primary+corner. 4 bits for N/E/S/W individually. This would require restructuring the flags encoding.

**Recommendation:** Option 2 (auto-upgrade to full wall at junctions). When placing a wall that would create a 3+ edge junction, convert that tile to full thickness. This is physically realistic (structural junctions need more mass) and sidesteps the encoding limitation.

### Problem 4: Walls on Non-Wall Tiles
Can you put a wall on a tile that already has furniture? A tile with a bed — can you add a wall on the north edge? Physically yes (the bed is in the open part of the tile). But the block type is BT_BED, not BT_WALL. The thin wall system is encoded in the flags of wall-type blocks.

**Current behavior:** Walls and furniture are different block types. You can't have both in one tile.

**Recommendation:** This is a fundamental limitation of the 1-block-per-tile model. Solutions:
- **Wall goes on the adjacent empty tile** — the wall is always its own tile, furniture goes next to it
- **Sub-grid surface items** (DN-002) — furniture becomes an overlay on top of the wall tile
- **Accept it** — thin walls leave enough room that furniture fits in adjacent tiles

### Problem 5: Isolated Wall Segments
A single wall tile floating in space (not connected to anything). Is this allowed?

**Current behavior:** Allowed. It's a fence, barrier, or partition.

**Recommendation:** Allow it. Freestanding walls are useful for pens, wind breaks, and barriers. No need to enforce connectivity.

### Problem 6: Single-Tile Enclosure
A tile with walls on all 4 edges (thickness 1) = a tiny pillar/column with no interior space.

**Current behavior:** Possible with a full-thickness wall (thick_raw=0). But impossible with thin walls + corner flag (max 2 edges per tile).

**Recommendation:** A full-thickness wall effectively IS a pillar. This is fine. No need to support thin-wall 4-edge tiles.

## Placement Rules (Proposed)

### Rule 1: No Double Walls
When placing a wall on an edge, check the adjacent tile. If it already has a wall on the mirrored edge, skip this tile (the edge is already sealed).

### Rule 2: Auto-Upgrade Corners
When placing a wall on a tile that already has a thin wall on a DIFFERENT edge:
- If the edges are adjacent (N and E, E and S, etc.) → upgrade to corner
- If the edges are opposite (N and S, E and W) → upgrade to full wall (pillar)
- If three edges needed → full wall

### Rule 3: Auto-Corner on Rectangle Drag
Already implemented. Rectangle drags automatically set corner flags on corner tiles.

### Rule 4: No Wall-on-Furniture
Walls can only be placed on empty tiles (air/dirt) or existing wall tiles (to add another edge). Furniture tiles are not valid wall placement targets.

### Rule 5: Thickness Consistency
Optional: when extending an existing wall run, inherit the thickness of the connected wall. Prevents jarring thickness changes mid-wall.

### Rule 6: Demolish by Edge
When demolishing a thin wall, remove only the selected edge (not the whole tile). A corner tile loses one edge and becomes a single-edge wall. Only when all edges are removed does the tile become dirt.

## Roof Detection Impact

The roof fill algorithm currently checks `is_wall_block(bt) && height > 0`. With thin walls, a room might have walls on all edges but the wall tiles themselves are still "ground level" outside their wall strip.

The algorithm needs to use `edge_blocked()` for flood-fill boundaries:
1. Start from an interior tile
2. Flood fill, stopping when `edge_blocked()` returns true
3. All reached tiles get the roof flag

This is more expensive than checking individual tiles but gives correct results for thin walls.

## Visual Rendering Impact

The raytrace shader's `pixel_is_wall()` function correctly renders the wall strip. The open part of the tile shows the floor/ground beneath. Corners render as L-shapes.

**South face (oblique):** The oblique wall face check needs to account for thin walls. Only the wall portion of the south edge should show a face — not the entire tile width.

**Shadow casting:** The shadow map needs to check `pixel_is_wall()` at each step, not just block height. Currently it checks `block_height > 0` which means the entire tile casts shadows even if the wall only occupies 25% of it.

## Summary of Recommendations

| Issue | Recommendation | Effort |
|-------|---------------|--------|
| Double walls | Block placement if mirrored edge exists | Small |
| Corner upgrades | Auto-merge when adding edge to existing wall | Medium |
| T-junctions | Auto-upgrade to full wall at 3+ edges | Small |
| Wall on furniture | Disallow (existing behavior) | None |
| Isolated walls | Allow (fences, barriers) | None |
| Thickness consistency | Optional: inherit from connected wall | Small |
| Edge demolish | Remove individual edges, not whole tile | Medium |
| Roof detection | Use edge_blocked() flood fill | Medium |
| Shadow precision | Use pixel_is_wall() in shadow ray | Medium |

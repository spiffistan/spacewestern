# DN-004: Thin Walls and Sub-Grid Furniture

## Status: Proposed

## Problem

Walls occupy a full tile. A 4-wide room with full-tile walls has 2 usable tiles inside — walls eat 50%+ of small buildings. Furniture can't be pushed against walls because the wall IS the tile. Buildings feel like bunkers, not rooms.

## Solution

Walls have variable thickness (1-4 sub-cells) and press against one edge of their tile. The remaining sub-cells are open for furniture placement. A chair (2×2 sub-cells) fits inside the open area of a wall tile.

### Sub-Grid Layout Examples

```
Thickness 1, north wall:     Thickness 2, north wall:     Current (thickness 4):
[W][W][W][W]                 [W][W][W][W]                 [W][W][W][W]
[ ][ ][ ][ ]                 [W][W][W][W]                 [W][W][W][W]
[ ][C][C][ ]  ← chair       [ ][C][C][ ]                 [W][W][W][W]
[ ][C][C][ ]                 [ ][C][C][ ]                 [W][W][W][W]
```

### Furniture Sizes (Sub-Cell Units)

Most furniture fits within one tile's open sub-cells:

| Item | Sub-cells | Notes |
|------|-----------|-------|
| Chair/stool | 2×2 | Fits against any thin wall |
| Small table | 2×2 | Same tile as wall |
| Bench segment | 4×2 | Full width, 2 deep |
| Standing lamp | 1×1 | Tucks into corners |
| Armchair | 3×3 | Needs thickness-1 wall to fit |

### Corner Junctions

A tile with walls on two edges (e.g. north + west) forms an L-shape:

```
North + West wall, thickness 1:
[W][W][W][W]
[W][ ][ ][ ]
[W][ ][C][C]  ← chair fits in the open corner
[W][ ][C][C]
```

## Data Encoding

6 bits in the block flags:

```
wall_edges: 4 bits  (bit flags: N=1, E=2, S=4, W=8)
thickness:  2 bits  (0=1 sub-cell, 1=2, 2=3, 3=4/full — shared across all edges)
```

Full-thickness walls (thickness=4) are backwards-compatible with current behavior.

## Placement Validation

When placing a sub-grid item on a tile:
1. Compute which sub-cells the item would occupy (based on item size + sub-position)
2. Compute which sub-cells the wall occupies (based on wall_edges + thickness)
3. If no overlap → placement allowed

No neighboring tile checks needed for furniture that fits within one tile.

## Pathfinding

A* remains tile-level. The passability check between two adjacent tiles becomes edge-aware:

- Moving from tile A to tile B (northward): blocked if tile A has a north wall OR tile B has a south wall
- Within a tile that has a wall on one edge: the tile itself is walkable (pawns occupy the open sub-cells)
- A tile with walls on all 4 edges at thickness 4 is fully impassable (current behavior)

```rust
fn can_cross_edge(grid: &[u32], from: (i32, i32), to: (i32, i32)) -> bool {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    // Check if 'from' has a wall on the exit edge
    // Check if 'to' has a wall on the entry edge
    // Blocked if either has a wall on the shared edge
}
```

## Vision / Shadow Casting

`blocks_vision()` becomes edge-aware. A thin wall on the north edge blocks sightlines crossing that edge, but the tile is not opaque from other directions. The shadowcasting octant scan needs to know which edge of a tile is walled, not just whether the tile blocks.

This is the most complex system change — shadowcasting algorithms assume per-tile opacity.

## Thermal / Fluid

Wall sub-cells are solid (no heat conduction, no fluid flow). Open sub-cells conduct and flow normally. The compute shaders would need to sample the wall_edges flags to determine which directions are blocked per-tile.

## Rendering

Straightforward. For a tile with a north wall of thickness T:

```wgsl
let wall_frac = f32(thickness) * 0.25;
if fy < wall_frac {
    // Render wall material
} else {
    // Render floor / furniture
}
```

Wall face rendering (oblique south face) uses the thickness to control face height proportionally.

## Impact Assessment

| System | Change Required | Difficulty |
|--------|----------------|------------|
| Rendering | Sub-cell wall check in shader | Low |
| Placement | Sub-cell overlap validation | Low |
| Pathfinding | Edge-based crossing checks | Medium |
| Vision/fog | Edge-aware shadowcasting | High |
| Thermal | Directional conductivity | Medium |
| Fluid | Directional flow blocking | Medium |
| Block data | 6 bits in flags | Low |

## Migration

Existing walls (thickness 4, all edges) behave identically to current full-tile walls. No save-breaking changes. Thin walls are a new option, not a replacement.

## Recommendation

Implement in phases:
1. **Data + rendering + placement** — thin walls render correctly, furniture can be placed in open sub-cells
2. **Pathfinding** — edge-based crossing checks
3. **Vision** — edge-aware shadowcasting
4. **Thermal/fluid** — directional blocking

Phase 1 alone delivers the visual impact. Phases 2-4 make it physically correct.

# DN-005: Windows and Doors as Wall Features

## Status: Proposed

## Problem

Windows (BT_GLASS) and doors (BT_WALL with door flag) currently **replace** the wall block entirely. A glass tile is a separate block type with its own material — not a window in a wall. This creates several problems:

1. **Lost wall material.** A stone wall with a window becomes a glass tile — the stone material is gone. You can't have a "stone wall with window" that inherits the stone appearance.

2. **Full-tile occupancy.** A window replaces the entire tile. You can't have a narrow window slit, a half-height window, or a window that's only part of the wall face.

3. **Doors eat the wall.** A door replaces the wall block. The wall material around the door frame disappears. Door frames should show the wall material, not be a separate block type.

4. **With thin walls (DN-004), it gets worse.** A thin wall with a window should be a thin wall that has a glass section cut into it — not a separate glass block at full tile width.

## Current Implementation

### Windows
- `BT_GLASS` (ID 5) is a separate block type
- Placed by replacing a wall block: `BuildTool::Window` sets the tile to `make_block(5, height, roof_flag)`
- Rendered in shader with `render_glass_block()` — detects wall orientation from neighbors, draws glass inset with stone surround
- The "stone surround" is hardcoded (`vec3(0.55, 0.53, 0.50)`) — doesn't match the original wall material
- Light transmission, interior sunbeam tracing, and shadow casting all check for `BT_GLASS` specifically

### Doors
- A wall block with flags bit 0 set (`is_door`)
- Flags bit 2 toggles open/closed (`is_open`)
- The wall block type is preserved (BT_WALL, which is generic stone) but the original wall material (wood, granite, etc.) is lost — doors are always placed as `BT_WALL` with door flag
- Open doors don't block vision or pathfinding; closed doors do

## Proposed: Windows and Doors as Wall Attributes

Instead of replacing the wall block, windows and doors become **features of the wall** — stored in flags or a companion data structure. The wall block type and material are preserved.

### Window as Wall Feature

A window is a rectangular opening in the wall, filled with glass. The wall material remains around it.

**Data:** The wall block keeps its type (BT_STONE, BT_WOOD_WALL, etc.). Window presence and properties are encoded in flags or a per-tile feature byte.

**Rendering:** The shader checks for window presence. Where the window is, it renders glass with the wall's own material as the frame surround — not a hardcoded stone color. A wood wall with a window has a wood frame. A granite wall has a granite frame.

**Sub-grid integration (DN-004):** With thin walls, the window occupies a section of the wall's sub-cells. On a thickness-2 wall along the north edge:

```
Without window:          With window (2 sub-cells wide, centered):
[W][W][W][W]             [W][_][_][W]    _ = glass
[W][W][W][W]             [W][_][_][W]
[ ][ ][ ][ ]             [ ][ ][ ][ ]
[ ][ ][ ][ ]             [ ][ ][ ][ ]
```

The window cuts through the wall thickness at specific sub-cell positions.

### Door as Wall Feature

A door is a section of wall that opens and closes. The wall material shows as the door frame.

**Data:** Same as current — door flag on the wall block. But now the wall type is preserved during door placement (currently replaced with BT_WALL).

**Sub-grid integration:** A door occupies a span of sub-cells along the wall edge. A 2-sub-cell-wide door in a 4-sub-cell wall:

```
Door closed:              Door open:
[W][D][D][W]              [W][ ][ ][W]
[W][D][D][W]              [W][ ][ ][W]
[ ][ ][ ][ ]              [ ][ ][ ][ ]
[ ][ ][ ][ ]              [ ][ ][ ][ ]
```

The wall sub-cells flanking the door are the frame — rendered in the wall's material.

### Variable Window/Door Width

With 4 sub-cells along each wall edge, windows and doors can be 1, 2, 3, or 4 sub-cells wide:

| Width | Sub-cells | Appearance |
|-------|-----------|------------|
| 1 | `[W][_][W][W]` | Arrow slit / narrow window |
| 2 | `[W][_][_][W]` | Standard window |
| 3 | `[_][_][_][W]` | Wide window / picture window |
| 4 | `[_][_][_][_]` | Full-width glass wall (current BT_GLASS behavior) |

Same applies to doors.

### Variable Window/Door Position

The opening can be at any position along the wall edge:

```
Left-aligned:   [_][_][W][W]
Centered:       [W][_][_][W]
Right-aligned:  [W][W][_][_]
Off-center:     [_][_][_][W]
```

## Data Encoding

### Option A: Pack into Existing Flags

The flags byte (bits 0-7) is heavily used:
- Bit 0: is_door
- Bit 1: has_roof
- Bit 2: is_open
- Bits 3-4: direction/segment (furniture)
- Bits 5-6: rotation / variant
- Bit 7: wire overlay

Not enough room for window position + width + door position + width alongside existing uses.

### Option B: Feature Overlay Buffer

A separate per-tile `u8` or `u16` buffer — `wall_features[]` — parallel to `grid_data[]`.

```
For walls with windows:
  bits 0-1: window start position (0-3 along wall edge)
  bits 2-3: window width (0=none, 1-3 sub-cells)
  bits 4-5: door start position (0-3)
  bits 6-7: door width (0=none, 1-3)
```

8 bits per tile. For 256×256 grid = 64KB. Trivial memory cost. Only meaningful for wall tiles — all other tiles have value 0.

Uploaded to GPU as a storage buffer alongside grid_data, terrain_buf, etc.

### Option C: Encode in Height Byte

For wall blocks, the height byte (bits 8-15) stores wall height (1-3 typically). The upper bits are unused. Could pack window info into height bits 4-7 since wall height rarely exceeds 4.

Risky — height is used by many systems. Separate buffer is safer.

**Recommendation: Option B.** Clean separation, no interference with existing data, easy to add/remove.

## Rendering Changes

### Top-Down View

Wall tiles check the feature buffer. If a window or door is present, the corresponding sub-cells render differently:
- **Window sub-cells:** Glass with slight blue tint, specular highlight, frame edge from wall material
- **Door sub-cells (closed):** Wood plank texture, slightly recessed from wall face, handle dot
- **Door sub-cells (open):** Floor visible through the opening
- **Frame sub-cells:** Wall material (stone, wood, granite, etc.) — the block's own `block_base_color()`

### Wall Face View (Oblique South)

The existing glass face rendering (`WINDOW_SILL_FRAC`, `WINDOW_LINTEL_FRAC`) applies only to the sub-cells that are window. Adjacent sub-cells show solid wall material. This creates natural window frames in the face view.

```
Face of a wall with centered 2-wide window:
[stone][glass + frame][glass + frame][stone]
[stone][  sill below ][  sill below ][stone]
```

### Interior Lighting

The sunbeam tracing (`trace_interior_sun_ray`) currently checks `is_glass()` for the entire tile. With wall features, it checks whether the specific sub-cell the ray passes through is a window sub-cell. Partial window walls let some light through and block the rest.

## Shadow Casting / Vision

A wall with a window partially blocks vision:
- Solid sub-cells block vision (wall material)
- Window sub-cells transmit vision (glass — already treated as non-blocking in `blocks_vision()`)
- Door sub-cells: block when closed, transmit when open

The `blocks_vision()` function becomes sub-cell-aware for wall tiles that have features. For most tiles (no features), behavior is unchanged.

## Pathfinding

Only doors affect pathfinding. An open door with width N along a wall edge creates a passable crossing at those N sub-cells. Combined with thin walls (DN-004), the edge-based pathfinding check becomes:

```
Can cross from A to B through shared edge?
  1. Check if wall sub-cells fully block the edge → impassable
  2. Check if there's an open door at the edge → passable
  3. Check if there are non-wall sub-cells (thin wall gap) → passable
```

## Placement UX

### Placing a Window

1. Player selects Window tool
2. Clicks a wall tile
3. System places a centered, 2-wide window (default)
4. Q/E adjusts width (1-4 sub-cells)
5. Shift+Q/E adjusts position along the wall edge

### Placing a Door

1. Player selects Door tool
2. Clicks a wall tile
3. System places a centered, 2-wide door (default)
4. Q/E adjusts width
5. Click door to toggle open/close (existing behavior)

### Removing

Window/door removal restores the wall to solid — the wall block type was never changed, so no material is lost.

## Migration from Current System

- Existing `BT_GLASS` blocks → converted to wall-with-window (using the neighbor detection that `render_glass_block()` already does to determine wall material)
- Existing door blocks → wall type preserved (fix the current bug where door placement replaces material with BT_WALL)
- `BT_GLASS` could remain as a block type for non-wall uses (glass floor? greenhouse panel?) or be deprecated

## Impact Summary

| System | Change | Difficulty |
|--------|--------|------------|
| Wall feature buffer | New per-tile u8 buffer | Low |
| GPU upload | One new storage buffer binding | Low |
| Placement | Feature editing instead of block replacement | Medium |
| Top-down rendering | Sub-cell material switching | Medium |
| Wall face rendering | Partial window/door in face | Medium |
| Interior lighting | Sub-cell-aware ray tracing | Medium |
| Vision/fog | Sub-cell-aware for featured walls | High |
| Pathfinding | Edge-based with door check | Medium (depends on DN-004) |

## Dependencies

- **DN-003** (sub-grid placement): Window/door position uses the same 4×4 sub-grid coordinate system
- **DN-004** (thin walls): Window/door width expressed in sub-cells, wall thickness determines frame depth
- Can be implemented independently of DN-003/DN-004 by treating all current walls as thickness-4 and using sub-cells only for the window/door opening along the wall edge

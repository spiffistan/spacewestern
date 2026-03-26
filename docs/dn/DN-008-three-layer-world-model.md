# DN-008: Three-Layer World Model — Walls, Blocks, Surface Items

## Status: Proposed

## Problem

The world stores one u32 per tile. This creates three colliding limitations:

1. **Walls consume tiles.** A wall and a bench can't share a tile. Thin walls (DN-004) helped by putting the wall on one edge, but the tile's block type is still "wall" — no furniture can be placed there.

2. **One thing per tile.** A bench can't have a cup on it. A floor can't have a dropped tool AND a decoration. The `ground_items` system partially addresses this but only for dropped loot, not placed objects.

3. **Wall encoding gymnastics.** Walls store edge bitmask in the height byte (DN-007), thickness in the flags byte, and material as the block type. This works but is fragile — every system that reads "height" must know to mask wall types differently.

## Solution: Three Independent Layers

```
┌─────────────────────────────────────────────────┐
│ Layer 3: SURFACE ITEMS (sparse)                 │
│   Cup on bench, tool on workbench, candle,      │
│   dropped resources. Multiple per tile.         │
│   Storage: Vec<SurfaceItem> or HashMap          │
├─────────────────────────────────────────────────┤
│ Layer 2: BLOCKS (dense grid, 1 per tile)        │
│   Terrain, floors, furniture, equipment, pipes,  │
│   plants. One block per tile. Defines walkable,  │
│   interactable, functional properties.          │
│   Storage: grid_data[idx] — u32 per tile        │
├─────────────────────────────────────────────────┤
│ Layer 1: WALL EDGES (dense grid, per tile)      │
│   Structural walls on tile boundaries.          │
│   Independent of what's ON the tile.            │
│   Storage: wall_data[idx] — u16 per tile        │
│     bits 0-3: edge bitmask (N/E/S/W)            │
│     bits 4-5: thickness (0=full, 1-3 thin)      │
│     bits 6-8: material (8 wall materials)        │
│     bits 9: has_door                             │
│     bit 10: door_is_open                         │
│     bit 11: has_window                           │
│     bits 12-15: reserved                         │
└─────────────────────────────────────────────────┘
```

## Layer 1: Wall Edges

### What moves here

All structural wall types become wall materials, not block types:
- BT_STONE (1) → wall material 0
- BT_WALL (4) → wall material 1
- BT_GLASS (5) → wall material 2
- BT_INSULATED (14) → wall material 3
- BT_WOOD_WALL (21) → wall material 4
- BT_STEEL_WALL (22) → wall material 5
- BT_SANDSTONE (23) → wall material 6
- BT_GRANITE (24) → wall material 7
- BT_LIMESTONE (25) → wall material 8 (needs 4 bits)
- BT_MUD_WALL (35) → wall material 9
- BT_DIAGONAL (44) → special case (see below)

### Data format: u16 per tile

```
bits 0-3:  edge bitmask (bit0=N, bit1=E, bit2=S, bit3=W)
bits 4-5:  thickness (0=full/4, 1=3, 2=2, 3=1 sub-cell)
bits 6-9:  wall material index (0-15, into a wall material table)
bit 10:    has_door on this tile
bit 11:    door_is_open
bit 12:    has_window (glass section in the wall)
bits 13-15: door/window edge (which edge the door/window is on)
```

If edge bitmask is 0, the tile has no walls. Simple, clean.

### What this solves

- A bench tile can have walls on its north and west edges
- Wall material is decoupled from block type — frees up 11 BT_* slots
- Door and window are wall features, not block type hacks
- The height byte no longer stores edge masks — wall height is fixed per material
- `block_height_rs()` no longer needs special wall masking
- T-junctions and crosses work naturally (just OR the edge bits)

### GPU buffer

```rust
wall_data: Vec<u16>  // 256×256 = 128KB (or u32 for more room)
```

Uploaded as a storage buffer, read by raytrace, shadow_map, lightmap, thermal, sound, fog shaders.

### Rendering

The raytrace shader checks `wall_data[idx]` for edge presence and thickness. Wall material color comes from a small wall material table (10 entries) instead of the full block material table. `pixel_is_wall` reads from `wall_data` instead of the block's height/flags bytes.

### Pathfinding / Edge blocking

`edge_blocked()` reads from `wall_data` instead of the block grid. Much simpler — no need to check `is_wall_block(bt)` or decode edge masks from the height byte.

### Shadow casting

The shadow map shader checks `wall_data` for wall presence at each step. Wall height is a per-material constant (typically 3), not stored per-tile.

## Layer 2: Blocks (simplified)

### What stays

The block grid (`grid_data`) keeps:

**Terrain (ground surface):**
- BT_AIR (0), BT_DIRT (2), BT_WATER (3), BT_DUG_GROUND (32)
- BT_WOOD_FLOOR (26), BT_STONE_FLOOR (27), BT_CONCRETE_FLOOR (28), BT_ROUGH_FLOOR (60)

**Furniture (functional stations, one per tile):**
- BT_BENCH (9), BT_BED (30), BT_CRATE (33), BT_WORKBENCH (57), BT_SAW_HORSE (61)
- BT_KILN (58), BT_WELL (59)

**Equipment (machines, one per tile):**
- BT_FIREPLACE (6), BT_CEILING_LIGHT (7), BT_FLOOR_LAMP (10), BT_TABLE_LAMP (11)
- BT_FAN (12), BT_COMPOST (13), BT_CANNON (29), BT_FLOODLIGHT (48)
- BT_WALL_TORCH (55), BT_WALL_LAMP (56)

**Power grid (height byte = connection mask):**
- BT_WIRE (36), BT_SOLAR (37), BT_BATTERY_S/M/L (38-40)
- BT_WIND_TURBINE (41), BT_SWITCH (42), BT_DIMMER (43), BT_BREAKER (45)
- BT_WIRE_BRIDGE (51)

**Pipe network (height byte = connection mask):**
- BT_PIPE (15), BT_PUMP (16), BT_TANK (17), BT_VALVE (18)
- BT_OUTLET (19), BT_INLET (20), BT_RESTRICTOR (46)
- BT_LIQUID_PIPE (49), BT_PIPE_BRIDGE (50)
- BT_LIQUID_INTAKE (52), BT_LIQUID_PUMP (53), BT_LIQUID_OUTPUT (54)

**Plants:**
- BT_TREE (8), BT_BERRY_BUSH (31), BT_CROP (47), BT_ROCK (34)

### What changes

- The height byte no longer stores wall edge masks (that's in wall_data now)
- Doors and windows are no longer block types — they're wall features
- The flags byte is freed from wall-specific encoding
- `block_height_rs()` returns the raw byte again (no masking needed)
- Placement: `can_place_on()` checks block type AND wall_data (can place furniture on a tile with walls)

### Block format (unchanged)

```
[type:8 | height:8 | flags:8 | roof_height:8]
```

But the height and flags bytes are now simpler — no wall edge encoding gymnastics.

## Layer 3: Surface Items

### What this is

Small objects placed on top of blocks. Multiple items can share a tile. They render visually but don't affect pathfinding, fluid flow, or structural physics.

### Data structure

```rust
struct SurfaceItem {
    x: f32, y: f32,           // world position (sub-tile precision)
    stack: ItemStack,          // what item (uses existing item system)
    placed: bool,              // true = intentional decoration, false = dropped loot
    sub_x: u8, sub_y: u8,     // sub-grid position within tile (0-3 each)
}
```

Storage: `Vec<SurfaceItem>` (sparse — only tiles with items have entries). For fast lookup, a `HashMap<(i32,i32), Vec<usize>>` indexes items by tile.

### What goes here

- Dropped resources (currently `ground_items` — absorbed into this system)
- Decorations: cups, books, candles, paintings, rugs
- Items on furniture: tools on workbench, food on table, lamp on bench
- Small props: bucket by well, plank stack near saw horse
- Signs, labels, markers

### Rendering

Surface items render after blocks, before plebs. Each item type has a small sprite or procedural shape. The existing `ground_items` rendering code is the starting point.

### Interaction

- Click: select the item (shows info panel)
- Right-click: pick up, move, use
- Auto-haul: `placed=false` items get hauled to storage (current behavior)
- `placed=true` items stay put (decorations)

## Migration Plan

### Phase 1: Wall Edge Layer (extract walls from blocks)

**Goal:** Walls are stored in `wall_data`, blocks no longer have wall types.

1. Add `wall_data: Vec<u16>` to App (256×256 = 128KB)
2. Add `wall_buffer: wgpu::Buffer` to GfxState, upload as storage buffer
3. Create wall material table (10 entries: color, height, conductivity, is_transparent)
4. Update `edge_blocked()` to read from `wall_data` instead of `grid_data`
5. Update `pixel_is_wall()` in raytrace.wgsl to read from wall buffer
6. Update `has_wall_on_edge` in all 4 shaders
7. Update shadow_map.wgsl to check wall buffer for shadow casting
8. Update fog.rs FOV to use wall_data
9. Update placement: wall tools write to `wall_data`, not `grid_data`
10. Update demolish: removing a wall edge clears bits in `wall_data`
11. Remove wall block types from `is_wall_block()` — they no longer exist as blocks
12. Tiles under walls revert to their terrain type (BT_DIRT, or whatever floor is there)
13. Update roof detection to use `wall_data` for flood-fill boundaries
14. Update thermal conduction to read wall material from `wall_data`

**Estimated effort:** Large (touches 15+ files, 4 shaders). But each change is mechanical — find wall block reads, redirect to wall_data.

**Backward compatibility:** World gen creates both grid_data and wall_data. Save/load must handle both. Old saves need migration (scan for wall block types, convert to wall_data entries).

### Phase 2: Simplified Block Grid (clean up)

**Goal:** Block types no longer include walls. Height byte is just height.

1. Remove BT_WALL, BT_STONE, BT_GLASS, etc. from block type constants (or keep as legacy)
2. Remove `block_height_rs` masking for wall types
3. Remove `block_height_raw` (no longer needed)
4. Simplify `can_place_on()` — furniture can go on any ground tile regardless of walls
5. Update blocks.toml — wall entries move to a walls.toml or wall material table
6. Free up 11 block type IDs

### Phase 3: Surface Items (add decoration layer)

**Goal:** Small items can be placed on tiles independently of blocks.

1. Convert `ground_items: Vec<GroundItem>` to `surface_items: Vec<SurfaceItem>`
2. Add `placed` flag for decorations vs drops
3. Add item placement UI (pick up item from inventory, click to place)
4. Render surface items with sprites/procedural shapes
5. Update click handling — clicking a surface item selects it
6. Add hauling logic — `placed=false` items auto-hauled, `placed=true` stay

## Diagonal Walls

BT_DIAGONAL (44) is a special case. It's not an edge-based wall — it's a solid triangle within the tile. Options:

1. **Keep as a block type.** Diagonal walls are rendered differently (triangle half) and don't fit the edge bitmask model. They can stay as BT_DIAGONAL in the block grid.

2. **Add diagonal bits to wall_data.** Use 2 reserved bits (13-14) for diagonal variant (0-3). A tile with diagonal wall has a special rendering path.

**Recommendation:** Keep BT_DIAGONAL as a block type for now. It's fundamentally different from edge walls.

## Door and Window Handling

Currently doors are a flag on wall block types. With the wall edge layer:

- `bit 10: has_door` — this edge has a door
- `bit 11: door_is_open` — the door is currently open
- `bits 13-15: door_edge` — which edge the door is on (for tiles with multiple wall edges)
- `bit 12: has_window` — this edge has a window (glass section)

When a tile has a door:
- `edge_blocked()` checks door_is_open — open door doesn't block
- Rendering shows the door frame/handle on the specified edge
- Interaction: click to open/close
- Auto-open when pleb approaches (existing behavior)

Windows:
- Block light partially (glass tint + absorption)
- Block movement always
- Transmit sound partially
- Visual: glass pane in the wall section

## Benefits

### For the player
- Place furniture against walls (chair in the corner of a room)
- Decorate with items on surfaces (cup on table, candle on shelf)
- Walls don't waste tile space — a wall IS a boundary, not an object

### For the developer
- Wall code is isolated in its own layer (no more height byte masking)
- Block types are simpler (no wall-specific encoding)
- Adding new wall materials is trivial (one table entry)
- Adding new surface items is trivial (one item definition)
- No more `is_wall_block()` checks scattered everywhere

### For the renderer
- Wall rendering reads one buffer (wall_data) with consistent format
- Block rendering reads another buffer (grid_data) with consistent format
- No more "is this a wall? decode height differently" branching

## Storage Costs

| Layer | Format | Size | GPU binding |
|-------|--------|------|-------------|
| Wall edges | u16 per tile | 128 KB | 1 storage buffer |
| Blocks | u32 per tile | 256 KB | existing grid_buffer |
| Surface items | sparse Vec | ~1-10 KB typical | upload as needed |

Total additional memory: ~128 KB for walls. Surface items are negligible.

## Open Questions

1. **Wall material count.** 4 bits gives 16 materials. Currently 11 wall types. Enough for now, but if we add more wall materials we may need to expand.

2. **Per-edge material.** Can one tile have a stone wall on north and a wood wall on east? The current design uses one material per tile. Per-edge material would need more bits (4 bits × 4 edges = 16 bits just for materials).

3. **Wall height.** Is wall height always uniform per material (stone=3, wood=3) or can it vary per tile? Uniform is simpler and probably sufficient.

4. **Save format.** Need to store wall_data alongside grid_data. Migration from old saves needs a scan-and-convert pass.

5. **Roof detection.** Currently uses block types. Needs to use wall_data edges for flood-fill boundaries. The roof_height byte stays in the block grid (it's a property of the tile space, not the wall).

# Furniture, Work Surfaces, and Seating

How physical workspace layout affects crafting. Surfaces and seating are separate items that combine — you don't build a "workstation," you place a table, pull up a stool, lay out your tools. The workshop assembles itself from its parts.

## Design Principle

Furniture is **combinatorial**, not monolithic. A workshop is a table + stool + tools. A dining area is a table + chairs. A study is a desk + chair + bookshelf. The game detects what you've made from the arrangement, not from a blueprint.

## Surfaces by Tier

### Tier 0: Natural / Scavenged
Found or trivially made. Crude but functional.

| Item | Size | Tool slots | Speed | Source |
|------|------|-----------|-------|--------|
| Flat rock | 1x1 | 1 | 0.7x | Found in world (natural feature) |
| Log stump | 1x1 | 1 | 0.7x | Left behind when tree is chopped |
| Wreck panel | 1x1 | 2 | 0.8x | Salvaged from crash debris |

A flat rock with a hammerstone is the very first "workshop." No crafting needed — the world provides.

### Tier 1: Primitive (sticks, fiber, rough logs)
Built by hand. Functional but rough.

| Item | Size | Tool slots | Speed | Recipe |
|------|------|-----------|-------|--------|
| Rough bench | 1x1 | 2 | 0.85x | 3 sticks + 1 fiber |
| Plank table | 1x1 | 3 | 1.0x | 2 planks + 2 sticks |
| Corner shelf | corner | 1 | 0.8x | 2 sticks + 1 fiber |
| Drying rack | 1x1 tall | 0 (dedicated) | — | 4 sticks + 2 fiber |

The rough bench doubles as both work surface and seating (uncomfortable, but it works). Plank table is the first "real" furniture — requires saw horse → planks chain.

### Tier 2: Constructed (planks, rope, pegs)
Proper furniture. Requires planks (saw horse) and sometimes rope.

| Item | Size | Tool slots | Speed | Recipe |
|------|------|-----------|-------|--------|
| Work table | 1x1 | 3 | 1.0x | 3 planks + 1 rope |
| Long table | 2x1 | 5 | 1.0x | 5 planks + 2 rope |
| Corner table | L-shape | 3 | 1.0x | 4 planks + 1 rope |
| Wall shelf | wall-mounted | 2 (storage) | — | 2 planks + 1 fiber |
| Tool rack | wall-mounted | 4 (display) | — | 3 sticks + 1 rope |

The long table is a luxury — 5 tool slots means you can have a hammerstone, knife, whetstone, mortar, AND a spindle all on one surface. A dedicated crafting station.

Corner tables snap into inner wall corners. They use dead space that would otherwise be wasted. Good for small workshops.

### Tier 3: Stone / Metal (future)
Heavy, permanent, precise.

| Item | Size | Tool slots | Speed | Recipe |
|------|------|-----------|-------|--------|
| Stone workbench | 1x1 | 4 | 1.1x | 4 stone + 1 plank |
| Metal workbench | 1x1 | 4 | 1.2x | iron + planks |
| Forge table | 1x1 | 2 | 1.0x | stone + metal (heat-resistant) |
| Lab bench | 2x1 | 6 | 1.15x | metal + glass + planks |

Stone and metal benches are heavier (can't be moved easily) but grant speed bonuses. The forge table is heat-resistant — required for metalworking near a forge.

## Seating

Seating is separate from surfaces. A pleb CAN work standing — seating makes them faster and happier.

### Effects
| Seating | Craft speed | Comfort mood | Fatigue | Notes |
|---------|------------|-------------|---------|-------|
| None (standing) | 1.0x | -1 mood/hr | Normal | Works but tiring |
| Ground (sitting on dirt) | 0.9x | -2 mood/hr | Slow | Only if exhausted |
| Log stump | 1.0x | 0 | Normal | Free from tree chopping |
| Rough stool | 1.05x | +1 mood/hr | Reduced | 2 sticks + 1 fiber |
| Chair | 1.10x | +2 mood/hr | Reduced | 2 planks + 1 fiber |
| Padded chair | 1.12x | +3 mood/hr | Low | chair + 2 fabric (future) |

### Seating Items

| Item | Tier | Recipe | Notes |
|------|------|--------|-------|
| Log stump | 0 | — (tree byproduct) | Doubles as 1-slot surface |
| Rough stool | 1 | 2 sticks + 1 fiber | Minimal, portable |
| Bench seat | 1 | 3 sticks + 1 fiber | 2-wide, seats 2 |
| Chair | 2 | 2 planks + 1 fiber | Has back support |
| Rocking chair | 2 | 3 planks + 1 rope | Mood bonus when idle |

### Seating + Surface Pairing

The pleb automatically uses nearby seating when working at a surface. Logic:
1. Pleb walks to work surface to craft
2. Check adjacent tiles for seating (N/S/E/W of surface)
3. If seating found: sit, gain speed + comfort bonuses
4. If no seating: work standing (baseline speed, fatigue accumulates)

No player action needed — plebs prefer sitting when available, like how they prefer beds over ground for sleeping.

## Corner Pieces

Corner surfaces snap to inner wall corners. They detect adjacent wall edges and orient automatically.

```
    ┌────┐
    │    │
    │  ┌─┤  ← corner shelf fits here
    │  │ │
    └──┘ │
         │
```

**Detection**: A corner piece placed at tile (x,y) checks wall_data for edges on two adjacent sides (N+E, N+W, S+E, S+W). If two perpendicular walls are found, the corner piece rotates to fit.

**Uses**:
- Corner shelf: 1 tool slot in dead space (efficient use of small rooms)
- Corner table: L-shaped, 3 tool slots, feels like a real workshop corner
- Corner storage: small crate that fits under a shelf

## Wall-Mounted Items

Shelves and racks mount on walls. They don't occupy floor space.

**Tool rack**: Displays 4 tools on the wall. Tools on a rack are accessible to plebs working at adjacent surfaces — they count as "on the surface" for capability purposes, even though they're stored on the wall.

**Wall shelf**: Stores 2 item stacks. Decorative + functional. Shows items visually (jars, bottles, rocks).

**Implementation**: Wall-mounted items use the wall_data system. They're stored as wall features (like windows/doors in DN-005) on a specific edge. A shelf on the north wall of tile (x,y) is encoded in that tile's wall data.

## Workshop Room Detection

When a room contains:
- At least 1 work surface with tools
- Walls + door + roof (enclosed)

The room system detects it as a **Workshop**. Effects:
- Crafting speed +5% (sheltered, organized)
- Tool wear -10% (tools stored properly, less exposure)
- Mood bonus: "Nice workshop" if quality furniture

If the room also has:
- Seating → "Comfortable workshop"
- Wall shelves → "Organized workshop"
- Multiple surfaces → "Well-equipped workshop"

These stack as room quality modifiers, similar to bedroom quality.

## Visual Design

- Tools on surfaces render as small icons/sprites on the table tile
- Slightly scattered/rotated for natural look (not grid-perfect)
- Seating shows the pleb sitting (different sprite pose than standing)
- Wall racks show tools hanging (silhouettes on the wall)
- Corner pieces have distinct triangular footprint in the build preview

## Open Questions

- Should surfaces be rotatable? A table against a wall vs in the middle of a room?
- Can two surfaces share a stool between them? (Pleb swivels between tasks)
- Should tool placement be drag-and-drop or right-click menu?
- How does this interact with the sub-grid system (DN-003)? Can a small stool fit in a half-tile?
- Should wall shelves count as tool slots for nearby surfaces, or just storage?

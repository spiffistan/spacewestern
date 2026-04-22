# DN-033: Work Surfaces and Tool Placement

**Status:** Draft
**Depends on:** DN-032 (tool components), DN-025 (primitive tools)
**Related:** crafting.md, recipes.toml

## Problem

The current crafting model is Rimworld-style: build a "workbench" block, it unlocks recipes. This is functional but artificial — a workbench is a magic box, not a workspace. It doesn't match the Primitive Technology feel where you assemble capability from real objects.

## Core Concept: Surfaces + Tools = Capability

A **work surface** (table, bench, flat rock) is just a place to work. It provides a speed bonus and stability. What you can DO there depends on what **tools are placed on it**.

### How It Works

1. **Place a surface** (table, bench, stone slab). It has N tool slots (2-4 depending on size).
2. **Place tools on it** by right-click → "Place on surface" or drag-drop. Tools appear visually on the table.
3. **Capabilities emerge** from the combination of surface + tools. A table with a hammerstone enables knapping. Add a whetstone and it also enables sharpening.
4. **Recipes check capabilities**, not station types. "Craft Stone Axe" requires the `knapping` capability. The recipe doesn't care if it comes from a workbench, a flat rock with a hammerstone, or a fully-equipped workshop.

### Capability Tags

Each tool placed on a surface contributes one or more capability tags:

| Tool on surface | Capabilities granted |
|---|---|
| Hammerstone | `knapping`, `sharpening` (slow) |
| Whetstone | `sharpening` (fast) |
| Knife/blade | `cutting`, `carving` |
| Mortar & pestle | `grinding` |
| Clay mold | `shaping` |
| Drop spindle | `spinning` |
| Needle & awl | `sewing` |
| Saw (on saw horse) | `sawing` |

### Multi-Capability with Efficiency Penalty

A single tool CAN provide multiple capabilities, but with reduced efficiency for secondary ones:

- Hammerstone on table: `knapping` at 100%, `sharpening` at 60%
- Dedicated whetstone: `sharpening` at 100%

This means early game you get by with fewer tools (a hammerstone does everything poorly), but specialization rewards investment. The player naturally upgrades: "my hammerstone sharpens slowly... I should make a whetstone."

### Recipe Capability Requirements

```toml
[[recipe]]
id = 8
name = "Stone Axe"
station = "surface"       # any work surface (was "hand" or "workbench")
capabilities = ["knapping"]
time = 6.0
inputs = [{item = 201, count = 1}, {item = 204, count = 1}, {item = 202, count = 1}]
output = {item = 500, count = 1}
```

Some recipes require multiple capabilities:
```toml
[[recipe]]
name = "Leather Strip"
station = "surface"
capabilities = ["cutting", "scraping"]
```

The player needs BOTH a knife and a hide scraper on the same table. This creates meaningful workshop layout decisions.

## Visual Placement System

### "Loud" Placement Indicators

When the player picks up a tool or enters placement mode:

1. **Valid surfaces pulse** with a soft highlight (warm amber glow on tables/benches that have empty slots)
2. **Ghost outlines** appear on empty table slots showing where the tool would sit
3. **Capability preview**: hovering over a surface shows what capabilities it currently has AND what adding this tool would unlock: "Adding Hammerstone: +knapping, +sharpening (slow)"
4. **New recipe notification**: if placing a tool would unlock a new recipe, show a brief "NEW: Stone Axe now craftable" indicator

### Tool Icons on Surfaces

Tools placed on surfaces are visible in the world:
- Small sprite/icon rendered on the table tile
- Rotated slightly for natural look (not grid-aligned)
- Click to pick up / right-click for options (remove, swap)
- Tooltip on hover: "Hammerstone — provides: knapping, sharpening (60%)"

## Surface Types

| Surface | Slots | Speed bonus | Notes |
|---|---|---|---|
| Flat rock (natural) | 1 | 0.7x | Found in world, free |
| Rough bench | 2 | 0.85x | 2 sticks + 1 fiber |
| Wooden table | 3 | 1.0x | 2 planks + 2 sticks |
| Stone workbench | 4 | 1.1x | 4 rock + 1 plank |
| Metal workbench | 4 | 1.2x | iron + planks (future) |

Bigger/better surfaces = more tool slots = more simultaneous capabilities. A wooden table with hammerstone + knife + whetstone is a proper workshop.

## Standalone Stations

Some activities need dedicated structures, not table-tools:

| Station | Why standalone |
|---|---|
| Kiln | Too hot, fire hazard, needs chimney |
| Charcoal mound | Smoky, outdoor only |
| Saw horse | Too tall, needs its own frame |
| Drying rack | Needs airflow, vertical |
| Tanning rack | Smelly, outdoor preferred |
| Smoking rack | Needs heat + smoke proximity |
| Forge/anvil | Heavy, hot, needs fuel source |

These remain as placed blocks with built-in capability.

## Interaction with Existing Systems

- **DN-032 (tool components)**: Tools on surfaces wear down from use. A hammerstone used for knapping at the workbench loses durability just like one used in the field.
- **DN-025 (primitive tools)**: The tier 0→1→2 tool progression maps naturally to capability. Better tools = faster capability (flint blade cuts faster than stone blade).
- **Thermal system**: A forge near a heat source works better. A drying rack in wind dries faster. Physics-driven efficiency.
- **Room system**: An enclosed workshop with roof gets a cleanliness/quality bonus. Outdoor crafting works but with weather penalties.

## Migration Path

Current system → new system:

1. **Phase 1**: Keep existing `station = "workbench"` recipes working. Add `station = "surface"` as new option. Both work.
2. **Phase 2**: Make BT_WORKBENCH a surface with 3 built-in tool slots (auto-populated with hammerstone + knife). Existing gameplay unchanged.
3. **Phase 3**: Add tool placement UI. New recipes use capabilities. Old recipes still work via workbench's built-in tools.
4. **Phase 4**: Remove `station = "workbench"` — everything is capability-based.

## What the Player Sees

**Early game**: Right-click near pleb → hand-craft recipes (no surface needed for simple stuff like hammerstone). Build a rough bench, place hammerstone on it → knapping recipes appear. Place a knife too → cutting recipes appear.

**Mid game**: Wooden table with 3 tools = proper workshop. Player experiments: "what if I put the mortar here?" New recipes appear. The workshop grows organically through tool collection.

**Late game**: Stone workbench with 4 specialized tools. Each tool is high-quality (skilled crafter made them). Crafting speed and quality reflect both the surface and the tools on it.

The workshop isn't BUILT — it EVOLVES. Every tool placed is a small investment. The player's workspace tells a story of their progression.

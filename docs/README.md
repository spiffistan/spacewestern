# Docs Structure

## Root Documents

| File | Purpose |
|------|---------|
| `SPEC.md` | Full game specification — systems, mechanics, world rules |
| `MASTER_SPEC.md` | High-level vision and design pillars |
| `PLAN.md` | Implementation roadmap and feature tracking |
| `CRAFTING.md` | Crafting chain diagram and recipe flow |
| `AI_SYSTEM.md` | Pleb AI, pathfinding, needs, scheduling |
| `PHYSICS_SYSTEM.md` | Thermal, fluid, electrical simulation details |

## Design Notes (`dn/`)

Numbered technical documents for specific architectural decisions. Format: `DN-NNN-short-name.md`. Each has a Status (Proposed / Accepted / Implemented / Superseded).

| File | Status | Topic |
|------|--------|-------|
| `DN-001-blender-sprite-pipeline.md` | Proposed | Blender-to-heightmap-sprite asset pipeline |
| `DN-002-block-vs-surface-items.md` | Proposed | When to use block types vs surface item entities |
| `DN-003-subgrid-placement.md` | Proposed | 4×4 sub-grid for finer placement snapping of multi-tile objects |
| `DN-004-thin-walls.md` | Proposed | Variable-thickness walls with sub-grid furniture placement |
| `DN-005-windows-doors-as-wall-features.md` | Proposed | Windows and doors as wall attributes, not separate block types |
| `DN-006-subgrid-system-implications.md` | Proposed | Full audit of every system affected by the sub-grid architecture |

## Ideas (`ideas/`)

Early-stage design exploration. Not commitments — creative brainstorming for future directions.

| File | Topic |
|------|-------|
| `CHARGEN.md` | "The Manifest" — frontier crew recruitment card system |
| `CARDS.md` | Cards as narrative meta-layer: events, blueprints, abilities, crises |
| `ARTSTYLE.md` | Six art direction options (Dust & Iron, Moebius, Dime Novel, etc.) |
| `SPRITES.md` | Practical sprite design: what needs sprites, production approaches |
| `WALLS_AND_GROUND.md` | Wall and ground rendering assessment — procedural vs sprites |

## Reference (`fluid_mechanics/`)

| File | Topic |
|------|-------|
| `INSPIRATON.md` | Fluid mechanics reference and inspiration |

## Conventions

- **Design notes** are for concrete technical decisions that affect architecture. They get a DN number.
- **Ideas** are for creative exploration with no commitment. They may never be built.
- **Root docs** are living documents that track the current state of the game design.
- When a design note is implemented, update its Status field.
- When an idea graduates to implementation, create a design note or update the relevant root doc.

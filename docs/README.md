# Docs

All documentation uses **lowercase-kebab-case.md** naming. `README.md` is the only exception (convention).

## Root Documents

Living documents for the current game state.

| File | Purpose |
|------|---------|
| `spec.md` | Full game specification — systems, mechanics, world rules |
| `master-spec.md` | High-level vision and design pillars |
| `plan.md` | Implementation roadmap and feature tracking |
| `crafting.md` | Crafting chain diagram and recipe flow |
| `crafting-tree.md` | Mermaid diagram: full crafting/building dependency tree |
| `ai-system.md` | Pleb AI, pathfinding, needs, scheduling |
| `physics-system.md` | Thermal, fluid, electrical simulation details |
| `game-design.md` | Core game design document |
| `sprites.md` | Sprite rendering and asset pipeline |
| `tree-palette.md` | Tree color palette reference |
| `oak-build.md` | Oak build system notes |

## Design Notes (`dn/`)

Numbered technical decisions. Format: `DN-NNN-short-name.md`.

| File | Status | Topic |
|------|--------|-------|
| `DN-001-blender-sprite-pipeline.md` | Proposed | Blender-to-heightmap-sprite asset pipeline |
| `DN-002-block-vs-surface-items.md` | Proposed | When to use block types vs surface item entities |
| `DN-003-subgrid-placement.md` | Proposed | 4x4 sub-grid for finer placement snapping |
| `DN-004-thin-walls.md` | Proposed | Variable-thickness walls with sub-grid furniture placement |
| `DN-005-windows-doors-as-wall-features.md` | Proposed | Windows and doors as wall attributes, not separate blocks |
| `DN-006-subgrid-system-implications.md` | Proposed | Full audit of systems affected by sub-grid architecture |
| `DN-007-building-patterns.md` | Analysis | Allowed building patterns — corners, T-junctions, double walls |
| `DN-008-three-layer-world-model.md` | Proposed | Three-layer architecture: wall edges, blocks, surface items |
| `DN-009-pleb-sprites.md` | Proposed | Layered sprite compositing for colonist rendering |
| `DN-010-discovery-layer.md` | Proposed | Hidden features: creature dens, minerals, artifacts |
| `DN-011-combat-rework.md` | Proposed | Kinematic bullets, spatial hash collision, creature hits |
| `DN-012-wound-system.md` | Proposed | Anatomical hit locations, bleeding, infection, treatment |
| `DN-013-communication-flocking.md` | Implemented | Pleb shout communication + boids-like group movement |
| `DN-014-tall-grass.md` | Proposed | Vegetation as gameplay: concealment, fire fuel, harvesting |
| `DN-015-leaders-and-ranks.md` | Implementing | Leaders, combat ranks, command shouts, morale aura |
| `DN-016-terrain-elevation-and-water.md` | Proposed | 1024x1024 elevation, GPU water flow, digging, moats |

## Ideas (`ideas/`)

Early-stage exploration. Not commitments — creative brainstorming.

| File | Topic |
|------|-------|
| `chargen.md` | "The Manifest" — frontier crew recruitment card system |
| `cards.md` | Cards as narrative meta-layer: events, blueprints, abilities, crises |
| `artstyle.md` | Six art direction options (Dust & Iron, Moebius, Dime Novel, etc.) |
| `sprites.md` | Practical sprite design: what needs sprites, production approaches |
| `walls-and-ground.md` | Wall and ground rendering assessment — procedural vs sprites |
| `multi-level.md` | Underground levels: depth as difficulty, fluid coupling, UX |
| `character-visuals.md` | Character visuals: wear, emotes, relationships, scars, paper doll |
| `gameplay-systems.md` | Day/night, weather, sound, ruins, trade, seasons, fire, reputation |
| `deeper-systems.md` | Food/cooking, medicine, knowledge, psychology, aesthetics, comms |
| `philosophy.md` | Design philosophy: permanence, silence, loneliness, scarcity |
| `combat.md` | Real-time tactical combat: RTwP, breaching, cover, suppression |
| `alien-fauna.md` | Nocturnal alien creatures: duskweavers, thermogasts, glintcrawlers, hollowcalls, mistmaws, borers. Sound sourcing, system hooks. |
| `emergent-physics.md` | 20+ ideas exploiting the physics stack: acoustic ecology, pressure traps, echo location, fire whirls, condensation, erosion, pipe organ. |
| `the-human-layer.md` | Narrative/psychology/culture: planet lore, radio, dreams, moral drift, naming, superstition, scrap economy, saloon, silence, letters. |
| `lore-and-research.md` | World lore, history, research mechanics |
| `asides.md` | Misc design asides (thumper, drones, sunskirter) |
| `tall-grass.md` | Tall grass terrain type design |
| `water-flow.md` | Water flow, pooling, rain, rivers |
| `subgrid-features.md` | Sub-grid feature ideas |
| `group-ai-roadmap.md` | Squad roles, morale contagion, surrender, overwatch, radio/horn |
| `compound-defenses.md` | Palisades, earthworks, platforms, moats, self-sustaining compound design |

## Naming Convention

- All files: `lowercase-kebab-case.md`
- Design notes: `DN-NNN-short-name.md` (numbered, sequential)
- Ideas: descriptive kebab-case, no numbering
- Root docs: short descriptive names

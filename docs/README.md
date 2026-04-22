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
| `DN-017-fauna-and-food.md` | Implementing | Passive wildlife, hunting, trapping, fishing, cooking, early food loop |
| `DN-018-equipment-system.md` | Proposed | Layered body equipment: belt/vest/pack grids, Diablo-style sizing, pouches for tiny items, auto-deploy |
| `DN-019-knowledge-and-crafting.md` | Proposed | Knowledge gradient (6 levels), three-lock crafting, social transfer, deception, per-map calibration, domain dependencies |
| `DN-020-traits-and-aptitudes.md` | Draft | Hidden traits, aptitudes, diverging from Rimworld skill model |
| `DN-021-ui-philosophy.md` | Proposed | World-as-interface: thermal tinting, body language, diegetic elements, silhouettes, radial menus, tiered notifications |
| `DN-022-skill-scale.md` | Draft | 0.0-10.0 skill scale, exponential XP curve, aptitude ceilings, failure rates, decay |
| `DN-023-sub-tile-mining.md` | Draft | 8x8 sub-cell mining grid, mineral veins, directional carving, geology skill, structural rock types |
| `DN-024-terrain-and-wilderness.md` | Draft | Three-zone map (clearing/margins/deep forest), glades, game trails, geological zones, alien flora integration |
| `DN-025-primitive-tools.md` | Draft | Hands → stone → flint → metal tool chain, durability, knapping, in-hand crafting, mining speed gates |
| `DN-026-world-lore-and-discovery.md` | Draft | World reveals itself through lore stages: survival → adaptation → comprehension → mastery → ancient. Per-pleb knowledge, lore journal, label progression |
| `DN-027-discovery-feel.md` | Draft | Two-tier discovery: silent passive identification (common) vs conditional moments (rare). No markers, no examine action, serendipity over system |
| `DN-028-wall-conduits.md` | Draft | Wire/gas/liquid pipes inside walls, thickness-gated capacity, wall outlets/vents/taps, overlay X-ray view |
| `DN-029-wall-types-and-progression.md` | Draft | 14 wall types from brush fence to resonite, biome-driven building, thermal/structural/conduit trade-offs, palisade defense system |
| `DN-030-terrain-types.md` | Draft | New terrain types: mineral-stained soil (mining clues), leaf litter (forest floor), sand (glass-making), volcanic (future) |
| `DN-031-crash-salvage.md` | Draft | Ship types, crash landing, salvage crates, artifacts (named unique items), condition randomization, wreck as shelter |
| `DN-032-tool-components.md` | Draft | Tools as assemblies (head/handle/binding), component-based failure/repair, auto-maintenance, upgrade-by-replacement |
| `DN-033-work-surfaces.md` | Draft | Surfaces + tools = capability, efficiency penalties for multi-use, visual placement system, migration from station types |
| `DN-034-knowledge-and-ledger.md` | Draft | Three knowledge states (known/noticed/unknown), colony ledger UI, heatmap, per-pleb knowledge, progressive craft menu |

## Ideas (`ideas/`)

Early-stage exploration. Not commitments — creative brainstorming.

| File | Topic |
|------|-------|
| `chargen.md` | "The Manifest" — frontier crew recruitment card system |
| `cards.md` | Cards as narrative meta-layer: events, blueprints, abilities, crises |
| `artstyle.md` | Six art direction options (Dust & Iron, Moebius, Dime Novel, etc.) |
| `sprites.md` | Practical sprite design: what needs sprites, production approaches |
| `exploration-directions.md` | Future directions: fire infrastructure, seasons, night ecology, trade, underground, relationships |
| `early-game-flow.md` | WIP: how the first 30 days unfold — crash, tools, shelter, food, exploration, building progression |
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
| `food-and-survival.md` | Alien crops, physics-driven spoilage, smokehouse as fluid sim showcase, cooking as heat transfer, cold chain engineering, hunting, fermentation, seasonal pressure. |
| `lore-and-research.md` | Knowledge economy: observation, experimentation, lore items, library, trading knowledge, apprenticeship, unreliable narrators. |
| `social-knowledge.md` | Social knowledge transfer: 6-level gradient, conversation mechanics, lies and deception, information veracity, chat bubbles, randomness, dependencies. |
| `world-and-seasons.md` | Two-scale world (512×512 colony + hex world map), four seasons across all systems, expeditions, outposts, weather, living ecosystem, terrain memory, visual mood. |
| `the-setting.md` | The wider universe: the Reach, the charters, the Perdition, the Silence, surviving settlements, hostile factions, the ancient layer, and how the lore enters gameplay through radio, traders, letters, and ruins. |
| `light-and-sound.md` | Light and sound as twin gameplay senses: colored light as material language, the lightline, glass/lenses/mirrors, acoustic archaeology, the hollowcall as standing wave, noise discipline, music as physics, the night duality. |
| `asset-pipeline.md` | Production tools: AI sound (ElevenLabs, Stable Audio, AudioCraft), AI sprites (PixelLab, Sprixen), AI music (Suno, Soundverse), free libraries (Freesound CC0), and the hybrid AI→refine workflow. |
| `asides.md` | Misc design asides (thumper, drones, sunskirter) |
| `tall-grass.md` | Tall grass terrain type design |
| `water-flow.md` | Water flow, pooling, rain, rivers |
| `subgrid-features.md` | Sub-grid feature ideas |
| `group-ai-roadmap.md` | Squad roles, morale contagion, surrender, overwatch, radio/horn |
| `compound-defenses.md` | Palisades, earthworks, platforms, moats, self-sustaining compound design |
| `sharpening-and-maintenance.md` | Whetstone, grinding wheel, sharpness vs durability split, workbench repair |
| `emergent-crafting.md` | Per-map material calibration, serendipitous discovery, skill-gated perception, cross-pleb knowledge |
| `furniture-and-surfaces.md` | Work surfaces by tier, seating speed/comfort, corner pieces, wall-mounted racks, workshop room detection |

## Naming Convention

- All files: `lowercase-kebab-case.md`
- Design notes: `DN-NNN-short-name.md` (numbered, sequential)
- Ideas: descriptive kebab-case, no numbering
- Root docs: short descriptive names

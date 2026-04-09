# DN-029: Wall Types and Building Progression

**Status:** Draft
**Depends on:** DN-004 (thin walls), DN-025 (primitive tools), DN-028 (wall conduits)
**Related:** DN-024 (terrain/wilderness), blocks.toml

## Problem

The game currently has a few wall types that function as a linear upgrade path (wattle → wood → stone → insulated). Real frontier building is about trade-offs: what's available, what's fast, what suits the climate, what you can maintain. A desert colony and a forest colony should build differently — not because of arbitrary tech gates but because different materials make sense in different places.

## Design Principles

1. **No wall is strictly better.** Each has a niche defined by cost, build speed, durability, thermal properties, conduit capacity, and aesthetics.
2. **Biome shapes building style.** Available materials determine what you build. Near forest: log and timber. Near rock: dry stone and mortared. Near clay: adobe and brick. This creates regional identity.
3. **Upgrade isn't replacement.** You don't tear down all your wattle to build stone. Wattle stays as interior partitions, storage walls, temporary shelters. Each tier has permanent utility.
4. **Building takes real time.** A mortared stone wall takes much longer than wattle. A rammed earth wall takes forever but lasts forever. Speed vs permanence is a real trade-off.

## The Wall Types

### Tier 0: Emergency (day 1, bare hands)

#### Brush Fence
- **Recipe:** 3 thornbrake or 5 sticks
- **Build time:** 3 seconds
- **Height:** Low (1). Not a real wall — a barrier.
- **Properties:** Blocks creature pathing. Does not support roof. Does not block wind/weather. Flammable. Degrades rapidly (~10 game-days).
- **Conduits:** None
- **Mood:** None (outdoor furniture, not shelter)
- **Use case:** Night-one perimeter. Place it around camp to slow duskweavers while you build real shelter. Disposable.
- **Visual:** Tangled dark sticks and thorns, irregular silhouette

#### Wattle & Daub (exists: BT_MUD_WALL)
- **Recipe:** 2 sticks + mud (auto-dug)
- **Build time:** 2 seconds
- **Height:** Full wall (3)
- **Properties:** Solid wall. Supports roof. Blocks wind. Degrades in heavy rain over time (surface erosion). Flammable (the stick frame, not the mud). Low structural HP.
- **Conduits:** None — solid mud fill, no cavities
- **Thermal:** Heat capacity 4.0, conductivity 0.006. Moderate — better than nothing, worse than stone.
- **Mood:** -1 (crude housing)
- **Use case:** First real shelter. Cheap, fast, functional. Replace when you can.
- **Visual:** Brown-ochre with visible stick texture underneath

### Tier 1: Permanent Primitive (day 5-15, basic tools)

#### Adobe / Mud Brick
- **Recipe:** 3 clay (sun-dried, no kiln needed)
- **Build time:** 4 seconds (bricks must dry first — delay between placement and completion?)
- **Height:** Full wall (3-4)
- **Properties:** Does NOT degrade in rain (bricks are dried before building). Good structural HP. Not flammable.
- **Conduits:** None — solid brick, no cavities
- **Thermal:** Heat capacity 7.0 (excellent), conductivity 0.003. Best passive climate regulation at this tier. Thick adobe walls keep interiors cool during day and warm at night.
- **Mood:** 0 (neutral — solid but plain)
- **Use case:** The desert/plains smart wall. If you have clay access, adobe is superior to wattle in every way except build speed. Climate-optimized shelter.
- **Visual:** Tan-orange with visible brick courses. Slightly irregular. Warm and organic.

#### Log Palisade
A palisade is sharpened logs driven vertically into the ground. It comes in two heights, creating a defensive progression:

**Low Palisade (half-height)**
- **Recipe:** 2 logs
- **Build time:** 3 seconds
- **Height:** Low (1-2). Half wall.
- **Properties:** Creatures can't path through but can attack over. Plebs can see and shoot over. Does not support roof. Not a sealed wall — wind passes through gaps between logs. Flammable.
- **Conduits:** None
- **Mood:** None (exterior defense)
- **Use case:** Quick perimeter upgrade from brush fence. You can shoot duskweavers from behind it. Defines your territory without fully enclosing it. Think frontier ranch fence — functional, not comfortable.
- **Visual:** Vertical logs, roughly hewn, pointed tops visible from above. Gaps between logs visible. Darker than walls, raw wood color.

**High Palisade (full-height)**
- **Recipe:** 4 logs + 1 rope (to lash the top rail)
- **Build time:** 6 seconds
- **Height:** Full wall (3-4)
- **Properties:** Full barrier — creatures can't see through, can't attack over. Blocks wind partially (gaps between logs reduce insulation). Strong structurally (logs are thick). Flammable. Supports a crude walkway/platform on top (future: plebs can patrol the wall top).
- **Conduits:** Wire only (thread between logs)
- **Thermal:** Heat capacity 3.0, conductivity 0.008 (poor — gaps let heat escape). Better than no wall, worse than mud.
- **Mood:** 0 (secure but rough)
- **Use case:** The frontier fort. Strong perimeter when you expect attacks. Not comfortable to live in — cold, drafty, and the gaps whistle in wind. Build palisade for the perimeter, wattle/adobe for the living quarters.
- **Visual:** Tall vertical logs, close-packed but with narrow dark gaps. Pointed tops cast small shadow teeth on the tile behind. Raw wood grain. Imposing from outside, rough from inside.

**Palisade design notes:**

The two-tier palisade system (low → high) creates a natural defense progression:
1. Brush fence (hour 1 — slows creatures)
2. Low palisade (day 2 — blocks pathing, allows shooting over)
3. High palisade (day 5 — full barrier, secure compound)

Each is a meaningful upgrade, not a waste of the previous. The brush fence stays inside the palisade as thornbrake planting (creature deterrent). The low palisade becomes the inner compound fence while the high palisade forms the outer wall.

A full palisade perimeter is EXPENSIVE — 4 logs per tile means a 20-tile perimeter costs 80 logs (16 trees). This is a serious commitment that competes with other log uses (charcoal, building, fuel). The player must decide: "Do I fully enclose, or leave gaps with brush fence and accept the risk?"

The palisade is specifically a DEFENSE structure, not a house wall. You wouldn't build a bedroom from logs standing vertically — that's what wattle, adobe, and timber frame are for. The palisade is the outer shell. Different wall types for different purposes.

#### Dry Stone
- **Recipe:** 3 rock (no mortar, just stacking)
- **Build time:** 4 seconds
- **Height:** Full wall (3)
- **Properties:** Decent strength. Gaps between stones let wind through (poor insulation but passive ventilation). Not flammable. Does not degrade.
- **Conduits:** Wire at thickness 3+ (thread through gaps)
- **Thermal:** Heat capacity 6.0 (good mass), conductivity 0.010 (leaky — gaps). Stores heat well but loses it through gaps. Best in mild climates, poor in extreme cold/heat.
- **Mood:** 0 (solid but drafty)
- **Use case:** If you're near rock and far from trees. Permanent, no maintenance, no fire risk. The gaps are a feature in hot climates (ventilation) and a bug in cold ones.
- **Visual:** Grey irregular stone shapes fitted together. Dark gaps between stones visible. Uneven top edge.

### Tier 2: Constructed (day 15-30, kiln, lime, metal tools)

#### Rammed Earth
- **Recipe:** 0 materials (uses ground itself) + 2 planks (for forms, recovered after)
- **Build time:** 10 seconds (longest build time of any wall — ramming is labor-intensive)
- **Height:** Full wall, always thick (3-4 minimum)
- **Properties:** Extremely thick. Best thermal mass in the game. Very strong structurally. Not flammable. Does not degrade. But: cannot be thin (minimum thickness 3). Slow to build. Slow to demolish.
- **Conduits:** None — solid compressed earth, no cavities
- **Thermal:** Heat capacity 9.0 (highest), conductivity 0.002 (excellent). The earth ship wall — interior temperature barely fluctuates regardless of exterior.
- **Mood:** +1 (substantial, grounded feeling)
- **Use case:** The ultimate passive climate wall. If you have time and don't need conduits, rammed earth is unmatched. Perfect for sleeping quarters — cool in summer heat, warm in winter cold, zero fuel cost. But you can never run pipes through it.
- **Visual:** Horizontal layers visible (each rammed course), varying earth tones from different soil layers. Thick, monolithic appearance.

#### Timber Frame
- **Recipe:** 3 planks + joinery (needs saw horse)
- **Build time:** 5 seconds
- **Height:** Full wall (3-4)
- **Properties:** Strong. Flammable. Supports heavy roofs. Natural cavities between studs.
- **Conduits:** Full capacity — wire, gas, liquid at appropriate thicknesses. THE infrastructure wall.
- **Thermal:** Heat capacity 3.0, conductivity 0.005 (moderate). Better than palisade (sealed), worse than adobe/stone (less mass).
- **Mood:** +1 (proper construction)
- **Use case:** Any room that needs plumbing, electrical, or gas. Kitchen, workshop, bathroom (future). The functional wall — not the prettiest, not the strongest, but it carries your infrastructure.
- **Visual:** Visible post-and-beam structure with panel infill. Cleaner than wattle, warmer than stone.

#### Mortared Stone
- **Recipe:** 3 rock + 1 lime mortar
- **Build time:** 6 seconds
- **Height:** Full wall (3-4)
- **Properties:** Very strong. Weatherproof. Not flammable. Permanent.
- **Conduits:** Wire + liquid at thickness 3+ (chiseled channels). No gas (too rigid to seal).
- **Thermal:** Heat capacity 8.0 (excellent), conductivity 0.003 (good). Massive thermal flywheel — takes hours to warm or cool.
- **Mood:** +2 (solid, permanent, reassuring)
- **Use case:** The fortress. Perimeter walls, defensive structures, any room that must last and resist damage. Pairs with timber frame interiors (stone outside, wood inside for conduits).
- **Visual:** Fitted stone with visible mortar lines. More regular than dry stone. Imposing.

### Tier 3: Industrial (day 30+, advanced materials)

#### Fired Brick
- **Recipe:** 3 fired brick (clay → kiln) + 1 lime mortar
- **Build time:** 5 seconds
- **Height:** Full wall (3-4)
- **Properties:** Strong, uniform, beautiful. Weatherproof. Not flammable.
- **Conduits:** Wire + liquid at thickness 2+, gas at thickness 3+. Brick has designed cavities (frog bricks).
- **Thermal:** Heat capacity 6.0, conductivity 0.004 (good balance). Not as massive as stone or rammed earth, but well-insulated.
- **Mood:** +3 (the highest natural-material mood bonus — "proper civilization")
- **Use case:** The prestige wall. Plebs living in fired brick rooms get a significant mood boost. It's the first wall that feels like HOME, not just shelter. Use for bedrooms, common rooms, anywhere morale matters.
- **Visual:** Regular brick courses in warm red-orange. Clean mortar lines. Uniform and tidy. The visual marker of "this colony has arrived."

#### Insulated Wall (exists: BT_INSULATED)
- **Recipe:** 2 planks + 2 clay + fill material
- **Build time:** 6 seconds
- **Height:** Full wall (3-4)
- **Properties:** Double-layer construction. Best thermal insulation (low conductivity). Full conduit capacity.
- **Conduits:** Full — wire, gas, liquid at all thicknesses. Designed for infrastructure.
- **Thermal:** Heat capacity 5.0, conductivity 0.001 (lowest — best insulation). The wall for extreme climates.
- **Mood:** +2 (comfortable, climate-controlled)
- **Use case:** Medical bay, nursery (future), any room with critical temperature requirements. Or: extreme climate maps where survival depends on insulation.

#### Glass Wall (exists: BT_GLASS)
- **Recipe:** Glass panes (sand + kiln)
- **Build time:** 4 seconds
- **Properties:** Light transmission. Fragile. No conduits. Poor insulation.
- **Conduits:** None
- **Mood:** +1 (light and views)
- **Use case:** Greenhouse. Light wells. Aesthetic. Not structural.

### Special: Alien Materials

#### Hearthstone Wall
- **Recipe:** Hearthstone blocks (mined from geothermal deposits)
- **Properties:** Naturally warm — radiates 3-5°C above ambient permanently. No fuel needed. Moderate structural strength.
- **Conduits:** None (alien crystalline structure)
- **Mood:** +3 (warm, exotic, comforting)
- **Use case:** Bedrooms in cold climates. The holy grail of passive heating. Extremely rare — finding hearthstone is a major discovery.

#### Resonite Wall
- **Recipe:** Resonite blocks (mined from deep formations)
- **Properties:** Vibrates when disturbed. Natural intruder alarm — any creature touching the wall creates an audible tone. Duskweavers avoid resonite surfaces.
- **Conduits:** None (vibration would interfere with pipes)
- **Mood:** +2 (safe feeling, but slightly eerie humming)
- **Use case:** Perimeter defense. A resonite wall section at your gate means you hear anything approaching. Strategic, not aesthetic.

## Property Comparison Table

| Wall | HP | Thermal mass | Insulation | Conduits | Mood | Burns? | Rain decay? |
|------|-----|-------------|------------|----------|------|--------|------------|
| Brush fence | 10 | — | None | None | 0 | Yes | Yes (fast) |
| Wattle & Daub | 30 | Medium | Poor | None | -1 | Frame | Yes (slow) |
| Adobe | 60 | High | Good | None | 0 | No | No |
| Low Palisade | 50 | Low | None (open) | None | 0 | Yes | No |
| High Palisade | 80 | Low | Poor (gaps) | Wire | 0 | Yes | No |
| Dry Stone | 70 | High | Poor (gaps) | Wire (3+) | 0 | No | No |
| Rammed Earth | 100 | Highest | Excellent | None | +1 | No | No |
| Timber Frame | 60 | Medium | Moderate | Full | +1 | Yes | No |
| Mortared Stone | 120 | High | Good | Wire+Liq (3+) | +2 | No | No |
| Fired Brick | 90 | Medium | Good | Most | +3 | No | No |
| Insulated | 70 | Medium | Best | Full | +2 | No | No |
| Glass | 20 | Low | Poor | None | +1 | No | No |
| Hearthstone | 80 | High | Good + heat | None | +3 | No | No |
| Resonite | 60 | Low | Moderate | None | +2 | No | No |

## Biome-Driven Building Patterns

The available materials shape each colony's architectural identity:

**Forest colony:** Wattle → log palisade (perimeter) + timber frame (rooms) → fired brick accents
- Abundant wood. Short on stone. Timber frame everywhere. Fire risk is the enemy.

**Plains/desert colony:** Wattle → adobe → rammed earth (sleeping) + timber frame (workshop)
- Clay abundant. Trees scarce (logs are precious). Adobe and rammed earth dominate. Thermal mass is king.

**Mountain colony:** Wattle → dry stone → mortared stone (once lime is found) + timber frame (conduit rooms)
- Rock everywhere. Trees limited. Stone walls with timber interiors for infrastructure.

**Mixed (central clearing):** Wattle → whatever's nearest → eventually all types
- The starting position has some of everything. The player chooses based on preference and priorities.

## Implementation Notes

### New block types needed

| Block ID | Name | Category |
|----------|------|----------|
| 71 | Brush Fence | Survival |
| 72 | Adobe Wall | Shelter |
| 73 | Low Palisade | Shelter |
| 74 | High Palisade | Shelter |
| 75 | Dry Stone Wall | Shelter |
| 76 | Rammed Earth | Shelter |
| 77 | Timber Frame | Shelter |
| 78 | Mortared Stone | Shelter |
| 79 | Fired Brick | Shelter |

Existing types adjusted:
- BT_MUD_WALL (35): stays as Wattle & Daub
- BT_INSULATED (14): stays
- BT_GLASS (5): stays
- BT_STONE (1) / BT_WALL (4): could map to dry stone or mortared stone depending on recipe

### Mood system integration

Wall mood bonus applies per-room (from room detection). The room's "wall quality" is the average mood bonus of all wall tiles in its boundary. This means a room with 3 brick walls and 1 wattle wall averages lower than full brick. Incentivizes consistency.

### Rain degradation

Wattle & daub loses HP slowly during rain (1 HP per game-hour of heavy rain). At 0 HP, the wall segment collapses into rubble. Plastering with lime (from DN-025 limestone chain) makes wattle weatherproof. Adobe bricks are pre-dried and don't degrade.

### Fire spread to flammable walls

Walls marked flammable (wattle, palisade, timber frame) can catch fire from adjacent burning tiles. The fire system already handles fire spread — these walls just need `is_flammable = true` and appropriate `ignition_temp` in blocks.toml.

## Implementation Order

1. Adobe wall (new BT, clay recipe, high thermal mass)
2. Low + High Palisade (two new BTs, log recipe, defense focus)
3. Dry Stone wall (new BT, rock recipe, gap properties)
4. Timber Frame wall (new BT, plank recipe, full conduit capacity)
5. Rammed Earth (new BT, no-material recipe, extreme thermal mass, slow build)
6. Mortared Stone (new BT, stone + lime recipe, requires limestone chain)
7. Fired Brick (new BT, requires kiln + clay chain)
8. Brush Fence (new BT, thornbrake recipe, simplest barrier)
9. Wall mood integration in room detection
10. Rain degradation for wattle

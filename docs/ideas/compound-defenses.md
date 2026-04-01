# Compound Defenses: Palisades, Earthworks, and Self-Sustenance

## The Progression Arc

The frontier compound evolves through distinct phases, each unlocking new defense options:

1. **Crash site (Day 1-3)**: Mud walls, basic shelter, sleeping on dirt
2. **Frontier camp (Day 4-10)**: Short palisade at weak points, first ditch, campfire perimeter
3. **Stockade (Day 10-20)**: Full palisade perimeter, gate, platforms, cleared kill zones
4. **Fort (Day 20+)**: Watchtower, earthwork berms, stone wall sections, moat
5. **Settlement (Late)**: Inner/outer walls, overlapping fire positions, self-sustaining systems

Each phase uses the output of the previous — ditch dirt becomes berms, cleared trees become palisade logs, quarried stone replaces critical palisade sections.

## Palisade Walls

Sharpened logs driven vertically into the ground, lashed together. The classic frontier fortification.

### Heights

| Type | Height | Logs/tile | Effect | Interaction with combat |
|------|--------|-----------|--------|------------------------|
| Short palisade | Low (~1m) | 2 | Cover (like low wall) | Crouch behind for cover, fire over while standing |
| Tall palisade | Full (~2.5m) | 4 | Full block | Blocks LOS, bullets, movement. Need platform to fire from |
| Pointed palisade | Full + spikes | 5 | Block + damage on contact | Enemies who touch take bleed damage |

### vs Other Wall Types

| Feature | Mud | Short Palisade | Tall Palisade | Stone |
|---------|-----|----------------|---------------|-------|
| Material | Dirt (free) | 2 logs | 4 logs | Quarried stone |
| Build time | Fast | Medium | Slow | Very slow |
| Durability | Low | Medium | Medium-high | Very high |
| **Fire resist** | **High** | **LOW** | **LOW** | **Immune** |
| Bullet block | Low wall only | Low cover | Full block | Full block |
| Aesthetics | Rough, temporary | Frontier | Military stockade | Permanent |

### Fire Vulnerability — The Key Tradeoff

Palisades are WOOD. This is the critical strategic tension:
- Enemies with torches, fire arrows, or incendiary grenades can burn down your walls
- Fire spreads along connected palisade sections (existing fire sim)
- Forces the player to: maintain fire breaks, keep water barrels nearby, station fire fighters, eventually upgrade to stone

This makes palisades a mid-game defense with a clear upgrade path, not an end-state.

### Cost Analysis

A small compound (10x10 interior):
- Perimeter: ~44 wall tiles
- Short palisade: 88 logs (22 trees)
- Tall palisade: 176 logs (44 trees)
- This is a SERIOUS investment — the player must choose what to fortify

## Platforms and Elevated Positions

### Palisade Platform
- Built against the inside face of a tall palisade
- 3 logs + 2 planks per tile
- Raises pleb to wall height — can see and shoot over
- Height advantage: bullets arc downward, harder for enemies to return fire
- Destructible: if shot/burned, pleb falls (stagger damage)
- Space for 1 pleb per platform tile

### Walkway
- Horizontal platform running along inside of palisade
- Connects platform sections, allows patrol movement
- Same cost as platform
- Enable "patrol path" — pleb walks the perimeter automatically

### Stairs
- Connects ground level to platform/walkway level
- 4 logs, 1 tile footprint
- Required to access elevated positions
- Could be kicked away / destroyed to prevent enemy use after breach

### Watchtower
- 2x2 elevated platform, taller than walls
- Extended fog-of-war vision radius (2x normal)
- Ideal for headlight/spotlight placement
- High-priority enemy target
- 12 logs + 4 planks

## Earthworks

### Ditch
- Dug in front of the palisade using existing BT_DUG_GROUND
- Slows approaching enemies (movement penalty in ditch)
- Can be flooded from water source → functional moat (fluid sim!)
- Stakes at bottom → damage on entry (punji trap)
- The dug-out dirt becomes available for berms

### Berm (Raised Earth)
- Pile excavated dirt behind the palisade
- Creates a fire step — pleb stands on berm, gains +1 height tier
- Effectively makes a short palisade + berm = medium cover with firing position
- Connects digging and building: dig moat → build berm → place palisade on top
- Historically accurate (motte-and-bailey)

### Fire Break
- Cleared 3-tile strip around the compound exterior
- No vegetation = fire can't reach walls from outside
- Creates a kill zone — open ground with no cover for attackers
- Maintenance task: vegetation regrows (if tall grass system is added)

## Obstacles and Traps

### Abatis (Felled Trees)
- Trees felled outward in front of walls
- Massive movement slow (0.2x speed through)
- Blocks charges, forces enemies to path around
- 0 cost if felled in place — just chop and leave
- Burns (can be used as a fire trap if pre-soaked in fuel?)

### Sharpened Stakes
- Angled outward, 1 log per tile
- Enemies who path through take bleed damage (0.1 per crossing)
- Stacks with ditch: stakes in a ditch = deadly
- Cheap, fast to build, effective against melee rushes

### Sandbag Positions
- Quick-deploy low cover inside the compound
- Faster to build than walls (dirt-filled sacks)
- Movable/repositionable (furniture-class item)
- Creates fallback defensive lines within the compound
- Pairs with crouch system

## Compound Layout Design

The interesting gameplay emerges from HOW you lay out defenses:

### Kill Zones
- Clear areas outside walls where defenders have unobstructed fire
- Abatis/stakes force enemies to cross kill zones
- Platform positions above cover the zone from height

### Overlapping Fields of Fire
- Platform positions at corners/junctions that cover each other
- No blind spots — enemy approaching any wall section is visible from 2+ positions
- T-junctions and L-bends in the wall create crossfire opportunities

### Fallback Positions
- Inner wall or sandbag line for when outer wall is breached
- Creates depth to the defense
- Inner positions cover the breach point

### Escape Route
- Back gate or weak wall section for evacuation
- If the compound is overwhelmed, retreat to secondary position
- Prevents total wipe

## Integration with Existing Systems

### Fluid Sim
- Moat filled from river/rain (real water, not decoration)
- Fire fighting: stored water barrels, bucket brigade
- Boiling liquids from elevated positions (future)
- Flooding a breach to slow enemies

### Sound Sim
- Warning bell/horn on watchtower (alerts colony)
- Enemies hear compound activity (noise discipline matters)
- Shout system carries over walls — rally from the watchtower

### Headlight/Torch
- Wall-mounted torches illuminate approach at night
- Torches consume fuel (resupply task)
- Defenders in shadow, attackers in light
- Enemies shoot out torches to create dark approach corridors

### Dust/Smoke
- Smoke from fires obscures vision — attackers can use fire to create smoke screens
- Dust from explosions near walls

### Morale/Stress
- Being inside compound walls reduces stress (safety need)
- Watchtower provides "I can see them coming" comfort
- Compound breach = massive stress spike for defenders
- Well-designed compound with fallback positions = stress resilience

## Self-Sustenance Requirements

For the compound to be truly self-sustaining:

| Need | Solution | Location |
|------|----------|----------|
| Food | Farm plots, berry bushes | Inside walls, irrigated |
| Water | Well, piped from source, rain collection | Central, protected |
| Fuel | Wood store (for torches, cooking, heat) | Near gate (resupply from outside) |
| Ammo | Crafting bench + materials store | Interior, protected |
| Medical | Treatment beds, herb store | Interior, quiet area |
| Rest | Beds under roofs | Interior, away from walls |
| Light | Torch fuel supply chain | Distributed along walls |
| Morale | Campfire gathering area, furniture | Central courtyard |

### The Supply Chain Loop
1. Trees cut outside → logs hauled inside → palisade/fuel/planks
2. Dirt dug for moat → berms built behind walls
3. Stone quarried → replaces critical palisade sections
4. Crops grown inside → food → fed to workers/fighters
5. Water piped in → irrigation + fire fighting + moat

## Visual Design

### Palisade Appearance (Top-Down)
- Vertical log cross-sections in a row along the wall edge
- Bark-colored circles (dark brown) with lighter wood interior
- Slightly irregular spacing (hand-placed feel)
- Pointed tops visible in oblique view (triangular tips)
- Charring/burn marks when damaged by fire

### Platform Appearance
- Plank floor visible from above (horizontal wood grain)
- Slightly elevated (shadow underneath)
- Railing on outer edge (thin line)
- Pleb standing on platform rendered at higher Y offset

## Implementation Notes

### Block Types Needed
- `BT_PALISADE_SHORT` — low wall equivalent, wooden, flammable
- `BT_PALISADE_TALL` — full wall, wooden, flammable
- `BT_PALISADE_GATE` — openable, uses door mechanics
- `BT_PLATFORM` — elevated floor, supports pleb height offset
- `BT_STAIRS` — connects ground to platform level
- `BT_STAKE` — obstacle, damages on contact
- `BT_SANDBAG` — movable low cover

### Height System Extension
Platforms introduce a second Z-level for plebs. Could use:
- `pleb.on_platform: bool` — simple flag
- Platform tiles have `height = 2` in grid_data
- Pleb on platform gets +2 Z for bullet calculations
- Shooting from height: bullet arc starts higher, more accurate downward

### Fire Interaction
- Palisade blocks have `is_flammable = true` in block_defs
- Fire sim already handles spread between adjacent flammable blocks
- Burning palisade eventually becomes `BT_SCORCHED` (destroyed)
- Water on fire: existing fluid sim extinguishes

### Patrol Paths (Future)
- Walkways could have a "patrol" zone type
- Drafted pleb assigned to patrol walks the path loop
- Stops at platforms to scan (headlight sweep)
- Auto-engages enemies spotted during patrol

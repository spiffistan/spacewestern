# DN-024: Terrain Generation and Wilderness Design

**Status:** Draft
**Depends on:** DN-016 (terrain/elevation), DN-014 (tall grass)
**Related:** DN-023 (mining), alien flora implementation

## Vision

The map should feel like Link to the Past's overworld meets RimWorld's colony site. A safe, readable central clearing for building, surrounded by increasingly dense wilderness that rewards exploration but resists conquest. The forest is alive, structured, and dangerous — not just "lots of trees."

## Three Concentric Zones

### Zone 1: The Clearing (center, ~30-40 tile radius)

Open ground with gentle elevation. Scattered rocks, dustwhisker, a few lone trees. Full visibility edge-to-edge. This is where you build.

RimWorld vibes: flat, buildable, defensible. The interesting decisions are internal — where to place the campfire, which direction to face buildings, where the water is.

At night, duskweavers probe the edges. The clearing is safe in daylight, threatened after dark.

### Zone 2: The Margins (~15-20 tiles wide)

Transitional. Trees get denser. Dustwhisker grows tall. Thornbrake patches force detours. Canopy starts closing. Berry bushes, reeds along streams, saltbrush at water margins. Dusthares live here.

Movement is slower. Vision is shorter. You send plebs here to work but keep an eye on them. At dusk you call them back.

### Zone 3: The Deep Forest (outer ring, to map edges)

Dark. Dense canopy blocks most light. Trees are tall and close together, interspersed with thornbrake. Movement 30-40% slower. Vision limited to 3-4 tiles. The minimap shows it as a dark mass.

Link to the Past vibes: not something you clear-cut, something you find paths through. Inside it: glades, creature dens, hidden water sources, mineral deposits. Rich rewards, real risk.

## What Makes the Forest Feel Alive

### It resists you

Cut a tree and a sapling grows. Clear a path and the margins creep inward over game-weeks. Maintaining a path is ongoing work. Neglect it and it closes. The forest is a living barrier, not a static one.

### It has structure

Not random tree noise. The forest has:
- **Ridges**: high ground where old trees grow tall
- **Hollows**: depressions where water collects and reeds grow
- **Game trails**: narrow natural corridors where creatures move, slightly cleared, winding
- **Glades**: clearings with their own micro-ecology, hidden until discovered
- **Dens**: thornbrake-ringed areas where duskweavers sleep during day

### Light behaves differently

Canopy blocks sunlight. Forest floor is in permanent shade — thermally cooler, visually darker. At night, no moonlight penetrates — torches essential. But torches make you visible to predators.

### Sound shifts

Wind through canopy replaces open-air gusts. Creature sounds are closer, more directional. Hollowcalls echo differently under trees. Your own footsteps are louder because everything else is quiet.

## Glades: Rooms in the Wilderness

5-10 per map. Semi-enclosed clearings within dense forest. Each is unique.

### Glade Parameters

| Axis | Options |
|------|---------|
| **Shape** | Basin (round depression), Ridge clearing (along rock), Meander (follows dry stream), Twin chambers (two connected), Crescent (wraps around feature) |
| **Floor** | Bare earth, Sandy, Mossy, Rocky, Muddy |
| **Edge** | Dustwhisker wall, Thornbrake hedge (one gap), Mixed canopy, Rock face, Reed fringe |
| **Features** (0-3) | Spring, Boulder cluster, Fallen log, Hollow tree, Mineral stain, Bones, Standing stone |
| **Ecology** | Dusthare warren, Duskweaver den, Empty (tension), Insect swarm |

Combinations create unique places:
- Basin + mossy + spring + dusthare = **tranquil oasis**
- Ridge + rocky + mineral stain + empty = **prospector's find**
- Twin chambers + bare earth + thornbrake + duskweaver = **the killing ground**

### Glade Discovery

Hidden by fog of war until a pleb enters. Minimap may show a slightly lighter patch as a hint. First discovery triggers an event: "Ada discovered a mossy clearing with a freshwater spring. She calls it Quiet Basin."

Each glade gets a procedural name from its traits. Shown on minimap once discovered. Plebs reference them in logs.

## The Path System

### Natural Game Trails

Generated during worldgen as narrow 1-2 tile corridors through dense forest. Connect glades and water sources. Creatures use them — dusthares browse along trails, duskweavers patrol them at night.

### Player-Cut Paths

Assign plebs to clear trees and brush in a line. Slow work (axe helps). Path is 2 tiles wide. Compacts the ground (uses existing compaction system), which suppresses regrowth. Well-traveled paths stay clear. Abandoned paths regrow in 20-30 game-days.

### Path Lighting

Wall torches or campfires along a path create safe corridors through the forest at night. A lit path repels duskweavers. An unlit one is an invitation.

## Geological Zones

The map's underlying geology varies by region, creating strategic differentiation:

| Zone | Stone types | Resources | Terrain |
|------|-----------|-----------|---------|
| Northwest | Granite ridges | Strong walls, hard to quarry | Sparse vegetation, defensible |
| Northeast | Chalk downs | Flint for tools, poor building | Open, good visibility |
| Central | Sandstone + clay | Easy early building (degrades in rain) | Good farmland |
| Southwest | Limestone plateau | Lime → mortar → concrete | Cave systems underneath |
| Southeast | Basalt + volcanic | Best building stone, obsidian, hearthstone | Dense forest, dangerous |

Starting position is always central. Expansion direction determines tech path.

## Water Features

### Surface Water

- **Creek/stream**: 1-2 tile ribbon, flowing. Follows elevation. Natural barrier and landmark. Reed habitat.
- **Spring**: single tile, permanent. Reliable water in any weather. Found in glades.
- **Puddles**: temporary after rain. Collect in depressions. Evaporate in sun. Reflective.
- **Bog**: soft ground near water. Slows movement. Reeds and saltbrush habitat.

### Groundwater

Existing water table system. Wells only work where water table is high enough. Drought lowers the table. Reeds indicate high water table (living groundwater map).

## Implementation Order

### Phase 1: Zone-Based Forest Density (highest impact)
Reshape worldgen so center is reliably open and edges are reliably dense. Distance-from-center gradient combined with existing forest noise, with organic (not circular) boundaries.

### Phase 2: Canopy Darkness
Darken tiles under dense tree canopy. Dense tree clusters reduce ambient light on the forest floor. Could treat dense canopy as a soft roof in the lightmap.

### Phase 3: Movement Penalty in Dense Vegetation
Dense forest tiles impose significant speed penalty. Cleared/compacted paths negate it.

### Phase 4: Game Trail Generation
Worldgen carves narrow corridors through forest connecting points of interest. A* or random walk between glade centers.

### Phase 5: Glade Generation
Shape stamper carves clearings. Floor/feature/ecology placement from template parameters. Named, discoverable.

### Phase 6: Geological Zones
Stone type varies by map region. Terrain noise determines local geology. Creates strategic map reading.

### Phase 7: Water Features
Creek generation (follows elevation gradient). Springs in glades. Puddle system after rain.

## Alien Flora Integration

Five plant species already implemented (dustwhisker, hollow reed, thornbrake, saltbrush, duskbloom). Their spawn rules should align with the zone system:

- **Dustwhisker**: dominant in Zone 1 and Zone 2. Defines the margins. Tall variant blocks vision in Zone 2.
- **Thornbrake**: concentrated at Zone 2-3 boundary. Natural barrier. Duskweaver deterrent.
- **Hollow Reed**: along water features and wet hollows. Water indicator.
- **Saltbrush**: where water meets dry ground. Uncommon, strategic.
- **Duskbloom**: scattered in glades. Never in open clearing. Discovery reward.

## Design Principles

1. **The map teaches through play.** Reeds mean water. Dark ground means forest. Colored rock means minerals. No tutorial needed.
2. **Expansion is a choice, not a chore.** You CAN stay in the clearing forever. Going into the forest is for ambition, not obligation.
3. **The forest pushes back.** It's not conquered, it's negotiated with. Paths close. Clearings shrink. Maintenance matters.
4. **Every location has character.** Not "tile 47,82" but "Quiet Basin" or "the granite ridge." Places, not coordinates.
5. **Risk scales with reward.** Deep forest has the best resources. Also the most duskweavers. Always a trade-off.

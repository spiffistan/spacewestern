# World, Seasons, and Exploration

The colony lives on a 512×512 patch of physically-simulated world. Every tile has real temperature, real airflow, real sound propagation. This is home — the 1km² of planet you've tamed. Beyond the edge: the frontier. Not another grid. Something fundamentally different.

## The Two-Scale Model

The game operates at two scales simultaneously:

**Colony scale (512×512 tile map):** Detailed, physical, real-time. Full Navier-Stokes fluid sim, wave-equation sound, per-tile thermal, raytraced lighting. This is where you build, craft, fight, and the physics shine. The core experience.

**World scale (hex/region map):** Strategic, abstract, day-by-day. Terrain regions, points of interest, trade routes, weather systems. This is where you explore, trade, establish supply lines, and manage the frontier. The expansion experience.

The player zooms between them. Most time at colony scale. World scale for strategic decisions.

---

## Colony Map: 512×512

### Why 512×512

The current grid is 256×256. Doubling to 512×512 is 4× the cells. Performance impact:

- **Fluid sim:** Jacobi pressure solve (35 iterations) scales linearly. 4× cells = 4× compute per dispatch. On a modern discrete GPU, 512×512 at 35 iterations is well under 1ms per tick. Fine.
- **Sound sim:** Wave equation at 512×512. Same linear scaling. Fine.
- **Thermal sim:** Per-tile temperature buffer. 512×512 × 4 bytes = 1MB. Trivial.
- **Block grid:** 512×512 × 4 bytes = 1MB. Trivial.
- **Dye texture:** Could stay at 512×512 (1:1 with grid) or go to 1024×1024 for visual fidelity. Either way, under 16MB.
- **Raytracer:** Already renders at viewport resolution, not grid resolution. Unaffected by grid size.
- **Total memory delta:** ~10–15MB additional. Negligible.

**The upgrade path:** Change `GRID_W` and `GRID_H` constants, update workgroup dispatch counts, resize GPU buffer allocations. No architectural change.

512×512 gives a map roughly 500m × 500m at 1m per tile. Large enough for meaningful terrain variation, multiple biomes within one map, natural exploration, and room to grow without feeling cramped. Large enough that you can't see the whole map at once at default zoom — there's always something beyond the current view.

### Terrain Variation Within the Colony Map

A 512×512 map isn't flat or uniform. The terrain should create natural zones that drive settlement decisions:

**Elevation.** A hilltop, a valley, a ridgeline, a lowland. Elevation determines: water flow (downhill), cold air pooling (valleys — thermal inversion from emergent-physics.md), wind exposure (hilltops), defensive advantage (high ground), and crop viability (frost pockets in valleys).

**Water features.** A stream crossing the map. A pond in a depression. Marshy lowland. Maybe a small waterfall where terrain drops sharply. These are the primary settlement attractors — you build near water (water-flow.md).

**Micro-biomes from aspect.** The south-facing slope is warmer (more sun in the thermal sim). The north-facing slope is cooler (char-cap grows here naturally). The ridgetop is windy (good for windmills, bad for smoke management). The valley floor is sheltered but floods and pools cold air. The rocky outcrop has mineral deposits. The flat area near the stream is the best farmland.

**Creature territories.** Different zones have different creature densities. The forest edge is duskweaver territory. The rocky outcrop has glintcrawler nests. Open grassland is ridgeback grazing ground. Deep forest is where the hollowcall originates. The colony's expansion pushes into these territories.

**Ruins and ancient infrastructure.** 2–3 discoverable sites per colony map — partially buried structures, ancient pipe conduits, sealed vaults. Found through digging, prospecting, or following anomalous readings. Feed the alien tech track (DN-019).

### Colony Placement Matters

WHERE within the 512×512 the player builds is the first and most permanent decision (philosophy.md):

| Location | Advantages | Disadvantages |
|----------|-----------|---------------|
| Hilltop | Defense, wind power, clear air, visibility | No water, exposed to weather, hard to build |
| Valley floor | Water access, sheltered from wind, rich soil | Flooding, cold air pooling, poor defense |
| Mid-slope | Compromise position, natural drainage | Neither the best view nor the best water |
| Streamside | Water, irrigation, fishing | Flood risk, mud, creature traffic at water's edge |
| Forest edge | Wood, shelter, creature resources | Fire risk, blocked sight lines, glintcrawler nests |
| Rocky outcrop | Mineral access, natural walls | Poor soil, no water, difficult construction |

The physics make each location genuinely different. The thermal sim means a valley colony is measurably colder at night. The fluid sim means a hilltop colony has different wind patterns. The water system means a streamside colony floods when it rains. These aren't abstract tags — they're physical consequences the player discovers through the simulation.

---

## The World Map

### Structure

Above the colony view: a procedurally generated region map showing the wider landscape. Your colony is one point on this map.

The world map contains:

**Terrain regions** with biome types:
- Temperate forest (your starting biome — balanced)
- Arid scrubland (sulfur deposits, condensation-based water, heat challenges)
- Tundra (frozen water, extreme cold, ice harvesting, thermogast density)
- Volcanic (geothermal heat, rare minerals, toxic gas vents)
- Coastal (salt, fishing, wind, flooding, trade access)
- Mountain (ore deposits, defensible, isolated, altitude cold)
- Swamp/marsh (peat, herbs, difficult terrain, disease risk)

**Points of interest:**
- Crash sites (salvage — finite, non-renewable)
- Ancient ruins (alien artifacts, blueprint cards, lore items)
- Mineral deposits (iron, copper, sulfur, rare metals)
- Water sources (springs, rivers, lakes — critical for arid regions)
- Creature dens (duskweaver warrens, thermogast nesting grounds)
- Other settlements (trade partners, potential allies or enemies)
- Natural features (hot springs, canyons, cave systems, ancient dams)

**Routes between regions:**
- Paths and animal trails (fast but exposed)
- River routes (fast downstream, impassable upstream without boats)
- Mountain passes (seasonal — closed in winter)
- Forest trails (slow but concealed)
- Each route has: travel time, risk level, seasonal availability

**Weather systems** that move across the world map:
- Visible as cloud formations approaching your region
- 1–2 day advance warning of major weather events
- Storm tracks, cold fronts, heat waves — all visible on the world map before they hit the colony map
- Creates preparation gameplay: "Storm coming from the northwest in 2 days. Secure the smokehouse. Stock firewood."

---

## Expeditions

### How Exploration Works

When a colonist is sent on an expedition, they leave the 512×512 colony map and enter the world map layer. Their journey is NOT tile-by-tile simulated. It's a series of events, decisions, and discoveries played out through the card system (cards.md) and time progression.

**The colonist is GONE from the colony for the duration.** Their labor is missed. Their knowledge is absent. If the colony's only Expert metallurgist goes exploring, the smelter sits idle. Expeditions cost the colony its most precious resource: people.

### Equipment Determines Capability

Expedition success depends heavily on loadout (DN-018):

| Equipment | Expedition Effect |
|-----------|------------------|
| Pack (required) | Enables multi-day trips. Capacity determines food/water/trade goods carried. |
| Bedroll (in pack) | Enables overnight camping. Without it: must return same day or find shelter. |
| Compass (vest) | Reveals more map per day of travel. Without it: exploration radius halved. |
| Spyglass (vest) | Spots threats and points of interest at longer range. Fewer ambush events. |
| Ranged weapon + ammo | Can hunt for food en route. Can defend against hostiles. |
| Rope (belt) | Enables cliff traversal, river crossing, dragging salvage. |
| Cooking skill + fire kit | Better food management on the trail. Extends range. |
| Medical supplies (vest) | Self-treatment of injuries. Without: injuries worsen during travel. |
| Coin purse | Enables trade at encountered settlements. |
| Specimen bag (vest) | Collects geological samples, seeds, artifacts en route. |

### Expedition Events

Each day of an expedition draws from an event deck weighted by terrain, weather, colonist skills, and equipment:

**Day 1:** "Kai crossed the eastern ridge. Rocky terrain, slow going. Found traces of an old road."

**Day 2:** "Kai discovered a ruined structure at the base of a cliff." [Explore / Mark and continue / Return]

**Day 3:** "Storm approaching from the north. Kai needs shelter." [Take cover in ruins / Push through / Turn back]

Events are visible in the colony's log. The player sees daily reports. Decisions are presented when the player next checks the world map (the expedition doesn't pause the colony — it runs in parallel).

The scout backstory gives better outcomes (more discoveries, fewer ambushes). The outlaw backstory handles hostile encounters better. The frontier doc can self-treat injuries. The mechanic identifies salvage value more accurately. Every backstory has expedition implications.

### What Expeditions Find

Things that feed back into the colony simulation:

**New material sources.** Your colony is in temperate forest. An expedition south finds arid land with sulfur deposits. You can't bring the mine to your colony — but you can establish a supply route. Periodic deliveries arrive by caravan or colonist hauling runs. The gunpowder domain (DN-019) unlocks because sulfur became available.

**Ruins with knowledge.** An underground installation — ancient infrastructure (the-human-layer.md). The explorer maps its location. A larger team can establish a temporary dig camp. Alien fragments feed the alien tech track. The knowledge comes from the world map; the application happens on the colony map.

**Other settlements.** Contact with another colony. Trade relationship established. Caravans arrive with goods from their biome. Radio frequency learned. Knowledge exchange: their "Irrigation Methods" for your "Duskweaver Anatomy." The social geography of the frontier opens up.

**Creature migration paths.** Ridgeback herds follow a seasonal route passing near your colony in autumn. Now you know WHEN to prepare hunting parties. Or: disturbing a thermogast nesting ground means more thermogast visits next winter. World map actions have colony map consequences.

**Map intelligence.** Terrain features, water sources, danger zones, resource locations — all permanently marked on the world map. The fog of war pushes back. Your colony's known world expands with each expedition.

### Outposts

Late-game: the colony is stable. Expeditions reveal resources too far for hauling trips. Establish **outposts** — simplified mini-colonies on world map hexes.

| Outpost Type | Purpose | Staffing | Output |
|-------------|---------|----------|--------|
| Mining camp | Extract ore from distant deposit | 2–3 colonists | Periodic ore deliveries |
| Farm outpost | Extended growing area, different biome crops | 2–3 colonists | Seasonal food shipments |
| Watchtower | Early warning of approaching threats | 1 colonist | Visible threat detection radius on world map |
| Trade post | Attracts caravans, established near routes | 1–2 colonists | More frequent, better trade |
| Dig camp | Extended excavation of ruins | 2–4 colonists | Artifact yield over weeks |

Outposts don't run the full physics sim. They're abstracted: a few stats (food supply, defense rating, population) that tick daily. But they produce resources, provide intelligence, and can be attacked.

Supply lines between colony and outposts matter. A raid that cuts the road between the mine and the base starves both of what the other provides. Roads (player-built on world map) improve travel time and reduce supply losses. The network of colony + outposts creates strategic depth.

---

## Seasons

Four seasons, each 30 game-days. A full year = 120 game-days. Seasons affect every system simultaneously — physics, creatures, crops, mood, exploration, and visual presentation.

### Spring (Days 1–30 of year)

**Thermal sim:**
- Ambient rises from 5°C to 15°C over the season
- Last frost possible until day ~15
- Ground temperature lags air by ~5 days (thermal mass)

**Water system:**
- Snowmelt fills streams and ponds. Spring is the wettest season.
- Flooding risk in low areas — the pipe model (water-flow.md) naturally fills depressions
- Water table rises from winter precipitation
- Mud: wet soil slows movement, building. Paths become gullies.

**Fluid sim:**
- Wind patterns shift — prevailing westerlies bring moist air
- Morning mist in valleys (warm air over cold water → condensation in fluid sim)
- Rain events frequent (weather system injects water + cloud cover)

**Crops and food:**
- Growing season begins. Planting window opens.
- Soil temperature must reach 10°C (thermal sim checks `block_temps` under crop tiles)
- Winter stores running low — the leanest time for food
- Sweetmoss appears on stream banks (harvestable wild food)
- Bloodgrass dormant (needs summer heat to activate)

**Creatures:**
- Duskweavers emerge from winter dens — more frequent spawns, hungry, bolder
- Ridgeback herds appear on the world map heading toward your territory
- Glintcrawlers begin appearing in warming grass
- Borers still dormant underground

**Light and mood:**
- Days lengthen from 8 to 14 hours. More working time, less danger time.
- "Spring has come: +3 mood" colony-wide. Relief after winter.

**World map:**
- Mountain passes open (snow melts). Expeditions become possible.
- Trade routes resume. First caravans of the year arrive.
- Flooding blocks some paths temporarily.

### Summer (Days 31–60)

**Thermal sim:**
- Ambient peaks at 25–35°C. Heat waves possible (40°C+ for days).
- Solar heating intense — south-facing walls hot to the touch
- Thermal mass of stone buildings creates cool interiors (lag effect)
- Heat shimmer over open ground (emergent-physics.md) affects visibility and combat

**Water system:**
- Evaporation accelerates. Ponds shrink. Drought risk.
- Irrigation becomes critical for bitterbulb and sweetmoss
- Wells draw down faster. Water table drops.
- Condensation (emergent-physics.md) becomes important in arid regions

**Fluid sim:**
- Hot air creates thermal updrafts visible in velocity overlay
- Dust (if dry) advects through the sim, reducing visibility
- Less rain, drier air — smoke from fires disperses differently

**Crops and food:**
- Peak growth. Harvest dustroot and bitterbulb.
- Bloodgrass burn season — dry vegetation ignites readily for fire-activated harvest
- Sap-vine produces most in long, warm days
- Spoilage risk highest — cold storage infrastructure stressed by ambient heat
- The abundance season: build surplus aggressively

**Creatures:**
- Borers peak — emerge nightly in dense clouds
- Thermogasts dormant (too warm — they seek heat, not flee it, but summer is already warm enough)
- Glintcrawlers most active — warm ground, tall grass, peak venom potency
- Duskweavers present but less aggressive (food is abundant in the wild)
- Ridgebacks grazing in open terrain — fattest, best hunting yield

**Fire risk:**
- Highest of the year. Dry vegetation ignites from campfires, lightning, controlled burns gone wrong.
- Forest fires from lightning (storm events). The fire system + vorticity confinement can produce fire whirls in large burns (emergent-physics.md).
- A summer fire is qualitatively different from other seasons — faster spread, larger scale, harder to control.

**Light and mood:**
- Longest days (16+ hours). Most productive season.
- Heat can cause discomfort: "Sweltering: -2 mood" when ambient > 35°C outdoors
- Balanced by abundance: "Eating well: +3 mood" if food variety is good

**World map:**
- All routes open. Best expedition season.
- Water scarcity in arid regions — southern expeditions need more water rations.
- Forest routes have fire risk.
- Best time for long-range exploration and outpost establishment.

### Autumn (Days 61–90)

**Thermal sim:**
- Ambient drops from 25°C to 10°C. First frosts appear around day 80.
- Night temperatures drop faster than day — increasing thermal differential
- Ground begins cooling — thermal mass means it lags behind air by days

**Water system:**
- Rain returns. Streams fill. Water table rises.
- Pre-frost rain saturates soil — good for water reserves, bad for construction (mud)
- Last opportunity to top up water storage before winter freezing

**Crops and food:**
- Final harvest. Everything must be in before first hard frost (`block_temps` < 0°C kills unharvested crops).
- **The most important season.** Preservation in overdrive: smokehouse, fermenter, drying racks all at capacity.
- Sap-vine: last harvest before frost. Fermentation batches started now will be ready by midwinter.
- Char-cap underground farm should be established before winter reliance.

**Creatures:**
- Ridgebacks fatten for winter — best hunting yield of the year. Migration routes peak near colony.
- Duskweavers grow bolder — pre-winter scavenging to build reserves. More nighttime raids on stockpiles.
- Hollowcall changes pitch (seasonal frequency shift — superstition material from social-knowledge.md).
- Borers decline as temperatures drop.
- Mistmaw increasingly active as nights lengthen.

**Light and mood:**
- Days shorten from 14 to 8 hours. Working hours shrink. Colony transitions to indoor work.
- "Winter approaches: -2 mood" as days grow shorter
- Offset by harvest satisfaction: "Stores are full: +3 mood" if sufficient food preserved
- Autumn leaves: tree sprites shift to golden/amber palette. Beautiful and ominous.

**World map:**
- Last chance for long expeditions before winter closes mountain passes.
- Caravan traffic peaks — everyone stocking up. Best trading season.
- Establish final supply runs from outposts.
- Any colonist still on expedition needs to return before passes close (~day 85).

### Winter (Days 91–120)

**Thermal sim:**
- Ambient drops to -5°C to 5°C. Extreme cold snaps hit -15°C.
- Indoor heating is critical. Insulation matters enormously.
- Heat loss through walls is physically calculated — stone walls lose heat slower than wood. Insulated walls (BT_14) barely lose at all.
- A cold snap + fuel shortage = death spiral. The thermal sim makes this viscerally visible.

**Water system:**
- Surface water freezes (water-flow.md ice mechanics). Rivers become walkable.
- Wells may freeze in extreme cold. Need insulated well housing.
- Ice harvest opportunity — cut blocks for ice house storage (cold storage for next summer).
- Snow accumulates as a white terrain overlay. Melts gradually in spring.

**Fluid sim:**
- Cold dense air pools in valleys — thermal inversion (emergent-physics.md). Ground-level fog.
- Smoke lingers at ground level in still, cold conditions.
- Indoor air quality matters more — doors stay shut, ventilation reduced. CO₂ builds up in poorly ventilated rooms.
- Breath visible as tiny fluid sim puffs at colonist positions in cold air.

**Crops and food:**
- Nothing grows above ground. Eat from stores.
- Underground char-cap farm is the only fresh food (if temperature-controlled at 10–18°C).
- Food consumption increases (bodies burn more calories in cold).
- Spoilage is low (everything is cold) — but frozen food needs thawing before eating.

**Creatures:**
- Thermogasts most active — drawn to the colony's heat. Peak threat. Every fire is a beacon.
- Duskweavers retreat to dens (fewer, but desperate and bolder when they appear).
- Mistmaw hunting peaks (long nights, 16+ hours of darkness).
- Borers fully dormant.
- Hollowcall continues (it never stops — its constancy is what makes its absence terrifying).

**Light and mood:**
- Days as short as 6 hours. Most of the day is darkness. Alien fauna owns the night.
- Cabin fever builds: "Cooped up: -1 mood per 10 days of winter" (cumulative). The saloon is critical.
- Midwinter blues: "Long dark: -5 mood" after 30 continuous winter days.
- But also: "Warm by the fire: +3 mood" when near a fireplace with friends. The contrast between cold outside and warm inside is the emotional core of winter.

**World map:**
- Mountain passes closed. Some routes impassable (snow, ice).
- Expeditions are short-range only. No long trips.
- Isolated from distant settlements. If you didn't trade in autumn, you're on your own.
- Outposts are vulnerable — their supply lines may be cut by weather.

---

## The Seasonal Rhythm

Each season has a dominant verb:

| Season | Dominant Verb | The Player's Focus |
|--------|-------------|-------------------|
| Spring | Plant | Recover from winter. Get crops in. Resume exploration. |
| Summer | Grow | Expand, explore, hunt, build surplus. The expansion season. |
| Autumn | Preserve | Harvest everything. Smoke, dry, ferment. Prepare or die. |
| Winter | Survive | Burn fuel. Eat stores. Stay warm. Protect knowledge. |

The cycle creates urgency that pure sandbox gameplay lacks. You're never just "building" — you're building AGAINST a deadline. The player who coasts through summer without stockpiling enters autumn behind schedule and faces a winter that might kill them.

**Year 1** is learning the cycle the hard way. The first winter is the test — did you prepare? If not, some colonists may die. That teaches the lesson for year 2.

**Year 2+** is mastering the cycle. Greenhouse for year-round sweetmoss. Underground char-cap farm for winter fresh food. Ice house stocked in late winter for summer cold storage. Multiple cold chain methods (food-and-survival.md) for redundancy. The colony becomes a machine tuned to the seasons.

**The meta-rhythm:** Within each season, there's a day/night cycle. Within each day, there's a work/rest cycle. The game has three nested rhythms — daily, seasonal, yearly — each with its own pressures and rewards. The colony's schedule (PlebSchedule) adapts to all three.

---

## The Living Map

The 512×512 isn't static terrain with weather painted on top. It's an ecosystem that responds to the colony's presence and the passage of time.

### Ecological Succession

Cleared forest doesn't stay clear:
- Year 0 (cleared): Bare soil. Fast movement. Full sun for crops.
- Year 1: Grass and wildflowers colonize. Movement unaffected. Soil holds moisture better.
- Year 2: Shrubs appear. Movement slightly slower through uncleared areas. Glintcrawlers nest.
- Year 3: Young trees. Visibility reduced. Duskweaver habitat returns.
- Year 5+: Dense regrowth. Functionally forest again.

If you don't actively maintain cleared land, the planet reclaims it. Abandoned buildings get overgrown. Unused paths get grass. The game is constantly, slowly erasing your marks unless you maintain them. This is the planet pushing back — it was here before you, and it's patient.

### Soil Depletion

Farm the same field every year without rotation or composting → soil quality drops. Terrain data tracks fertility per tile, and it changes based on usage:

- Monoculture: -10% fertility per season on the same crop
- Crop rotation (agriculture knowledge from DN-019): preserves fertility
- Composting (BT_13): +5% fertility per season to adjacent tiles
- Fallow period: +3% fertility recovery per season of no planting
- Flooding (spring): deposits nutrients on floodplain soil (+5%)

A colony that doesn't understand soil science (agriculture domain, DN-019) depletes its farmland within 3 years. One that rotates crops and composts maintains fertility indefinitely. Knowledge makes the difference between sustainable farming and soil death.

### Water Table Dynamics

Over-pump wells → local water table drops:
- Springs dry up. Ponds shrink. Downstream effects cascade.
- Recovery is slow — years of rain to restore a depleted aquifer.
- A colony that uses water carefully has permanent springs. One that pumps recklessly has dry wells by year 2.
- Per-map water table recharge rate (from MapCalibration in DN-019) varies by seed.

### Creature Displacement

As the colony expands (clearing forest, building, lighting perimeters), creatures are pushed outward:
- Duskweaver dens at the forest edge retreat as you build toward them.
- Glintcrawler nests destroyed by construction.
- But creatures don't disappear — they concentrate at the boundary.
- The frontier between colony and wilderness becomes more intense, not less.
- A large colony has a quiet, safe interior and a dangerous, creature-dense perimeter.

### The Map Remembers

The simulation accumulates seasonal history in physical marks (philosophy.md "map as memory"):

**Flood high-water marks.** After spring flooding, a visible line on terrain shows how high the water reached. Over years, a record of flood history. Build below the high-water mark at your peril.

**Fire scars.** Summer forest fires leave burned zones that take 2–3 game-years to regrow. Visible, physical. Changed ecology: no tree cover = different creatures, more sun = different crop potential, exposed soil = erosion.

**Erosion patterns.** Years of rain erode exposed hillsides. Trails become sunken paths. Cleared hillsides lose topsoil. The geology evolves.

**Compaction maps.** The path between mine and smelter, walked daily, becomes hard-packed road. Faster movement, lower mud — but channels water during rain, creating erosion gullies along most-used routes.

**Snow depth records.** Areas with deep winter snow have different spring runoff. Compacted terrain from heavy snow has different drainage patterns.

**Growth rings.** Trees planted by the colony grow over years — small in year 1, medium in year 3, large in year 5. A grove of old trees near the colony's first buildings marks the original settlement site. Chopping one reveals rings that count game-years.

After 3 game-years, the map is visually, physically, and ecologically different from how it started. A new player arriving at a year-3 colony could read its history from the terrain: where the fires were, where the floods reached, which paths are worn, which hillsides eroded, where forests were cleared, where ruins were excavated.

---

## Seasonal Visual Mood

The raytracer is uniquely positioned to sell seasonal atmosphere. Per-pixel lighting, volumetric fog, real shadows — these create mood that tile-based renderers can't match.

### Spring Look

- Longer golden-hour periods at dawn and dusk
- Green tint returning to terrain (procedural material palette shift)
- Morning mist in valleys (fluid sim: warm air over cold water → visible condensation)
- Wildflowers in procedural terrain detail (small color points in grass)
- Water everywhere — puddles, streams high, ground dark with moisture
- Palette: cool greens, soft gold light, blue-white mist

### Summer Look

- Harsh midday light with strong shadows
- Heat shimmer over open ground (thermal sim distortion in raytracer)
- Dust in dry air (fluid sim particle overlay — amber tint)
- Long shadows at dawn and dusk — dramatic, cinematic
- Vivid saturated colors. Big sky. Everything sharp and bright.
- Thunderstorm lighting: dramatic cloud shadows sweeping across the map
- Palette: warm yellows, deep greens, amber dust, white-hot sun

### Autumn Look

- Warm amber-red light. The sun sits lower — everything glows.
- Terrain shifts to golden tones (procedural material palette shift on grass and trees)
- Tree sprites swap to autumn palette: amber, rust, gold
- Mist thickens in mornings. Fog lingers longer as temperatures drop.
- Leaf fall particle effect near trees (sparse, not cartoonish)
- Everything feels ripe, heavy, ending. Beautiful and ominous.
- Palette: amber, rust, deep gold, cool blue shadows

### Winter Look

- Blue-white light. Cold color temperature on sunlight.
- Short days, long blue twilights. The sky is pale, close.
- Snow covering terrain: white overlay that smooths terrain detail, softens edges
- Breath visible as tiny puffs (fluid sim at colonist position — real physics)
- Firelight through windows: warm gold against the blue world. The contrast is stark and beautiful.
- Frost patterns on glass blocks (procedural texture overlay)
- Stars visible on clear nights (long darkness, less atmospheric scatter)
- Palette: blue-white, warm gold (interior only), deep blue shadows, white snow

### The Contrast

The most powerful seasonal visual: the boundary between warm and cold.

A winter night: the colony glows gold from fireplaces and lamps. Through the windows, warm light spills onto blue-white snow. Inside: amber, laughter (sound sim), the saloon full. Outside: blue, silence, duskweaver clicks in the distance. The raytracer makes this physically correct — light bleeds through glass, scatters in the cold air, illuminates falling snow.

No tile-based renderer creates this. The raytracer IS the art direction. The seasons are the art director.

---

## Connection to Other Systems

| System | World/Seasons Connection |
|--------|-------------------------|
| **Thermal sim** | Ambient temperature curve drives the seasonal cycle. Per-tile block_temps determine crop viability, spoilage, colonist warmth, creature behavior. |
| **Fluid sim** | Wind patterns shift by season. Mist, dust, smoke behave differently in each season's temperature and humidity. |
| **Water system** (water-flow.md) | Rain/drought cycle. Freezing/thawing. Snowmelt flooding. Evaporation. Water table recharge. |
| **Sound sim** | Winter silence vs. summer insect buzz. Seasonal ambient soundscape changes. |
| **Raytracer** | Seasonal palette, light angle, day length, atmospheric effects (mist, dust, shimmer, snow). |
| **Crops** (food-and-survival.md) | Growing seasons, planting windows, frost kill, harvest deadlines, underground winter farming. |
| **Food preservation** | Spoilage rate from thermal sim. Summer cold-chain stress. Winter natural freezing. Autumn preservation urgency. |
| **Creatures** (alien-fauna.md) | Seasonal behavior shifts: spawning, dormancy, migration, aggression. |
| **Equipment** (DN-018) | Seasonal loadout changes: winter clothing, rain gear, summer water rations on expeditions. |
| **Knowledge** (DN-019) | Seasonal knowledge windows. Agricultural knowledge requires surviving a full cycle. |
| **Trade** | Caravan frequency seasonal. Routes open/close with weather. |
| **Combat** | Seasonal raiding patterns. Winter isolation = fewer raids but more desperate ones. Summer = more activity. |
| **Mood** | Seasonal mood modifiers. Cabin fever. Spring relief. Harvest satisfaction. |

---

## Implementation Notes

### Grid Size Upgrade (512×512)

**Immediate changes:**
- `GRID_W` / `GRID_H`: 256 → 512
- All GPU buffer allocations that reference grid dimensions
- Workgroup dispatch counts in all compute shaders (fluid, thermal, sound, power, lightmap)
- `NUM_MATERIALS` clamp unchanged (block type count, not grid size)
- Dye texture: 512×512 or 1024×1024 (test performance)

**Terrain generation updates:**
- World gen must produce interesting 512×512 terrain (elevation, water features, biome variation)
- Noise parameters need retuning for larger scale (more octaves, different frequency)
- Creature territory zones defined per map seed
- Ruin/artifact placement at generation time

**Performance validation:**
- Fluid sim at 512×512: benchmark Jacobi iterations per frame
- Sound sim at 512×512: benchmark wave equation per frame
- If performance is tight: reduce Jacobi iterations (35→25), halve sound sim update rate
- Fallback: run fluid sim at 256×256 internally, upscale for rendering (existing dye resolution decoupling)

### Season System

**Core state:**
```
season: enum { Spring, Summer, Autumn, Winter }
day_of_season: u32  // 0–29
year: u32
ambient_temp_base: f32  // interpolated from seasonal curve
day_length_hours: f32   // interpolated from seasonal curve
```

**Temperature curve** (continuous, not stepped):
```
Spring day 0:   5°C,  8h daylight
Spring day 29: 15°C, 14h daylight
Summer day 0:  15°C, 14h daylight
Summer day 29: 30°C, 16h daylight
Autumn day 0:  25°C, 14h daylight
Autumn day 29: 10°C,  8h daylight
Winter day 0:   5°C,  8h daylight
Winter day 29:  0°C,  6h daylight
```

Plus random variation: ±3°C per day, ±5°C during weather events (cold snaps, heat waves).

**Integration point:** The existing `time.rs` day/night cycle extends to seasons. The ambient temperature already varies by time of day (5°C night, 25°C midday in CONTEXT.md) — seasons modulate these base values. Minimal code: interpolate seasonal curves, feed into thermal sim initialization.

### World Map Implementation

The world map is a separate data structure from the colony grid. It doesn't use the GPU sim stack — it's a CPU-side strategic layer.

```rust
/// A hex on the world map (~500m × 500m of terrain).
#[derive(Clone, Debug)]
pub struct WorldHex {
    pub biome: Biome,
    pub elevation: f32,           // rough terrain height
    pub water: bool,              // river, lake, or coast
    pub resources: Vec<Resource>, // what's here (ore, herbs, sulfur...)
    pub poi: Option<PointOfInterest>,
    pub explored: bool,           // fog of war
    pub settlement: Option<SettlementId>,
    pub creature_density: f32,    // how dangerous to traverse
    pub road: bool,               // player-built road (faster travel)
    pub seasonal_access: [bool; 4], // passable per season [spring, summer, autumn, winter]
}

#[derive(Clone, Debug)]
pub enum PointOfInterest {
    CrashSite { salvage_remaining: f32 },
    AncientRuin { excavation_progress: f32, fragments_available: u8 },
    MineralDeposit { resource: Resource, richness: f32 },
    WaterSource { flow_rate: f32 },
    CreatureDen { creature_type: String, population: u16 },
    Settlement { faction: FactionId, disposition: f32 },
    NaturalFeature { kind: String }, // hot spring, canyon, cave system
}

/// An active expedition on the world map.
#[derive(Clone, Debug)]
pub struct Expedition {
    pub colonist_ids: Vec<usize>,
    pub current_hex: (i32, i32),
    pub destination: Option<(i32, i32)>,
    pub day_of_trip: u32,
    pub food_remaining: f32,
    pub water_remaining: f32,
    pub discoveries: Vec<Discovery>,
    pub pending_event: Option<ExpeditionEvent>,
    pub status: ExpeditionStatus,
}

#[derive(Clone, Debug)]
pub enum ExpeditionStatus {
    Traveling,
    Exploring(PointOfInterest),
    Camping,
    Returning,
    Lost, // bad navigation roll, compass would have prevented this
}
```

The world map is generated at game start from the same seed as the colony map. The colony hex is always the center. Surrounding hexes are generated with biome coherence (forests cluster, mountains form ranges, rivers flow downhill from mountains to coast).

### Weather System

Weather lives on the world map and moves across it, hitting the colony when it arrives:

```rust
#[derive(Clone, Debug)]
pub struct WeatherSystem {
    pub kind: WeatherKind,
    pub position: (f32, f32),     // world map coordinates
    pub velocity: (f32, f32),     // movement per day
    pub intensity: f32,           // 0.0–1.0
    pub radius: f32,              // hexes affected
    pub duration_remaining: u32,  // days until dissipation
}

#[derive(Clone, Copy, Debug)]
pub enum WeatherKind {
    Clear,
    Cloudy,         // reduced solar, no rain
    LightRain,      // gentle, puddles in low spots
    HeavyRain,      // flooding risk, visibility drop
    Thunderstorm,   // lightning (fire risk), heavy rain, wind gusts
    DustStorm,      // arid biomes: visibility near zero, crop damage
    HeatWave,       // elevated ambient +10–15°C for days
    ColdSnap,       // depressed ambient -10–15°C for days
    Fog,            // visibility halved, mist in fluid sim
    Snow,           // winter: accumulation, freezing, beautiful
    Blizzard,       // winter extreme: near-zero visibility, deadly cold
}
```

**Weather → colony map translation.** When a weather system overlaps the colony hex:

| Weather | Thermal Sim Effect | Fluid Sim Effect | Water System Effect | Other |
|---------|-------------------|------------------|--------------------|----|
| Clear | Normal ambient | Normal wind | Evaporation normal | Full daylight |
| LightRain | -2°C ambient | Wind + moisture | Rain input: 0.0002/tick | Wet ground visual |
| HeavyRain | -5°C ambient | Strong wind + moisture | Rain input: 0.001/tick, flooding | Visibility -30% |
| Thunderstorm | -5°C, lightning heat spikes | Gusting wind (variable direction) | Heavy rain + flooding | Lightning fire starts, sound sim thunder |
| DustStorm | +3°C (insulating) | Strong directional wind + dust particles | None | Visibility -80%, crop damage |
| HeatWave | +10–15°C sustained | Still air, thermal updrafts | Evaporation 3× | Heat shimmer, fire risk |
| ColdSnap | -10–15°C sustained | Still cold air | Freezing risk | Frost damage to crops, pipe freeze risk |
| Fog | -2°C | Still air, mist injection | Condensation | Visibility -50%, sound carries differently |
| Snow | -5°C | Light wind | Snow accumulation | White terrain overlay, movement -20% |
| Blizzard | -15°C | Strong wind + snow | Heavy snow accumulation | Visibility -90%, outdoor = deadly |

### Weather Frequency by Season

| Weather | Spring | Summer | Autumn | Winter |
|---------|--------|--------|--------|--------|
| Clear | 30% | 40% | 25% | 20% |
| Cloudy | 20% | 15% | 20% | 15% |
| LightRain | 25% | 10% | 25% | 5% |
| HeavyRain | 15% | 5% | 15% | 0% |
| Thunderstorm | 5% | 15% | 10% | 0% |
| DustStorm | 0% | 10% | 0% | 0% |
| HeatWave | 0% | 15% | 0% | 0% |
| ColdSnap | 5% | 0% | 5% | 20% |
| Fog | 10% | 5% | 15% | 10% |
| Snow | 0% | 0% | 5% | 25% |
| Blizzard | 0% | 0% | 0% | 10% |

*(Percentages are per-day spawn chance for each weather type. Multiple systems can coexist on the world map, but only one affects the colony hex at a time — the most intense takes priority.)*

Weather events are spawned on the world map and drift according to prevailing wind (seasonal). The player sees them approaching 1–2 days before they hit. A watchtower outpost extends the early warning radius.

### Map Generation

**World map** is seeded from the game seed. Biome placement uses large-scale noise:
- Mountain ranges form spines across the map
- Rivers flow downhill from mountains, pooling in lakes
- Biomes cluster by latitude/moisture (tundra at high elevation, arid in rain shadows, forest in moderate zones)
- 3–5 other settlements placed at viable locations (near water, defensible terrain)
- 10–20 points of interest scattered across regions (crash sites near your starting hex, ruins in mountains, mineral deposits in foothills)

**Colony map** (512×512) is generated with finer-grained noise within the biome of the colony hex:
- Elevation: 3–4 octaves of Perlin noise, scaled to produce a hill, a valley, and a ridgeline
- Water features: rivers traced downhill from high points, ponds at local minima, marsh where water table intersects surface
- Vegetation: tree density from moisture + elevation noise, clustered forests with clearings
- Soil: fertility from moisture + terrain type, clay deposits at specific locations
- Rock: stone outcrops at high elevation, scattered debris
- Ruins: 2–3 placed at interesting terrain features (cliff base, hilltop, riverbank)
- Creature territories: assigned by terrain type (forest edge = duskweaver, rocky = glintcrawler, open = ridgeback)
- The crash site: player's starting location, placed at a reasonable spot (not underwater, not on a cliff)

---

## Open Questions

1. **World map size?** How many hexes across? A 20×20 hex grid (400 hexes) is large enough for varied exploration across multiple biomes. A 50×50 (2500 hexes) is an open world. Recommendation: 30×30 (900 hexes) — enough for 3–5 biome regions, 10+ points of interest, and multiple seasons of exploration content. Larger maps can always be generated but add no value until the expedition system is rich.

2. **Expedition control fidelity?** Current design: one decision per day (card system). Alternative: more granular control with hourly events and real-time travel. Recommendation: daily decisions. The colony is the real-time game; expeditions are the strategic game. Mixing fidelities creates whiplash. Daily events with meaningful choices are enough.

3. **How do outposts degrade?** An outpost needs food, fuel, and maintenance. If the supply line is cut, it degrades: food runs out → colonists starve. Structures decay without repair. Eventually abandoned. How fast? Recommendation: 10–15 days of autonomy with full stores. A cut supply line is an urgent but not immediate crisis.

4. **Can you have multiple colony-scale maps?** If you establish a major outpost, does it get its own 512×512 simulation? This is technically feasible (swap GPU buffers when viewing different bases) but dramatically increases complexity. Recommendation: one colony-scale map per game. Outposts are always abstracted. If the player wants a second full base, that's a different save/playthrough.

5. **Season length tuning?** 30 game-days per season = 120 per year. At what real-time speed does this feel right? At 1× speed with ~1 minute per game-day, a year is 2 hours. At 2× speed, 1 hour. Recommendation: tune so that a year feels substantial (2–3 hours of play at mixed speed) but not grinding. The player should feel each season's pressure, not coast through.

6. **Snow as physics or visual?** Snow accumulation could be a terrain layer (like water height) with real physics (insulation, weight on roofs, melt → water) or a visual overlay with simple gameplay effects (movement penalty, roof stress). Recommendation: start with visual overlay + movement penalty. Add physics (melt feeding water system, insulation value) in a later phase. Snow-as-physics is correct but not urgent.

7. **Ecological succession rate?** How fast does forest regrow? 5 game-years to full regrowth means the player sees the cycle in a long game. 2 years is faster than realistic but creates more gameplay pressure to maintain cleared land. Recommendation: 3 game-years from cleared → functional forest. Fast enough to matter, slow enough to not feel like whack-a-mole.

8. **World map biome for the starting colony?** Always temperate (balanced, forgiving)? Or player-selectable? Or random with the option to reroll? Recommendation: temperate as the default "standard" start. Arid, tundra, etc. as challenge modes for experienced players. Like RimWorld's biome selection — the default is moderate, extremes are opt-in.

---

## Summary

The world operates at two scales: a 512×512 colony map where every tile has real physics, and a hex-based world map where exploration, trade, and strategic decisions play out abstractly. Seasons are the bridge — they affect both scales simultaneously, creating a unified rhythm of plant, grow, preserve, survive that drives all gameplay.

**The 512×512 colony map** is where the simulation shines: Navier-Stokes fluid dynamics, wave-equation sound, per-tile thermal, raytraced lighting. It has internal terrain variation (hills, valleys, water, forests, ruins, creature territories) that makes WHERE within the map you settle a meaningful, permanent decision. The map is alive — ecological succession reclaims cleared land, soil depletes without rotation, water tables drop from over-pumping, erosion reshapes terrain over years. The simulation accumulates seasonal history in physical marks: flood lines, fire scars, worn paths, growth rings. After three game-years, the map tells the colony's story.

**The world map** is where the frontier lives: biome regions, trade routes, ruins, other settlements, creature migration paths, weather systems approaching. Exploration happens through expeditions — colonists leave the colony map and enter a card-driven daily event system weighted by their skills and equipment. Outposts extend the colony's reach without requiring additional physics simulation. Supply lines, trade relationships, and intelligence networks connect the colony to the wider world.

**Seasons** are the pulse. Spring is for planting and recovery. Summer is for expansion and abundance. Autumn is for preservation and preparation. Winter is for survival and endurance. Every system in the game — thermal, fluid, water, crops, creatures, food, equipment, knowledge, exploration, mood — varies by season. The seasonal cycle creates urgency in a sandbox that would otherwise lack direction. Year 1 teaches the cycle. Year 2+ masters it.

**The visual identity** of each season is sold by the raytracer: spring mist in valleys, summer heat shimmer, autumn amber light through golden leaves, winter firelight on blue snow. No tile-based renderer creates this. The per-pixel physics simulation IS the art direction.

The two scales and four seasons together create a game with nested rhythms — daily (day/night), seasonal (plant/grow/preserve/survive), and yearly (learn/adapt/master). Each rhythm operates at a different timescale and creates different pressures. The colony that thrives is the one that harmonizes all three.

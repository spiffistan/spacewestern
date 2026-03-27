# Gameplay System Ideas

Exploratory ideas for systems that would deepen the colony simulation. Not commitments — creative brainstorming.

## Day/Night as Gameplay

Night isn't just darker — it fundamentally changes what happens.

- Nocturnal predators emerge after dark (wolves, snakes, alien fauna)
- Redskulls prefer dawn raids (catching the colony waking up)
- Crops grow during day, frost risk at night
- The saloon only provides stress relief after dark (colonists need to unwind in the evening, not mid-shift)
- Night shift vs day shift creates scheduling tension — some work must happen at night (smelting, guard duty) but nobody wants to
- Torch/lamplight radius defines your safe zone. Beyond the light = fog of war + danger
- Some crafting (moonshine?) only happens at night

The day/night cycle becomes a strategic pressure, not just a lighting change.

## Weather as Events

Weather is something you prepare for and react to, not background decoration.

- **Thunderstorm:** lightning strikes set fires, wind pushes smoke sideways, flooding fills basements. A real crisis.
- **Dust storm:** vision radius drops to 3 tiles, outdoor work halts, crops take damage. Colonists shelter indoors or take stress damage.
- **Heat wave:** crops wilt without irrigation, colonists overheat outdoors, fires spread faster. Need shade structures and water.
- **Cold snap:** pipes freeze (burst?), outdoor colonists take cold damage, food consumption increases. Need stockpiled fuel.
- **Fog:** vision radius halved, enemies can approach unseen. Atmospheric but tactically dangerous.
- **Wind shift:** changes smoke/gas spread direction. A fire that was blowing away from your base suddenly blows toward it.

Each weather event demands a different response. The colony that only prepared for one season gets caught.

## Sound as Information

The sound propagation system already exists on GPU. Make it carry gameplay information.

- **Distant gunshots** tell you a raid is happening before you see it
- **Mining picks** echo through tunnels — you can hear how deep the mine goes
- **Door slam** means someone entered a building
- **Alarm bell** (placeable item) alerts colonists to danger
- **Animal growl** warns of predators before visual contact
- Sound travels through walls (muffled), around corners, up stairways
- Colonists react to sounds: hearing combat raises stress even if they can't see it
- Sound-sensitive colonists (light sleeper trait) wake up from nearby noise
- Stealth: soft-soled boots reduce footstep sound (enemies can hear you too)

The existing sound sim makes this physical, not abstract. Sound has a source, travels through the world, and decays with distance and obstacles.

## Ruins and Archaeology

The world has history. You're not the first ones here.

- Collapsed structures half-buried in terrain, visible on the surface as stone outlines
- Dig sites: assign colonists to excavate — slow work that reveals artifacts over time
- Ancient foundations give a head start on building (pre-laid stone floor, partial walls)
- Crashed ship as colony origin (your starting location, provides some scrap materials)
- Underground ruins at depth -4/-5 (connects to multi-level system)
- Blueprint cards found in ruins unlock technology you can't research otherwise
- Some ruins are dangerous: trapped, inhabited by creatures, or structurally unsound
- Lore fragments: text snippets that piece together what happened before you arrived

The world isn't blank. It has a past you uncover through exploration and excavation.

## Trade and the Outside World

The colony isn't alone. There's a wider world that interacts with it.

- Periodic trade caravans arrive with goods you can't produce locally
- Road/path system: caravans follow roads. Building near a road = more frequent trade
- Building remote = safety from raids but economic isolation
- Trade introduces items from other biomes (tropical fruit, arctic furs, desert spices)
- Prices fluctuate based on supply/demand and events (drought = expensive food)
- Reputation affects who comes: generous colonies attract settlers, wealthy ones attract thieves
- Trading post (building): attracts caravans more frequently
- Export economy: what does your colony produce that others want? Planks? Ore? Moonshine?

The economy becomes externally connected. Self-sufficiency is an option, not a requirement.

## Seasons and Long-Term Cycles

Each season demands different priorities. The calendar drives the survival loop.

**Spring:**
- Snow melts, ground softens, planting begins
- Mud slows movement on unpaved paths (terrain compaction matters)
- Flooding from snowmelt fills low areas and basements
- Migrating animals arrive (hunting opportunity)
- Building season starts (ground was frozen)

**Summer:**
- Peak crop growth, longest days, most productive season
- Heat and drought risk — need irrigation or water reserves
- Fire risk highest (dry vegetation + heat)
- Most trade caravans visit in summer (roads are passable)

**Autumn:**
- Harvest time — all crops must be gathered before frost
- Food preservation becomes critical (root cellar, smoking, drying)
- Animals fatten before winter (best hunting yields)
- Last chance for construction before ground freezes
- Leaves change color (visual: tree sprites swap palette)

**Winter:**
- Freezing outdoors, shelter and fire essential (existing night mechanics, extended)
- Food from storage only — nothing grows
- Short days, long nights (expanded danger window)
- Cabin fever: colonists cooped up together gain stress faster
- Snow covers terrain (visual: white overlay, movement penalty on unpaved ground)
- Pipes freeze if exposed to cold without insulation

Storing enough food and fuel for winter is THE fundamental survival loop. A colony that coasted through summer without preparing faces a death spiral in winter.

## Fire as a Tool

Fire isn't just a disaster to fight — it's a resource to use.

- **Controlled burns:** clear brush and long grass for farming or defensive sight lines
- **Smoke signals:** attract trade caravans (or unwanted attention)
- **Charcoal production:** burn wood in a kiln with limited air = charcoal, needed for smelting
- **Firing clay:** sustained fire in the kiln produces pottery, bricks (already exists)
- **Perimeter defense:** torch lines and fire pits deter nocturnal predators
- **Cooking:** fire + food = better meals = more nutrition + mood boost
- **Cauterization:** fire + wound = stop bleeding (desperate medical option)
- **Signal fires:** visible from far away, used for communication between outposts

The fire system becomes a tool you master, not just a threat you manage.

## Reputation and Consequences

Your decisions echo through the event/card system.

- **Generous:** help visitors, share food → word spreads → more migrants seek to join, traders offer better prices
- **Brutal:** execute prisoners, drive away beggars → feared → fewer raids but no one wants to join you, traders charge premium
- **Wealthy:** visible prosperity (nice buildings, surplus goods) → attracts thieves AND traders
- **Militaristic:** strong defenses, combat veterans → Redskulls avoid you, but rival factions see you as a threat
- **Isolationist:** remote location, no trade → self-sufficient but no outside help in crisis

Reputation is a number that shifts based on actions. Event cards reference it: "A stranger arrives. Your reputation precedes you — they [trust you / fear you / don't know you]."

## Music and Morale

Instruments are craftable items. Music is physical (travels through sound sim).

- A colonist with a guitar plays at the saloon in the evening
- Nearby colonists get a mood buff (heard through the sound system, not a global stat)
- Music travels through walls (muffled), up stairways, across the colony
- Work songs: a singing colonist near a work site slightly boosts nearby worker speed
- Funeral dirge after a death (colony-wide mood debuff, but helps process grief faster)
- Different instruments: guitar, harmonica, fiddle, drum. Each has a different sound character.
- A colonist with the "musical" trait plays better (stronger buff, more pleasing sound)

The sound sim makes this spatial and physical. A colonist in a distant mine can't hear the saloon guitar. One near a ventilation shaft might catch faint notes.

## Livestock and Animals

Animals are a long-term investment that connects to multiple systems.

- **Chickens:** eggs (food source), low maintenance, need coop (small building)
- **Cattle:** leather (crafting material), meat (food), milk. Need pasture (fenced area with grass)
- **Horses:** faster overland travel, carry goods. Need stable and feed.
- **Dogs:** early warning system (bark at approaching enemies — uses sound sim). Companionship (mood buff for owner). Can be trained to herd livestock.
- **Cats:** pest control (mice eat stored food — cats prevent this). Low maintenance.

Animals need: food, shelter (thin wall barn), fencing (half-height walls from DN-004), water. They're a commitment — neglected animals die, wasting the investment.

Animal sprites: own atlas, same layered system (body + head), fewer variants. 4-8 animal types is plenty.

Breeding: animals reproduce over time. A pair of chickens becomes a flock. Overpopulation needs management (butcher or sell).

## Letters and Narrative Fragments

Pure flavor that makes the world feel inhabited.

- Found notes in ruins: "Day 47. The water's rising. We should never have dug so deep." Connects to multi-level depth lore.
- Trade caravans carry letters from other settlements: news, rumors, requests for help
- Colonists write journal entries (generated text based on recent events): "Another raid. Jeb took a bullet. I'm getting too old for this."
- Message in a bottle arrives during flood/rain: "If you find this, head west. There's fresh water."
- Wanted posters for outlaws: "WANTED: Dutch 'Two-Shot' McCrae. 500 silver reward."

These are delivered through the card system as narrative events. No mechanics — just world-building. The colony's story emerges from the intersection of mechanical events and narrative fragments.

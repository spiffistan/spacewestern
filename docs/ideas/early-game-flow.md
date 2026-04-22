# Early Game Flow (Work in Progress)

How the first 30 game-days could unfold. Not a design spec — an exploration of how the systems interlock to create a natural progression arc. This will evolve as systems are implemented and playtested.

## Day 1: The Crash (first 5 minutes of real time)

Three survivors. Scattered wreckage. Crates strewn across the clearing. Midday — maybe 8 minutes of daylight.

**Immediate loop**: click crates, plebs walk to them and open them. Each is a small discovery event. "Ration packs (3)." "Broken stone pick." "Torn fabric." One crate has something interesting — a sealed seed packet, or a cracked solar cell.

**Reading the terrain**: the clearing is open — dustwhisker grass, loose rocks, scattered thornbrake. Dense forest visible at the edges. A glint of water to the south, reeds at the shoreline.

**First decision**: where to put the campfire. Near the wreck (metal walls block wind, partial shelter) or near water (future access). Most players pick the wreck — it's shelter from day one. A pleb gathers sticks from nearby dustwhisker, delivers them, lights the fire. One problem solved.

**First tools**: a pleb picks up a rock. They now have a hammerstone. Another uses the hammerstone on a rock — knapped stone blade. First cutting edge. No crafting menu, no infrastructure. Just: rock + rock = blade.

## Day 1 Evening: The First Night

Dusk warning. Everyone into the wreck. Metal walls aren't comfortable (-2 mood) but they stop duskweavers. A howl from the forest. The campfire flickers.

Everyone's hungry. Ration packs from the crate buy one meal each — that's tonight. Tomorrow food becomes urgent.

Someone with nothing to do knaps another blade. Tools accumulate through idle time. By morning the colony has 2-3 stone blades and a hammerstone. Enough to start.

## Day 2-3: Establishing

Three parallel needs emerge. The player doesn't choose an order — they address whichever feels most urgent.

### Food

Berry bushes at the forest margin. A pleb with a stone blade can harvest dustwhisker for fiber (not food, but useful). Dusthares browse in the clearing at dawn — hunting with a stone blade is slow and close-range but possible. Cooking raw meat at the campfire.

The food loop at this stage: forage berries + hunt dusthares + cook at campfire. Unreliable but functional. The player starts thinking about farming.

### Shelter

The wreck is cramped. Someone gathers sticks and builds wattle walls extending from the hull. The wreck's metal wall on the north side, wattle on east and west, open to the south (facing the clearing, away from forest). Rough floor (1 stick per tile). A lean-to over the gap.

Room detection triggers: "Shelter (8 tiles)." Sleeping indoors for the first time. Mood improves. The difference between "sleeping on the ground" and "sleeping in a shelter" is immediately felt.

### Defense

Thornbrake at the rocky edge is a natural duskweaver barrier — leave it. On exposed sides, drag brush fence (3 sticks each) in a loose arc. Not a wall, just enough to slow anything approaching. Good enough for now.

## Day 3-5: The Tool Chain

Stone blades keep breaking (durability 25). The player notices plebs pausing to sharpen (auto-behavior). More rocks needed. More sticks needed. The material pressure is constant but low-level.

The progression chain reveals itself:

```
Pick up rocks → hammerstone (instant)
    ↓
Hammerstone + rock → stone blade (10s)
    ↓
Gather sticks + fiber from dustwhisker
    ↓
Stone blade + stick + fiber → stone axe (20s)
    ↓
Stone axe → chop tree → logs
    ↓
Logs → saw horse → planks
    ↓
Planks → workbench
```

Each step unlocks the next. The workbench (day 3-5) is the first infrastructure milestone. Once it exists, broken tools auto-repair and the minimum stock system engages. Before it, everything is hand-crafted.

## Day 5-7: Fire Economy

The campfire needs fuel. Someone has to feed it sticks. When everyone's busy, the fire dies. Cold night, -mood, duskweavers get bolder.

The player builds a charcoal mound: 3 logs, cover, light. Wait a game-day. Get 4 charcoal. Charcoal burns 3x longer than sticks. One charcoal unit covers most of a night.

But charcoal needs logs → trees → axe. The fuel economy connects to the tool economy which connects to the material economy. Everything loops.

The player starts thinking about fire as infrastructure, not a one-time placement. Who tends it? Where do we store fuel? Do we need a second fire at the perimeter?

## Day 7-10: First Proper Buildings

Wattle walls degrade in rain. After the first storm, damage is visible. Options:

1. **Plaster the wattle** (lime from limestone — but limestone hasn't been found yet)
2. **Build dry stone** (rocks, no mortar, doesn't degrade — but drafty)
3. **Rebuild with wood** (planks, good for conduits later — but fire risk)

The first "real" room: four walls, a door, a floor, a bed, a roof. Room detection: "Bedroom (12 tiles)." Sleep mood jumps. The pleb who sleeps here is noticeably happier.

The wreck transitions from shelter to workshop. Workbench inside the hull. Tool maintenance becomes automatic. The colony now has specialization: workshop (wreck) and living quarters (new room).

## Day 10-15: Exploration Pressure

Resources near camp thin out. Dustwhisker around the clearing has been harvested (regrowing slowly — 8 game-days). Berry bushes are depleted (5-7 days to refill). Rocks nearest the wreck are mined.

The player looks at the forest margin. There's more out there. A pleb wanders toward the trees and notices pale ground with dark nodules — flint-bearing terrain. If they have geology knowledge: "Flint nodules — better tools." If not: "Discolored ground."

Finding flint is the first exploration reward. It's at the forest edge (Zone 2) — not dangerous but further than anyone's been. Bringing back flint: hammerstone + flint → flint blade (60 durability vs. 25 for stone). The colony immediately runs smoother.

Meanwhile: the saltbrush near the water edge yields salt. Salt preserves meat indefinitely. The food economy stabilizes — hunt, cook, salt. No more spoilage anxiety.

## Day 15-20: The Perimeter Question

The brush fence is failing. Duskweavers pushed through last night. Options:

1. **Low palisade** (2 logs each) — quick, blocks pathing, plebs can shoot over
2. **High palisade** (4 logs + rope each) — full barrier, expensive
3. **Thornbrake hedge** — leave natural thornbrake, plant more (future?)

A full palisade perimeter is EXPENSIVE. 20-tile perimeter × 4 logs = 80 logs = ~16 trees. This competes with charcoal production and plank-making. The player compromises: palisade on the forest-facing sides, brush fence elsewhere.

Torches along the palisade: each costs 1 stick + ongoing fuel. But they repel duskweavers (light safe zone). The perimeter at night: flickering torches on wooden walls, darkness beyond. The frontier feel crystallizes.

## Day 20-30: Deeper Exploration

A pleb ventures into the deep forest (Zone 3). Dense trees, low visibility. They find a glade — a natural clearing with a spring. Fresh water without digging a well. But duskweaver tracks in the mud.

Decision: establish a path to the spring (clear trees, place torches for night safety) or dig a well at camp (safer, more labor).

Someone mining rocks discovers iron-stained ground. Mining exposes iron veins in the cut face. The geology discovery triggers: "Ada found iron ore." But iron → smelting → kiln → clay → another exploration trip.

The pattern: each need sends you further from camp. Each trip reveals something new. Each discovery unlocks a capability. The game expands concentrically — clearing → margins → forest → deep forest.

## Rooms Built (roughly in order)

1. **The Wreck** — initial shelter, transitions to workshop
2. **Wattle extension** — extra covered space off the wreck hull
3. **Bedroom** — first proper room (4 walls, door, bed, roof)
4. **Kitchen area** — campfire with roof for rain-proof cooking
5. **Storage corner** — crate(s) near the workbench
6. **Perimeter** — brush fence → low palisade → high palisade (incremental)
7. **Well** — reliable water, placed where reeds indicate high water table
8. **Charcoal mound** — outside perimeter (smoky, fire hazard)
9. **Kiln** — clay brick production, outside (hot, smoky)
10. **Forest outpost** — small shelter at a discovered glade, torch-lit path back to camp

## Mood Arc

```
Day 1:   Neutral (crash adrenaline, survival focus)
Day 3:   Dipping (hunger stress, sleeping rough, tool frustration)
Day 5:   Recovering (first real shelter, cooked food, tools working)
Day 10:  Stable (bedroom, fire maintained, perimeter)
Day 15:  Rising (flint tools, salt-preserved food, room bonuses)
Day 20:  Comfortable or stressed (depends on duskweaver incidents)
Day 30:  Established (multiple rooms, automated tool supply, exploration underway)
```

The worst mood is around day 3-5 when the crash rations run out and the shelter isn't finished. This is the "survive or die" inflection point. After that, each improvement is felt directly through mood recovery.

## Onboarding: No Tutorial, No Hand-Holding

The game teaches entirely through its characters, its world, and consequences. No tutorial popups. No quest markers. No forced sequences. The player learns by watching what their plebs do, noticing what works, and feeling what doesn't.

### The Crash Landing Card

When the game starts (after manifest chargen, transitioning to the world), a single text card appears over the scene — styled like a worn paper dispatch, not a UI modal:

```
The Perdition broke apart at 4,000 feet.

Three survivors. A field of wreckage.
The forest is close. Dusk is closer.

[click anywhere to continue]
```

Short. No mechanics explanation. No "click here to build." Just: you're here, it's bad, go. The card fades and the game is live. The plebs are standing in the debris field. The clock is ticking.

Ship-type variants change the flavor:

- **Mining Vessel**: "The drill rig tore free on impact. Fuel cells scattered across the hillside."
- **Colony Transport**: "Seeds and medical supplies — somewhere in that wreckage."
- **Military Scout**: "Standard crash protocol: secure weapons, establish perimeter, assess casualties."
- **Science Expedition**: "The instruments. If even one spectrometer survived..."

The card sets tone and hints at priorities without instructing.

### Layer 1: Pleb Thoughts Teach Mechanics (First 60 Seconds)

Plebs think out loud. Their thought bubbles are the tutorial. In the first minute:

**Idle pleb near wreckage:**
> "Those crates might have supplies..."

The player clicks a crate. A pleb walks to it, opens it. Discovery event: "Ration packs (3), torn fabric." The player has learned: click things → plebs interact → you get stuff.

**Hungry pleb (after rations run out):**
> "I'm starving. Are those berries?"

The pleb looks toward the forest margin. The player follows their gaze. Berry bushes are visible. Right-click a bush → "Harvest." The player has learned: right-click for context actions.

**Cold pleb at dusk:**
> "We need a fire before dark."

The player opens the build menu (bottom bar, visible from the start). "Campfire" is in the Survival tab. Place it. A pleb gathers sticks and lights it. The player has learned: build menu → place things → plebs do the work.

**Pleb picking up a rock:**
> "Heavy enough to work with."

The pleb now has a hammerstone. They walk to another rock and start knapping. Thought bubble: "If I shape this right..." Stone blade produced. The player has learned: plebs have initiative. They craft what they can.

### Layer 2: Environmental Teaching

The world itself communicates what matters.

**Affordance through visibility:**
- Berry bushes have bright fruit — they look edible. Thornbrake looks hostile. The visual language is immediate.
- Water glints. Reeds sway at the shoreline. "There's water that way" is communicated without a marker.
- The forest edge is dark and dense. The clearing is open and safe. Danger is visible as shadow.
- Rocks on the ground look pickable-up. Sticks look gatherable. The world invites interaction.

**The hover card:**
Hovering over any tile shows a styled info card: terrain type, vegetation, what's there. "Dustwhisker — harvestable (fiber)." "Berry Bush — harvestable (berries)." "Loose Rock — gatherable." This is passive information — the player discovers it by curiosity, not instruction.

**The right-click context menu:**
Right-clicking anything in the world shows what you can DO with it. This is the core discovery mechanism. The player learns the game's verbs by right-clicking things:
- Rock → "Pick Up"
- Tree → "Chop" (requires axe)
- Bush → "Harvest"
- Crate → "Open"
- Pleb → "Prioritize", "Inspect"
- Ground → "Build Here", "Dig" (requires shovel)

Grayed-out options with a reason: "Chop (needs axe)" teaches the tool requirement without a tutorial. The player sees what's POSSIBLE and what's MISSING.

**Night as teacher:**
The first night teaches everything about light and danger. Dusk falls. The light retreats. Sounds from the forest. If the campfire is lit, the safe zone is visible — a warm circle in the dark. If it isn't, a duskweaver approaches. The player learns: fire = safety. Darkness = danger. No tooltip needed.

### Layer 3: Consequences Teach

The game doesn't explain WHY things matter. It shows you what happens when they go wrong.

**No shelter:**
Plebs sleeping outside: mood drops. Thought: "Slept on the ground again..." The player notices the mood bar dipping. Next game, they build shelter earlier.

**No food:**
Hunger builds. Plebs slow down. Thought: "Can't think straight... so hungry." Performance visibly degrades. The player prioritizes food next time.

**Tool breaks:**
A pleb's axe breaks mid-chop. Thought: "Damn. Need a new one." They switch to bare hands (visibly slower). The player learns: tools break → need replacements → need materials → need a supply chain.

**Rain without shelter:**
Plebs get wet. Movement slows. Mood drops. Thought: "Soaked through..." If they stand by a fire, they dry off. The player connects: rain → wet → bad. Fire → dry → good. Roof → prevents wet.

**Duskweaver breach:**
A duskweaver pushes through the brush fence. A pleb panics. Combat happens (messy, scary). Even if they win, the mood hit is severe. The player learns: the perimeter matters. Upgrade the walls.

Each consequence is a story the player remembers. "I lost my best crafter because I didn't build a wall" teaches wall-building better than any tutorial.

### Layer 4: Backstory-Specific Hints

Each pleb's backstory subtly guides the player toward their strengths.

**Engineer backstory:**
> "If I had a workbench, I could keep these tools in shape."

Hints at the workbench milestone. The player builds one. Auto-repair engages. The engineer's value is demonstrated through their own suggestion.

**Botanist backstory:**
> "That soil looks rich enough for planting."

Hints at farming. The player tries it. The botanist's skill makes crops grow faster. Their expertise is proven, not stated.

**Soldier backstory:**
> "We need a perimeter. Something between us and whatever's out there."

Hints at defense. The player builds a fence. The soldier's combat skill becomes relevant when something attacks. Their role emerges from the situation.

Backstory hints are RARE — one per pleb in the first few days. They're personality, not instructions. The player shouldn't feel guided. They should feel like their crew is thinking.

### Build Menu as Discovery

The build menu is organized by intent, not by material or tech tree:

- **Survival** — campfire, lean-to, drying rack (what you need RIGHT NOW)
- **Shelter** — walls, doors, floors, roofs (protection)
- **Light** — torch, lantern, campfire (pushing back the dark)
- **Food** — farm plot, cookfire, smokehouse (feeding people)
- **Craft** — workbench, saw horse, kiln (making things)
- **Power** — solar cell, wire, battery (future)
- **Pipes** — gas pipe, liquid pipe, vent (future)
- **Zones** — stockpile, farm, dump (organization)

Items the player can't build yet are visible but grayed out with the missing requirement: "Kiln — needs 10 clay, 4 rock." This teaches what's POSSIBLE and what resources to seek. The player sees the kiln, wants it, and goes looking for clay. The build menu is the tech tree — no separate screen needed.

Items unlock by having the required knowledge + materials. No research screen. A pleb who's never seen clay can't build a kiln. A pleb who found clay and has construction knowledge can. The unlock is organic: explore → find → know → build.

### What to Explicitly Avoid

- **No "press W to move" prompts.** The player clicks. Plebs move. It's obvious.
- **No arrow pointing at the campfire slot.** The build menu is right there. The pleb said they need fire.
- **No "Quest: Build a shelter" tracker.** The mood system IS the tracker. Bad mood = fix the problem.
- **No minimap markers for resources.** The hover card and right-click menu reveal the world. Walk around and look.
- **No "You've unlocked: Stone Axe!" popups.** A pleb crafts it. You see it happen. That's the unlock.
- **No difficulty selection.** Crash severity is random. Every start is a challenge. The game doesn't patronize.
- **No pause-and-explain.** The game never stops to teach. Time keeps moving. Learn while surviving.

### The First Five Minutes (What Actually Happens)

1. **0:00** — Crash card appears. Player reads, clicks. World is live.
2. **0:15** — Plebs are standing around the wreck. One thinks: "Those crates..." Player clicks a crate.
3. **0:30** — Crate opened. Items discovered. Player clicks another crate. Learning accelerates.
4. **0:45** — A pleb picks up a rock. Starts knapping. Player watches. "Oh, they do things on their own."
5. **1:00** — Player hovers over terrain. Info card appears. "Dustwhisker — harvestable." Player right-clicks. "Harvest." A pleb goes.
6. **1:30** — Player opens build menu. Sees campfire. Places it near the wreck. A pleb gathers sticks, builds it.
7. **2:00** — Dusk approaches. The sky changes. A pleb thinks: "We should stay near the fire tonight."
8. **3:00** — Night falls. The campfire glows. Sounds from the forest. The player understands the stakes without being told.
9. **4:00** — Dawn. Everyone survived. The player exhales. Looks at the build menu again. What else can I make?
10. **5:00** — The player is playing. No tutorial was needed.

The game taught itself through its world, its characters, and the pressure of time. The player learned because they had to — not because they were told to.

## Open Questions

- When does the first crop get planted? Needs soil richness check + cleared farmland + seeds (from crash or wild plants?)
- How does the first metal tool arrive? Kiln → smelt → forge is a LONG chain. Maybe 40+ days?
- When do plebs start having relationships/opinions about each other? Day 10? Day 20?
- Should there be a "first contact" event (traders, other survivors, radio signal)?
- How does the discovery system pace information? Too fast = no mystery. Too slow = frustration.
- What triggers the player to explore underground? (DN-023 mining, sinkhole discovery)
- How much pleb initiative is too much? If they do everything, the player feels unnecessary. If they do nothing, the player feels like a micromanager. The balance: plebs handle survival instincts (eat, sleep, flee). The player handles strategy (where to build, what to prioritize, when to explore).

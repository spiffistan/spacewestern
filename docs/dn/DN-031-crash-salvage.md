# DN-031: Crash Salvage and Starting Conditions

**Status:** Draft
**Depends on:** DN-025 (primitive tools), DN-026 (world discovery)
**Related:** Manifest chargen, ManifestCard in main.rs

## Problem

The game currently starts with plebs spawning at map center with gear in their pockets. There's no arrival event, no wreckage, no scavenging. The starting conditions are identical every game except for crew skills. This misses an opportunity for narrative, replayability, and the feeling of arriving somewhere hostile and scrambling to survive.

## Design Principles

1. **Randomness as narrative, not power.** The question isn't "did I get a rare drop?" — it's "what survived the crash?" Each start creates a unique situation that demands adaptation.
2. **No rarity tiers.** No common/rare/epic/legendary. No color-coded borders. No stat inflation. Items are functional objects, not collectibles.
3. **Commitment at the manifest, surprise at the crash.** The player makes a deliberate choice picking crew (skills, traits, backstory). The crash introduces controlled randomness AFTER that commitment. You chose your people; fate chose your supplies.
4. **Every artifact tells a story.** Named items have backstories and specific functions. Finding one is a small narrative event, not a stat upgrade.

## The Crash Event

### Ship Types

Before the manifest screen, the player chooses (or is assigned) a ship type. The ship determines:
- How many crew slots are available (2-5)
- Which backstories appear in the applicant pool
- The salvage pool (what COULD survive the crash)
- The crash pattern (debris field size and shape)

| Ship Type | Crew | Theme | Salvage Pool |
|-----------|------|-------|-------------|
| **Mining Vessel** | 3 | Industrial | Metal scrap, drill bits, fuel cells, ore samples |
| **Colony Transport** | 4-5 | Civilian | Seeds, medical supplies, fabric, cooking gear |
| **Military Scout** | 2-3 | Combat | Weapons, ammo, armor plating, optics |
| **Science Expedition** | 3 | Research | Instruments, solar cells, data tablets, specimens |
| **Merchant Freighter** | 3-4 | Trade | Random assortment, diverse materials, surprises |
| **Prison Transport** | 4 | Social tension | Basic rations, restraints, guard/prisoner dynamics |

### The Landing

When the game starts (after manifest, transitioning to Playing):

1. The wreck appears at map center. Its shape depends on ship type (a few pre-designed footprints using existing block types — metal walls, broken floor).
2. Salvage crates scatter in a 10-20 tile radius around the wreck. 3-6 crates depending on crash severity (random).
3. Crew spawn around the wreck, not at a precise point.
4. An event card appears: "The [Ship Name] went down hard. Survivors: [crew count]. Salvage scattered across the landing site."
5. The first activity loop begins: find crates, open them, gather scattered materials, establish perimeter before nightfall.

### The Wreck as Shelter

The wreck itself provides partial walls and roof on day one. It's not comfortable (cold metal, -2 mood) but it blocks wind and duskweavers. The player can:
- Shelter in the wreck for the first night (safe but cramped)
- Dismantle wreck tiles over time for metal/material
- Build around the wreck, incorporating it into their base
- Abandon it entirely and build elsewhere

The wreck is a permanent map feature — a landmark. Named on the minimap. Over time it becomes the colony's origin story: "We started at the wreck."

## Salvage Categories

### Salvage (common, expected)

Raw materials and basic supplies. What you'd expect from a crashed ship. Not exciting but necessary.

- Metal scrap (building material)
- Torn fabric (rope/fiber equivalent)
- Bent pipes (usable as primitive pipes)
- Broken glass (sharp edges — knife material?)
- Ration packs (food, but limited)
- Water containers (some cracked, some intact)
- Wire spools (electrical)

Each item appears with varying quantity and condition. Durability is randomized: 20-80% of max for tools, freshness 30-90% for food. The crash wasn't gentle.

### Intact (less common, valuable)

Items that survived undamaged. Full durability. These feel like wins.

- A working tool (stone pick, knife, axe — type depends on ship)
- Sealed food container (full freshness)
- Medical kit (future: bandages, antiseptic)
- Intact rope coil
- Functioning lamp or torch
- Full water container

Intact items are the same things as salvage, just in better condition. Not "better items" — better condition of normal items.

### Artifacts (1-2 per game, unique)

Named items drawn from a pool of ~30+. Each has a name, a brief backstory, and a specific capability that's interesting but not game-breaking.

**Functional artifacts:**

| Name | Description | Effect |
|------|-------------|--------|
| Cracked Solar Cell | Survived the crash, barely. | Produces 30% power. Free energy from day one, but weak. |
| Emergency Flare | Military-grade distress signal. | Single use: lights map for one night, all creatures flee. |
| Water Purifier (damaged) | Portable filtration unit. | Makes water safe to drink. Breaks after 50 uses. |
| Friction Fire Kit | Bow drill set in a case. | Guaranteed fire start in any weather. Infinite uses. |
| Sealed Seed Packet | Unmarked agricultural container. | Plant to discover what grows. Random crop variety. |
| Old Star Chart | Navigation data from orbit. | Reveals 3 random glade locations through fog of war. |
| Broken Radio | Comm unit with shattered display. | Non-functional. Can be repaired with advanced materials. Once fixed: contact traders? |
| Ore Scanner (cracked) | Geological survey device. | Reveals mineral-stained terrain in a 30-tile radius. Like geology skill but electronic. |
| Tattered Field Manual | Previous expedition's notes. | One pleb gains a knowledge domain at Familiar level. Random domain. |

**Sentimental artifacts:**

| Name | Description | Effect |
|------|-------------|--------|
| Grandmother's Locket | Silver pendant with a faded photograph. | +5 permanent mood for the pleb who carries it. No other function. |
| Lucky Coin | Worn smooth from years of rubbing. | Carrier gets +2% success on skill checks. Or does nothing — who can tell? |
| Child's Drawing | Crayon picture of a house with trees. | +3 mood. The pleb talks about it sometimes. |
| Dog-Eared Novel | "Tales of the Outer Reach, Vol. 3." | Reading it at campfire gives +mood to all nearby plebs. Consumed after 5 reads. |
| Captain's Log | Final entries from the ship's commander. | Lore fragment. Explains why you're here. Hints at the world's history. |

**Risky artifacts:**

| Name | Description | Effect |
|------|-------------|--------|
| Emergency Rations (irradiated) | 10 meals. The packaging is cracked. | Fills hunger but 15% sickness chance per meal. Desperate food. |
| Unmarked Canister | Sealed metal container. Chemical smell. | Could be fuel, medicine, acid, or fertilizer. Opening it is a gamble. |
| Strange Fossil | Embedded in a chunk of hull plating. | Alien origin. Lore implications. Practically: trade value? Research value? |
| Ticking Device | Small box. Makes clicking sounds. | Is it a clock? A Geiger counter? A bomb timer? (It's a radiation detector. Probably.) |

**Design notes on artifacts:**

- Each artifact has a discovery event when found: card-style notification with the item name, a one-sentence description, and who found it.
- Artifacts cannot be crafted. They're found ONCE per game.
- The pool should be large enough (30+) that players can play dozens of games and still encounter new ones.
- Not all artifacts are useful. Some are pure flavor. The locket does nothing mechanically meaningful — but the player who finds it remembers it.
- Artifacts can be lost (pleb dies carrying it, dropped in deep water). This creates attachment and caution.

## Salvage Crates

Physical objects in the world — block type or ground item with visual (wooden crate, metal container, sealed case).

### Finding and Opening

Crates are visible on the ground after the crash. Walking near one adds it to the minimap. Opening requires a pleb to interact (right-click → "Open crate"). Takes 3-5 seconds.

Opening triggers a discovery event: "Ada opened a salvage crate. Found: Stone Pick (worn), 3x Ration Pack, Torn Fabric."

Contents are determined at generation time (not when opened). The player can see the crate but not its contents until opened. This creates small moments of anticipation.

### Crate Condition

Some crates are damaged:
- **Intact**: all contents in good condition
- **Cracked**: contents have reduced durability/freshness
- **Smashed**: only 30-50% of contents survived (rest is "debris")
- **Waterlogged**: food spoiled, metal rusted, but some items fine

Condition is visible on the crate before opening (color/visual). The player can prioritize which to open first.

## Crash Severity

A hidden die roll at game start determines how bad the crash was:

| Severity | Crates | Wreck condition | Crew injuries |
|----------|--------|----------------|---------------|
| Controlled landing | 5-6 | Mostly intact, good shelter | None |
| Hard landing | 4-5 | Damaged, partial shelter | Minor (1 pleb at 80% health) |
| Crash | 3-4 | Wrecked, minimal shelter | Moderate (1-2 plebs injured) |
| Catastrophic | 2-3 | Scattered debris, no shelter | Severe (all plebs injured, 1 critical) |

This isn't shown to the player as a number — they discover it by seeing how much wreckage there is and how hurt everyone is. A catastrophic crash is immediately obvious: less stuff, more injuries, wreck is just debris. A controlled landing feels lucky: intact hull, everyone standing, plenty of crates.

## Implementation Strategy

### Phase 1: Crash Site

- Wreck footprint: 3-4 pre-designed layouts (per ship type), placed at map center using existing block types (metal walls, floor, roof)
- Salvage crates: new ground item type, placed in radius around wreck
- Opening mechanic: right-click context action, pleb walks to crate and opens it

### Phase 2: Salvage Contents

- Crate content tables per ship type
- Condition randomization (durability, freshness)
- Discovery events on open (card notification)

### Phase 3: Artifacts

- Artifact definition file (artifacts.toml? Or in items.toml?)
- Pool selection at game start (1-2 drawn from pool, placed in random crates)
- Unique item properties (mood bonus, single-use ability, lore text)

### Phase 4: Ship Selection

- Pre-manifest screen: choose ship type
- Ship type filters backstory pool
- Ship type determines crate count, content pool, wreck layout

## Interaction with Existing Systems

- **Manifest chargen**: ship type selection happens before crew recruitment. Ship type determines available backstories.
- **Worldgen**: wreck placed at center after terrain generation, before the game starts.
- **Discovery system (DN-026)**: artifact finds trigger Tier 2 discovery events with thought bubble + journal entry.
- **Primitive tools (DN-025)**: crash salvage provides SOME tools to supplement what plebs start with. The tool progression still matters — salvage tools are damaged and temporary.
- **Fire system**: the wreck doesn't burn (metal). But wooden crates around it do. A campfire accident near the landing site could destroy unopened crates.

## Item Condition and Maintenance

### Philosophy: Demand, Not Chores

Durability should create ongoing resource demand (need more flint, need a crafter), not micromanagement (click repair on each tool). The player should barely notice durability — they feel it as "we keep running out of materials" or "things run smoother since we found flint."

**The player NEVER:** clicks "repair" on a tool, chooses which tool to equip, manages durability bars, or gets popup alerts about wear.

### Three Phases (automatic)

**Working** (100-30% durability): Tool functions normally. Tiny durability bar visible on hover in inventory. No player action, no pleb action.

**Worn** (below 30%): The pleb auto-sharpens after finishing their current task. Brief 2-3 second pause, uses hammerstone or loose rock from inventory, restores to ~60%. If no sharpening material available, the pleb keeps working until the tool breaks. This is nearly invisible — a brief pause and a *tink* sound.

**Broken** (0% durability): Thought bubble "My axe broke...", the tool drops as a ground item ("Broken Stone Axe"). Pleb immediately switches to the next available tool of the same type. If none: works bare-handed (much slower, pleb grumbles about it). Broken tools auto-queue for repair at any available workbench.

### Repair (automatic)

When a workbench exists and a broken tool is nearby (ground or storage):
1. An idle pleb with crafting skill picks up the broken tool
2. Walks to the workbench
3. Repairs it (10-20 seconds, consumes materials: 1 stick + 1 stone/flint depending on tier)
4. Repaired tool has 60% durability (not full — repaired isn't as good as new)
5. Drops the repaired tool or places it in storage

This is a work task like construction — automatic, prioritized by the work system, no player click needed.

### Tool Tier as Durability Motivation

| Tier | Durability | Sharpen restores | Effective lifespan | Player feeling |
|------|-----------|-----------------|-------------------|----------------|
| Primitive (stone blade) | 25 uses | ~15 uses | ~40 total | "These keep breaking" |
| Stone (stone axe) | 50 uses | ~30 uses | ~80 total | "Decent but wears out" |
| Flint | 60-80 uses | ~40 uses | ~100-120 total | "Now we're getting somewhere" |
| Metal (future) | 200+ uses | ~120 uses | ~320 total | "Finally reliable" |

The jump from stone to flint is noticeable — flint tools last 2-3x longer. This makes finding chalk deposits and learning to knap flint a meaningful progression moment, driven by the durability system.

### Crash Salvage Condition

Items from the crash arrive in varying condition:

- **Intact**: full durability. "Thank god this survived."
- **Damaged**: 20-50% durability. Works but worn stage triggers quickly.
- **Broken**: 0%. "Broken [item name]" in inventory. Needs workbench + materials to repair.

Crash severity affects the ratio: controlled landing → mostly intact. Catastrophic crash → mostly damaged/broken.

### Unknown State (separate from condition)

"Unknown" is about KNOWLEDGE, not WEAR. An unknown item can be in perfect condition — you just don't know what it does.

- Unknown items display as "Unknown Device", "Unmarked Canister", "Strange Object"
- A pleb auto-examines unknown items when idle near them (10-30 seconds, ties into DN-027 discovery)
- Examination reveals the real name + description + function
- Some items require specific knowledge to identify (chemistry for canisters, alien tech for devices)
- An unidentified item can still be USED if the player is brave — but the outcome is uncertain

Examples:
- "Unmarked Canister" → examined by chemist → "Emergency Fuel (intact)" OR "Industrial Solvent (handle with care)"
- "Strange Device" → examined by engineer → "Cracked Solar Cell (damaged)"
- "Sealed Container" → no one can identify it → player can risk opening it (random: useful/useless/harmful)

### What the Player Manages (Strategic)

The player thinks about tools at the COLONY level, not the item level:

1. **Supply chain**: "Do we have enough rocks/flint to keep tools maintained?" — material pressure
2. **Crafting priority**: "Should someone be making replacement tools?" — work scheduling
3. **Tier investment**: "Is it worth the trip to the chalk deposits for flint?" — exploration motivation
4. **Specialist assignment**: "Who has the best crafting skill for tool-making?" — crew management

None of these are "click repair on item #7."

## What This Does for the Game

1. **Every start is different.** Same map seed + different salvage = different early game.
2. **Immediate activity.** Instead of "place campfire, build wall" the first minutes are "find the crates, check on the injured, take stock."
3. **Narrative anchor.** "The Wreck" is a place with history. The colony story starts with the crash.
4. **Commitment + surprise.** You chose your crew (commitment). Fate chose your supplies (surprise). Both shape your strategy.
5. **Replayability.** 30+ artifacts × 6 ship types × variable severity = hundreds of unique starts.
6. **Tool progression feels earned.** Durability creates pressure to upgrade from stone → flint → metal. Each tier is a relief, not just a number increase.

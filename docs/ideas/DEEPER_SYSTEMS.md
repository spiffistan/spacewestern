# Deeper System Ideas

Second-layer ideas that add depth to existing systems. These assume the core game (survival, building, crafting, combat) is working and explore what makes a colony feel alive long-term.

## Food Beyond Berries

Right now food is berries. A colony that eats berries on day 1 and berries on day 100 has a flat experience.

**Cooking progression:**
- Raw berries/meat → edible but low mood
- Simple meal (fire + raw food) → decent nutrition, easy
- Stew (pot + water + meat + vegetable) → good nutrition + mood buff
- Fine meal (multiple ingredients, skilled cook) → best mood buff
- Preserved food (smoked, salted, dried) → lasts through winter, lower nutrition

**Variety matters.** Eating the same thing every day incurs a mood penalty. Three different foods in a week = bonus. This drives crop diversity and hunting.

**Spoilage.** Food has a freshness timer. Room temperature accelerates decay. Root cellar (basement level -1) slows it drastically. The thermal system directly affects food preservation — a warm storeroom ruins food, a cool basement preserves it.

**Alcohol.** Brewing at the workbench: berries → wine, grain → beer, surplus → moonshine. Alcohol provides stress relief (social drinking at the saloon) but overconsumption causes problems (drunk colonist = impaired work). A trade good with high value-to-weight ratio. The "Binge" mental break connects directly — a stressed colonist raids the liquor supply.

**Hunting.** Wild animals on the map provide meat. Requires combat skill and a weapon. Risk of injury. Connects to the animal system — some animals fight back.

## Medicine and Disease

The fluid sim tracks air quality (O2, CO2, smoke). Disease is the biological version of bad air.

**Illness spreads physically.** A sick colonist exhales infected air. The fluid sim carries it. Other colonists in the same enclosed space breathe it in. Ventilation (fans, open windows) disperses the pathogen. Sealed rooms concentrate it. Quarantine means isolating the sick in a room with its own air supply — the existing pipe/fan system handles this mechanically.

**Treatment progression:**
- Rest (bed, time) → slow recovery, risk of complications
- Herbal medicine (gathered plants, applied by any colonist) → faster recovery
- Frontier medicine (doc backstory, medical tools, clinic building) → reliable
- Surgery (rare, high skill check, can save a dying colonist or go very wrong)

**Injury vs illness:**
- Injuries from combat/accidents: bandage → splint → surgery. Visible on sprite (scars from CHARACTER_VISUALS.md)
- Illness from exposure/contamination: rest → medicine → quarantine
- Chronic conditions: old injuries that flare up in cold weather, reduced work capacity

**The doc is irreplaceable.** The Frontier Doc backstory gives medical skill that no one else has. If the doc dies, colonists with serious injuries or illness may not survive. This creates a powerful protect-the-doc dynamic and motivation to train a second medic.

## Knowledge and Oral Tradition

The most unique idea here: **knowledge lives in people, not buildings.**

In most games, you research "Smelting" at a research bench and it's unlocked forever. Here: the Mechanic knows how to maintain machines. The Engineer knows how to build bridges. The Doc knows how to set bones. If they die, that knowledge is gone unless they taught someone.

**Teaching:** An experienced colonist can teach their skills to others. Requires time together at a work site — the apprentice works alongside the master, slowly gaining competence. The master's work slows (they're teaching), but the colony gains redundancy.

**Blueprint cards** (from CARDS.md) represent external knowledge — discovered, not invented. They unlock a recipe permanently. But the skill to execute that recipe efficiently still lives in the colonist.

**Specialist death is devastating.** Your only blacksmith dies in a raid. You still have the anvil, the recipes, the materials. But nobody left knows the nuances — work takes twice as long, quality is lower, until a new colonist levels up the skill or a new specialist arrives.

**Books and notes.** A literate colonist can write down knowledge (crafting a book at the workbench). Books preserve knowledge beyond the specialist's life. But a book teaches slower than a master — it's insurance, not a replacement. A library building stores books — the colony's accumulated wisdom.

This makes people the most valuable resource, not materials. Losing a veteran hurts more than losing a building.

## Psychology Beyond Stress

The stress system (0-100, mental breaks at 85+) is a single axis. Real psychological depth comes from individual variation.

**Memories, not just a number.** Each colonist accumulates positive and negative memories:
- "Ate a fine meal" (+3 mood, fades in 2 days)
- "Saw a friend die" (-15 mood, fades in 30 days)
- "Survived a raid together" (+5 mood with everyone present, permanent bond)
- "Slept in the cold" (-5 mood, fades in 1 day)

Mood = sum of active memories. This is how Rimworld does it and it works beautifully — you can inspect WHY someone is stressed, not just that they are.

**Phobias and preferences:**
- Claustrophobic: stress builds underground
- Pyromaniac: fascinated by fire, may start one during mental break
- Night owl: works better at night, stressed during forced day shifts
- Ascetic: doesn't need luxury, happy with simple meals and bare rooms
- Greedy: needs nice things, stressed by poverty

These emerge from chargen traits and make each colonist's experience unique.

**Trauma.** Witnessing death creates a lasting trauma memory that occasionally triggers flashbacks (brief work interruption, mood dip). Multiple traumas compound. A colonist who's seen too much becomes "hardened" — less affected by new trauma but also less capable of joy (emotional numbing trait).

**Bonds.** Shared crisis creates bonds between colonists. Two colonists who survived a cave-in together become friends. Bonds are visible in behavior (head turns, standing proximity from CHARACTER_VISUALS.md). A bonded pair fights harder together in combat.

## Building Aesthetics and Beauty

How a room looks affects who lives in it.

**Room quality** depends on:
- Spaciousness: tiles per occupant (cramped = stress)
- Flooring: rough < wood < stone (higher quality floors = better mood)
- Walls: mud < wood < stone < decorated
- Furniture: basic bed < nice bed, bench < table + chairs
- Decorations: paintings, carvings, flowers
- Light: natural light from windows, warm lamp light
- Cleanliness: dirt floors track mud, stone floors stay clean

**Beauty stat per room.** Colonists living/working in beautiful rooms get a mood buff. Ugly rooms (dirt floor, no windows, bare walls) give a mood debuff. This drives players to invest in aesthetics beyond the purely functional — you COULD sleep in a dirt-floored box, but your colonists will be happier in a proper room.

**Art objects.** A colonist with artistic skill can craft:
- Paintings (wall decoration, random quality)
- Carvings (placed on tables/shelves)
- Engravings (applied to stone walls)

Art quality varies by skill — a masterwork painting is a prized possession. Art subjects reference colony events: "A vivid painting of the Great Redskull Raid of Summer Year 2."

**Windows matter.** A room with windows lets in natural light and gives a view bonus. Connects to DN-005 (windows as wall features). A windowless basement room is functional but depressing.

## Mapping and the Unknown

The world beyond your colony is unexplored and dangerous.

**Expedition system.** Send a colonist (or small group) to explore in a direction. They leave the colony, disappear into fog of war, and return after N days with:
- Map reveals (terrain, water sources, ruins discovered)
- Found items (scrap, artifacts, blueprint cards)
- Encounters (hostile, neutral, or friendly — resolved via card system)
- Injuries/stories

You don't control them during the expedition — you send them off and hope. The scout backstory gives better expedition outcomes.

**Points of interest.** The wider map has locations you discover through exploration:
- Abandoned settlements (salvage opportunity)
- Water springs (vital during drought)
- Ore deposits (motivation to build outposts)
- Enemy camps (Redskull base — raid source)
- Ancient ruins (deep lore, blueprint cards)
- Natural features (canyon, river crossing, cave entrance)

**The map fills in over time.** Early game: tiny known circle around your colony, vast unknown. Late game: extensive mapped territory with known resources and threats. The unknown is always out there at the edges.

## Communication Systems

How does your colony connect to the wider world?

**Smoke signals** (early): visible from far away, attracts attention (trade AND enemies). Simple: build a signal fire, light it.

**Mirror signals** (early-mid): line-of-sight communication between two points. Requires clear sightline and daylight. Used for watchtower → base alerts.

**Telegraph** (mid-game): requires copper wire (mining!), power, and a telegraph machine (crafted). Sends text messages between connected stations. Could contact other settlements for trade/help. Needs the power grid AND the wire system — both already exist. A telegraph line running from your colony to a distant trading post.

**Radio** (late-game): requires advanced electronics. Broadcasts to anyone with a receiver. Could attract help OR unwanted attention. Range-based — more power = longer range.

Each tier expands your connection to the outside world. Early game you're isolated. Late game you're networked.

## The Endgame Question

What are you building toward?

**Option A: Sandbox (no win condition).** You play until you stop. The colony grows, faces crises, tells stories. This is how most people play Rimworld. The journey is the point.

**Option B: The mystery.** You find fragments of what happened to previous settlers. Ancient ruins at depth -5 hold answers. Piecing together the full story IS the endgame — a narrative goal that drives exploration and depth.

**Option C: The beacon.** Build a communication beacon powerful enough to contact the homeworld. Requires mastery of power (massive energy), mining (rare materials from deep), and crafting (advanced components). The beacon is a 30+ game-day megaproject. When activated: ending cinematic. But do you WANT to be found?

**Option D: The choice.** At the beacon moment, you choose: call for help (civilization arrives, colony becomes a proper settlement, you "win") or stay silent (remain independent, continue building, no rescue but no outside control). The choice reflects how you played — isolationist vs connected.

**My instinct:** Option A as default (sandbox), with Option B/D as optional narrative thread for players who want direction. The mystery/beacon is always there to pursue but never forced.

## Outposts and Multi-Settlement

Late game: your colony is stable. Now what?

- **Mining outpost:** a small camp near a deep ore deposit. 2-3 colonists, basic shelter, connected by road to main base. Hauls ore back periodically. Vulnerable to raids.
- **Farm outpost:** expanded growing area beyond main walls. Seasonal workers.
- **Watchtower:** single colonist on a hilltop with a mirror signal to base. Early warning of approaching threats.
- **Trade post:** on the road, attracts caravans. A dedicated colonist runs it.

Each outpost is a tiny separate colony — its own buildings, its own risks. Supply lines between them matter. A raid that cuts the road between your mine and your base starves both.

This extends the map from "one base" to "a network" — far more strategic depth.

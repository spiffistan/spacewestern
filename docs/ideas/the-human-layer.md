# The Human Layer

Ideas about the narrative, psychological, and cultural dimensions of the colony. Where EMERGENT_PHYSICS.md exploits the simulation stack, this doc explores what makes the *people* interesting. The physics make the world real; the human layer makes it matter.

## Guiding Principle

A colony sim where you only optimize systems is Factorio. A colony sim where you only manage people is The Sims. Rayworld should be the place where physical systems and human drama collide — where the fluid sim creates a crisis, and the crisis reveals who your people are.

---

## The Planet Has a History — And It's Wrong

The underground ruins (MULTI_LEVEL.md) don't look like human construction. The hollowcall (ALIEN_FAUNA.md) doesn't sound like a creature — what if it's artificial? An ancient signal still broadcasting from a buried installation, degraded beyond recognition into something that sounds organic.

**The lore:** This planet was *engineered*. The terrain, the ecosystem, the alien fauna — designed by something that's gone. The thermogast's attraction to heat isn't random evolution — it was a maintenance organism, drawn to failing thermal systems in ancient infrastructure. The glintcrawlers are pest control that outlived their purpose. The hollowcall is a beacon nobody's coming to answer.

Never stated outright. Discovered through fragments: a wall texture deep underground that's too regular to be natural. A blueprint card describing a machine with no human analog. A pattern in the terrain only visible from the map overview. The mystery is never fully answered — just enough to make the player uneasy about what they're building on top of.

**Gameplay implication:** Digging deep finds *context*, not just resources. Some of that context reframes the surface. "Oh. The hollowcalls aren't creatures. They're echoes of a system running for millennia." The lore doesn't unlock through a quest log — it accumulates through physical discovery. A colonist finds an inscription. The engineer deciphers part of it. The rest is lost. The player fills in the gaps themselves.

**Connects to:** MULTI_LEVEL.md (underground ruins), ALIEN_FAUNA.md (creatures as remnant biotech), EMERGENT_PHYSICS.md (ghost infrastructure)

---

## The Radio as Narrative Engine

Game-design.md mentions a radio tower. A radio could be the game's most powerful storytelling device. Once built, you pick up transmissions:

- Fragments of other colonies struggling with their own crises
- Automated distress signals from ships that crashed years ago
- Coded military communications you can't fully decode
- A weather report from a distant settlement giving advance storm warning
- Static-laden broadcasts from... somewhere else
- A repeating signal in a language nobody recognizes
- A broadcast that seems to be describing YOUR colony from the outside

The radio doesn't give quest markers. It gives *voices*. The player tunes the dial (frequency slider). Different frequencies carry different signals. Some are time-sensitive — a distress call that goes silent after 3 days. Some are recurring. Some are dangerous — responding to certain signals brings visitors you didn't want.

**The narrative power:** The radio connects the colony to a wider world that's implied but never fully seen. It's information, atmosphere, and threat in one device. A woman describing her colony's losing battle with winter — do you send help? Can you afford to? A man offering trade but his coordinates don't match anything on your map — trap or glitch?

The radio runs through the power grid. A power outage during a storm silences it when you need it most.

**Sound sim connection:** Radio transmissions play through a speaker block. The sound physically propagates from the speaker into the room. Colonists in earshot hear it. A colonist alone in the radio shack at 3 AM, listening to a voice describing something impossible — that's a scene the game generates, not scripts.

---

## Dual-Use Everything — The Tool/Weapon Philosophy

The space western means scarcity. People don't have dedicated weapons AND dedicated tools. Everything serves double duty. This should be a design *principle*, not just a feature:

- The pickaxe mines ore and splits skulls
- The kiln fires clay pots and heats cannonballs
- The pipe system ventilates rooms and can be weaponized with toxic gas
- The fan circulates air and creates pressure differentials for traps
- The campfire cooks food and cauterizes wounds
- The signal fire calls for trade and reveals your position to enemies
- Dynamite opens mine shafts and blows apart barricades
- The telescope maps stars and spots approaching raiders
- Alcohol is medicine, trade good, social lubricant, and the thing a stressed colonist binges on

Every survival system has an offensive or defensive application. Every weapon has a peacetime use. A well-designed colony IS a fortress. A well-designed fortress IS a functioning colony.

**The crafting tree should reflect this.** You don't research "weapons" as a separate branch. You research "metalworking" — that gives better tools AND better weapons simultaneously. The frontier doesn't distinguish.

---

## The Passage of Seasons on a Person

Colonists who've been here for two years are fundamentally different from fresh arrivals. Not just higher skills — a different relationship to the place.

**Acclimatization:** New arrivals are anxious, jumpy, stressed by alien fauna sounds. After a season, calmer. After a year, the hollowcall is just background noise. But they've lost something — letters from home hit harder. They're becoming *of this place*.

**Frontier hardening:** Colonists who survive crises become resilient but less empathetic. A hardened veteran doesn't get mood debuffs from witnessing death — but also doesn't get mood buffs from social interaction as easily. Effective but emotionally distant. The colony's best worker might be its loneliest person.

**Nostalgia cycles:** Certain triggers — a weather pattern, a food that reminds them of home, the anniversary of the crash — cause mood swings. Not random. Connected to their personal timeline. A colonist who arrived in autumn gets melancholy every autumn. The game tracks *personal calendars* of significant events.

**The old-timer effect:** The longest-surviving colonist becomes a psychological anchor. Others look to them for stability. If the old-timer dies, the mood cascade hits harder than any other death — they were the living history of the colony. New arrivals ask: "What happened to Red?" and the answer defines the colony's culture — "We don't talk about that" vs. "Let me tell you about the bravest person I ever knew."

**Connects to:** DEEPER_SYSTEMS.md (psychology beyond stress), PHILOSOPHY.md (permanence, the colony as character)

---

## Dreams and Omens

At night, sleeping colonists dream. Dreams are procedurally generated from recent events, personality traits, and stress levels:

- A colonist who nearly died in a fire dreams about flames. Wakes up with a brief mood debuff and avoids fireplaces for a day.
- A colonist who misses a dead friend dreams about them. Wakes up sad but slightly comforted — the dream processed grief, shortening the mourning period.
- A stressed colonist dreams about the hollowcall. Wakes up convinced it means something. If multiple colonists dream about the hollowcall in the same week, colony-wide unease builds.
- A happy, well-fed colonist dreams about a place they haven't been — a clue about an undiscovered ruin or resource. The game nudges exploration without quest markers.
- A colonist with the pyromaniac trait dreams about starting fires. A warning.
- A new colonist dreams about home. A veteran dreams about THIS place — they've shifted.

Dreams appear in the colonist's log but are never explained. The player interprets them. "Jenna dreamed about water underground" might mean there's a spring nearby. Or Jenna might just be thirsty. The ambiguity is the point — it feels like the frontier, where people read signs in everything because they have nothing else to go on.

**The mechanical subtlety:** Dreams can be wrong. Most are just emotional processing. But *occasionally* a dream correlates with reality (a hidden resource, an approaching threat). The player never knows which dreams to trust, just like the colonists. Over time, players develop superstitions about which colonists have "reliable" dreams — emergent folklore from a mechanical system.

---

## The Saloon as Social Physics

Not just "mood buff room." The colony's social *engine*.

Colonists gather at the saloon in the evening. Who sits with whom is determined by relationships. Friends cluster. Rivals take opposite corners. A loner sits at the bar alone. These positions are visible — you can SEE the social topology by watching where people sit.

**Conversations are sound sources.** A heated argument is louder (higher amplitude in the sound sim). A whispered conspiracy is quiet — but audible if you're listening from the next room through a thin wall. Laughter boosts nearby colonists' mood. A fight erupts: breaking furniture, other colonists intervening or backing away.

**Spatial design matters:**
- Big open saloon where everyone sees and hears each other → unified social group, conflicts are public, peer pressure works
- Private booths → cliques form, secrets are possible, conspiracies can develop
- A stage → the musician performs (sound sim mood buff)
- Near the barracks → soldiers stay social
- Near the workshop → workers talk craft (skill sharing from DEEPER_SYSTEMS.md knowledge system)

**The saloon is where:**
- Rumors spread (information propagates through social network)
- Grudges form (two colonists who argued avoid each other next day)
- Romances start (sitting together repeatedly → bond forms)
- Culture is negotiated ("We bury our dead properly" vs. "We're practical, compost them")
- The colony's mood is visible at a glance — a rowdy saloon = healthy colony, a quiet one = trouble

**Connects to:** PHILOSOPHY.md (stories around the fire), DEEPER_SYSTEMS.md (psychology, bonds), GAMEPLAY_SYSTEMS.md (music and morale)

---

## Names for Things — Language Emerges

Colonists are on an alien world. Over time, they name things. The first major landmark gets a name based on who discovered it and what happened:

- "Jenna's Ridge" (where Jenna found ore)
- "The Drowning" (the flooded basement)
- "Blackpit" (the mine where Marcus died)
- "Smokebreak Pass" (the corridor that always fills with smoke when the wind shifts)
- "The Quiet" (an area where the hollowcall doesn't reach)

These names appear on the map, in colonist logs, in conversation. New arrivals learn them — "What's Blackpit?" "Don't ask." The colony develops its own *vocabulary* for its geography.

**Creature naming:** The alien fauna gets named by the first colonist to encounter them. If the tough ex-military veteran discovers duskweavers, they get a practical name — "clickrunners." If the poetic preacher finds them, they get a dramatic name — "the whispering many." The name sticks for the rest of the playthrough. Different colonies call the same creature different things.

**Event naming:** "The Long Dark" (the cold snap of winter year 1). "The Red Morning" (the dawn raid). Generated from event type + severity + season. Used in colonist logs and stories around the fire.

Place names, creature names, event names — all procedurally generated from colonist personalities and discovery circumstances. The colony's language is unique to each playthrough. Storytelling that emerges from systems, not authored content.

---

## The Economy of Scrap

Technology is salvaged, not manufactured. The starting wreck has components. Other wrecks dot the landscape. Trade caravans bring scrap from crash sites. Every advanced item is built from *recombined pieces of a technological past nobody fully understands*.

A colonist doesn't "research electronics." They find a circuit board in a wreck, the mechanic figures out what it does through trial and error (chance of failure, chance of electrocution), and eventually it becomes a component in something new.

**The research tree isn't a tree — it's a junk pile.** What you can build depends on what you've found and who understands it. This creates unique tech paths per playthrough:

- Colony A finds a solar cell early → power grid focus
- Colony B finds a chemical processor → industrial chemistry chain
- Colony C finds a radio transmitter → makes contact with the wider world before they have reliable shelter
- Colony D finds medical supplies → becomes a regional hospital, attracting injured travelers

The randomness of salvage determines the colony's technological identity.

**Blueprint cards** (from CARDS.md) are literal schematics found in wreckage. Without the right backstory colonist (engineer, mechanic), a blueprint is just a pretty drawing. With them, it's the key to a new capability. Knowledge lives in people (DEEPER_SYSTEMS.md), and blueprints bridge the gap.

**The emotional hook:** A colonist holds up a piece of circuitry from a wreck. "I think I know what this does. I used one back on..." They trail off. The scrap connects them to a past they can't go back to. Working with salvage is bittersweet — every useful thing they build is a reminder of the civilization they lost.

**Connects to:** DEEPER_SYSTEMS.md (knowledge lives in people), CARDS.md (blueprint cards), game-design.md (the frontier, scarcity)

---

## Ghost Infrastructure — The Planet's Skeleton

The planet was engineered (see "History" section above). Ancient infrastructure exists underground. Pipes carrying unknown fluids. Conduits humming with residual energy. Vents circulating air from unknown sources.

When you dig deep enough, you break into this ancient system. Your pipe network can *connect to theirs*. What flows through the ancient pipes isn't water — a nutrient solution, a coolant, something with no analog.

**The choice:** If the mechanic figures out what it does (experimentation, risk of failure), the ancient fluid becomes a resource — maybe it accelerates crop growth, or heals wounds, or powers ancient machinery. If you don't understand it and let it flow into your water supply... consequences.

**Ancient power conduits** provide electricity — but at wrong voltages and frequencies. Lights flicker. Fans run backwards. Circuit breakers trip. An engineer can build an adapter. But the ancient power has properties: it attracts borers, or agitates the thermogast, or changes the hollowcall's frequency.

The ancient infrastructure is a gift and a risk. The planet's skeleton, exposed when you dig. Integrate with it or seal it off? Each choice cascades through the interconnected systems.

**The deeper question:** If the planet was engineered, and the ancient systems still run, and the creatures were designed to maintain those systems... what role does YOUR colony play? Are you settlers, or are you an immune response the planet is mounting against an infection?

---

## Silence as Formal Mechanic

PHILOSOPHY.md touches on silence as atmosphere. Make it a mechanical state.

The game tracks ambient sound level per area. When it drops below a threshold — no wind, no insects, no machinery — the game enters a *silence state*:

- Colonists become hyperaware — stress builds slowly from tension
- The sound sim amplifies tiny sounds — a normally-lost footstep is audible across the map
- Alien fauna shifts behavior — the mistmaw hunts more aggressively in silence
- The player's own attention sharpens — without background noise, every creak stands out

**Silence happens:**
- Before storms (the calm before)
- After mass creature death (something killed the borers — why?)
- During rare events that clear the map of ambient life
- When the hollowcall stops

**The most powerful version:** The hollowcall has been a nightly fixture for seasons. Then one night — nothing. The colonists notice. The log says "Unusual silence tonight." The player knows something changed but not what. The tension is unbearable precisely *because nothing is happening*.

Silence is an absence that signifies presence. The game teaches: when the world goes quiet, something is about to happen. Or already has.

---

## The Weight of a Name

Every colonist RimWorld gives you has a name. You forget most of them. What if Rayworld made you *remember*?

**Names earn weight through association.** A colonist named Kai means nothing on day 1. But Kai built the first wall. Kai survived the cold snap by burning their own bed for fuel. Kai's the one who figured out the ancient circuit board. Now "Kai" means something — it means the colony's history.

When Kai dies, the name doesn't vanish. New colonists reference it: "Kai's workshop" (the workbench they built). "Kai's method" (the technique for firing clay they invented). A gravestone with their name. The colony carries them forward in language (see "Names for Things" above).

**The mechanical version:** Colonists have a "legacy" score that accumulates with achievements, crises survived, things built, skills taught. High-legacy colonists contribute more to colony mood when alive, and their death has a proportionally larger impact. A legacy-0 newcomer who dies on day 2 is sad. A legacy-50 founder who dies in year 3 is devastating.

The player learns: protect the people who matter. Not every colonist matters equally — but which ones matter is determined by what happened, not stats.

---

## The Manifest — Crew as Fate

CHARGEN.md describes "The Manifest" — the starting crew. What if crew selection felt less like character creation and more like *drawing a hand of cards*?

You don't pick your crew. You're shown a manifest of available colonists — more than you can take. Each has a backstory, traits, skills, and a *reason they're on this ship*. You choose who to wake from cryo. The ones you don't choose stay frozen in the wreck — and can be woken later as a desperate measure (but cryo-delay has consequences: confusion, skill loss, trauma).

**The manifest has hidden connections.** Two colonists who seem unrelated share a past — the outlaw is wanted for killing the doctor's brother. The mechanic and the engineer were rivals at the same academy. The preacher is running from the same military the deserter deserted from. These connections emerge over time through saloon conversations, dreams, and radio transmissions that reference people by name.

The starting crew isn't just a skill allocation problem. It's a *cast of characters* whose interactions are pre-loaded with tension. The player doesn't know the tensions until they surface — and by then, it's too late to change the crew.

---

## Moral Drift — The Colony Changes You

The colony starts with values. Maybe the preacher insists on burying the dead properly. The doctor insists on treating wounded enemies. The sheriff insists on fair trials for prisoners.

Then winter comes. Food runs low. A raider is captured. Do you feed the prisoner when your own people are hungry? The preacher says yes. The outlaw says no. The player decides.

Each decision shifts the colony's moral center. Not a binary good/evil slider — a constellation of values that drift based on choices:

- **Mercy vs. Pragmatism:** Do you help strangers at cost to yourself?
- **Justice vs. Survival:** Do you follow rules when breaking them would save lives?
- **Openness vs. Security:** Do you welcome newcomers or view them as threats?
- **Memory vs. Moving On:** Do you honor the dead or recycle their belongings?

These aren't abstract — they manifest in colonist behavior. A colony that chose pragmatism over mercy three times starts to *feel* different. Colonists stop offering to help wounded strangers without being told. The saloon conversations get harder. New arrivals sense the mood.

**Moral conflict as drama.** The preacher confronts the player (through a log entry / event card) after the third pragmatic choice: "We came here to be better than this." If the preacher leaves or is overruled, the colony loses its moral voice — and some colonists feel relieved while others feel lost.

The colony's moral identity is the player's moral identity, reflected back through fictional characters who react to it. This is what RimWorld's Ideology DLC attempts abstractly. Here it emerges from choices under pressure.

---

## Superstition and Folk Belief

Colonists stranded on an alien planet with unexplained phenomena will develop beliefs. Not religion (too structured) — superstition. Folk explanations for things the simulation does.

- "The hollowcall is louder before storms." (Possibly true — weather correlation.)
- "The thermogast won't cross running water." (Testable — is it true?)
- "Glintcrawlers avoid camphor plants." (The scent system might support this.)
- "Mining on a quiet night brings bad luck." (Coincidence — or is the silence mechanic real?)
- "Kai's ghost walks the workshop." (The thermal footprint system leaves traces — a warm spot where no one's been?)

Superstitions spread through the saloon. A colonist states a belief. Others adopt or dismiss it based on personality (the engineer dismisses; the preacher considers; the scout trusts gut feelings). Over time, the colony develops a shared folk mythology about their planet.

**The design trick:** Some superstitions are *mechanically true* (the simulation actually does the thing). Some are false (coincidence that the colonist over-interpreted). Some are half-true (correlated but not causal). The player never knows which category a superstition falls into without testing it — and testing might be costly.

This gives the colony a cultural life that emerges from the simulation's opacity. The physics are deterministic, but the colonists don't have access to the debug overlays. They interpret the world the way real frontier people did — with pattern-matching, storytelling, and educated guesses.

---

## The Letter Home

Each colonist can write a letter. They'll never send it — there's no postal service on an alien frontier. But the act of writing it provides a mood buff (processing emotions) and the letter itself is a narrative artifact.

Letters are procedurally generated from the colonist's recent experiences, personality, and relationships:

*"Dear Mother, the nights here are long. There's a sound in the dark — we call it the hollowcall. Jenna says it's just wind in the caves but I don't think wind sounds like that. We finished the workshop roof today. Kai showed me how to fire clay the way they do it. I'm getting better. I miss your cooking. I don't miss the rain. — M."*

Letters reference real events, real people, real places (using the colony's own names). They're the colonist's perspective on the story the player is living. Reading them gives the player a window into how their management decisions feel from inside.

A letter written during a crisis reads differently than one written during peace. A letter from a hardened veteran reads differently than one from a fresh arrival. A letter from a colonist who's about to die — found in their belongings afterward — is the game's most devastating narrative moment. And it's procedurally generated from systems.

**Found letters from the past** (GAMEPLAY_SYSTEMS.md) work the same way but in reverse: letters from previous inhabitants found in ruins, describing events the player can see the physical evidence of. "The basement flooded again. Third time this season. We should have built on the hill." The player looks at the flooded ruin. The physics confirm the letter.

---

## The Colony's Song

Every colony develops a sonic identity. Not composed music — the *accumulated ambient sound* of the settlement.

A small colony sounds like: wind, fire crackle, one or two footsteps, distant hollowcall. Intimate. Quiet. Vulnerable.

A large colony sounds like: hammering, fans humming, pipe pressure hissing, multiple conversations in the saloon, the kiln roaring, footsteps everywhere, the alarm bell's echo still fading. Industrial. Alive. Loud.

The transition between these soundscapes happens gradually as the colony grows. The player doesn't notice it changing — but if they load an early save after playing a late-game colony, the silence is *deafening*. The sound sim has been tracking the colony's growth the entire time, and the acoustic difference IS the growth.

**The most powerful version:** When a colonist dies, their individual sound contributions stop. The workshop they used goes quiet. Their footstep pattern disappears from the ambient mix. The colony sounds *different* without them — not dramatically, but in a way the player feels without being able to name it. An absence in the soundscape. A hole in the colony's song.

---

## Summary — What These Ideas Share

These aren't physics exploits (see EMERGENT_PHYSICS.md). They're about making the colony feel *inhabited*. The common threads:

**Emergence over authorship.** Names, superstitions, dreams, letters, moral drift — all generated from systems interacting, not written content. Every playthrough produces unique culture.

**The colony as character.** PHILOSOPHY.md states this as a goal. These ideas are the mechanics that make it real. The colony has a vocabulary, beliefs, moral identity, sonic signature, and memory — all emerging from gameplay.

**Ambiguity as design tool.** Dreams that might mean something. Superstitions that might be true. Radio signals from uncertain sources. The planet's engineered history, never fully explained. The player fills in gaps with their own interpretation — and that interpretation becomes part of their story.

**People are irreplaceable in specific ways.** Not just labor. Kai's legacy, the preacher's moral voice, the mechanic's knowledge, the old-timer's stabilizing presence. Losing a colonist doesn't just reduce workforce — it changes the colony's identity. This is the space western's emotional core: every person matters because there are so few of them, and none of them chose to be here.

---

## Connection to Other Docs

| This Idea | Connects To |
|-----------|-------------|
| Planet history, ghost infrastructure | `MULTI_LEVEL.md`, `ALIEN_FAUNA.md` |
| Radio as narrative engine | `game-design.md` (radio tower), `EMERGENT_PHYSICS.md` (sound sim) |
| Dual-use everything | `COMBAT.md` (weapons), `CRAFTING.md` (recipes) |
| Passage of seasons on a person | `DEEPER_SYSTEMS.md` (psychology), `PHILOSOPHY.md` (permanence) |
| Dreams and omens | `DEEPER_SYSTEMS.md` (psychology), `ALIEN_FAUNA.md` (hollowcall) |
| Saloon as social physics | `GAMEPLAY_SYSTEMS.md` (music, morale), `DEEPER_SYSTEMS.md` (psychology, bonds) |
| Names for things | `PHILOSOPHY.md` (map as memory, colony as character) |
| Economy of scrap | `CARDS.md` (blueprints), `DEEPER_SYSTEMS.md` (knowledge), `game-design.md` (the frontier) |
| Ghost infrastructure | `EMERGENT_PHYSICS.md` (pressure, pipes), `MULTI_LEVEL.md` |
| Silence as mechanic | `PHILOSOPHY.md` (silence), `ALIEN_FAUNA.md` (mistmaw, hollowcall) |
| The weight of a name | `PHILOSOPHY.md` (permanence, colony as character) |
| The manifest | `CHARGEN.md` (crew selection), `CARDS.md` (card system) |
| Moral drift | `DEEPER_SYSTEMS.md` (psychology), `GAMEPLAY_SYSTEMS.md` (reputation) |
| Superstition | `ALIEN_FAUNA.md` (creature behavior), `EMERGENT_PHYSICS.md` (simulation opacity) |
| Letters home | `GAMEPLAY_SYSTEMS.md` (letters and narrative fragments) |
| Colony's song | `EMERGENT_PHYSICS.md` (acoustic ecology), `PHILOSOPHY.md` (silence) |

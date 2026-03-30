# Aside Ideas

Quick-capture ideas that don't have a home yet. May develop into full docs or design notes.

---

## Thumper

A placeable device that pounds the ground rhythmically. Heavy low-frequency impulse injected into the sound simulation — propagates through the map physically, bounces off cliffs, attenuates with distance.

**Primary use:** Draws out burrowing creatures. Duskweavers are curious/territorial — a thumper near their burrow provokes them to investigate during the day instead of raiding at night. Lure them into a trap.

**Escalation risk:** Bigger creatures respond too. A thumper running for days might attract a Thermogast from deep in the map. You wanted to clear a duskweaver den — now you have a building-sized heat-seeking tank approaching your colony. "Be careful what you call."

**Secondary uses:**
- Seismic survey: the sound reflection pattern reveals underground features (mineral deposits, caves, water table depth). The metal detector finds metal — the thumper finds everything else.
- Pest control: keeps small critters away from crops (continuous low-level thumping). But attracts predators if left on too long.
- Communication: two colonies with thumpers can send signals through the ground over vast distances. Morse code through stone.

**Implementation:** Just a block type that injects periodic high-amplitude low-frequency sound sources into the existing sound sim. The creature AI already reacts to sounds. The escalation mechanic: creatures have a `curiosity_threshold` — small thumps attract small creatures, sustained or powerful thumps attract apex predators.

---

## Drones (Late Game)

Unlocked through alien technology research (from DN-010 artifact study + LORE_AND_RESEARCH lore progression).

### Surveillance Drones

Small flying units that extend fog of war vision. Not colonists — automated machines.

- **Craft:** workbench + alien tech fragment + wire + battery
- **Deploy:** launches from a drone pad, flies to designated patrol area
- **Vision:** reveals fog of war in a radius around the drone, like a flying eye
- **Duration:** battery-limited (5-10 minutes real time), returns to pad to recharge
- **Vulnerability:** can be shot down by enemies. Lost drone = lost materials.
- **Patrol modes:** circle area, follow path, shadow a specific colonist
- **Night vision:** drones see in the dark (infrared from thermal sim data). Reveals duskweavers, thermogasts, approaching raiders before they reach torch range.

**Tactical value:** Early warning system. A drone circling the colony perimeter spots raiders 30 tiles out. A drone shadowing a scout doubles their vision range. A drone over a duskweaver den watches their movements.

### Killer Drones

Armed variants. Late-late game. Ethically questionable (lore implication: the previous civilization used these — it didn't end well).

- **Craft:** surveillance drone + weapon component + alien AI fragment
- **Armament:** small-caliber gun (low damage, high rate of fire, poor accuracy) or single explosive charge (kamikaze)
- **Autonomy levels:**
  - Manual: player designates target, drone engages
  - Patrol: drone fires at any enemy in patrol zone
  - Autonomous: drone decides targets (risk: friendly fire if AI fragment is damaged/corrupted)
- **Counters:** EMP (disables all drones in radius), shooting them down, jamming (radio block type)

**The ethical tension:** Killer drones are enormously powerful. A colony with drone defense is nearly impervious to raids. But the alien tech lore reveals: "They built drones too. Their drones decided humans were the threat." Using killer drones unlocks a special lore entry — and attracts attention from something that doesn't like competition.

### Drone Implementation Notes

Drones are NOT plebs. They're physics bodies with flight (z > 0 permanently, no gravity). They have a simple FSM (patrol → engage → return → recharge) similar to creatures. They use the existing bullet system for weapons. Vision extends fog of war via the existing fog computation (just add drone positions to the vision source list).

The alien tech progression gates them: you need 3+ artifact fragments studied to unlock the surveillance drone recipe, 5+ for killer drones. This puts them firmly in late game.

---

---

## Sunskirter

The word is too good for one thing. It could be three things that share a name — each named after the other, layered into the world's history.

### 1. The Creature: Sunskirter (Terminator Runner)

An alien creature that exists only at the boundary between day and night. It moves WITH the dawn — racing along the terminator line as light sweeps across the planet.

**What the colonists see:** Twice a day, for a few minutes, a shape crosses the landscape at impossible speed. At dawn it runs west-to-east, chasing the light. At dusk it runs east-to-west, chasing the dark. Always on the exact line where shadow meets sun. Then it's gone.

**What it actually is:** The sunskirter feeds on the thermal gradient at the terminator — not heat, not cold, but the *rate of change* between them. The steeper the gradient (clear day, sharp shadows), the faster and more energized it is. Overcast days: sluggish, barely visible. Clear days after cold nights: blazing fast, leaves a trail of disturbed dust.

**Behavior:**
- Appears only during dawn and dusk (the 0.15-0.20 and 0.80-0.85 day fraction windows — same as creature spawn windows)
- Moves at extreme speed (20+ tiles/sec) along the east-west axis
- Cannot be caught, cannot be fought during transit
- Occasionally STOPS when the gradient is disturbed — a large fire at dusk creates an artificial thermal edge that confuses it
- When stopped, it's vulnerable. And valuable — sunskirter carapace fragments are the rarest crafting material on the planet
- Completely silent. No sound injection. The absence of sound as it passes is the cue.

**Visual:** A blur of iridescent light, like heat haze given form. The shader renders it as a lens-distortion effect along its path — pixels behind it shimmer and bend, like looking through hot air. Not solid. Not a creature you can clearly see. More like a moving mirage.

**The mystery:** Nobody has seen one at rest. Nobody knows where they go when not at the terminator. Underground? Into the light? Do they exist only at the boundary? The Hollowcall sings at night. The sunskirter runs at dawn. Between them, the planet has a rhythm the colonists slowly learn.

**Lore implication:** The previous civilization studied them. Artifact fragments reference "terminus entities" and "gradient parasites." They tried to trap one by creating artificial thermal edges. It worked. What they found inside changed everything.

### 2. The Ship: *The Sunskirter*

The colony ship that brought the colonists here. Named for the creatures observed from orbit before landing.

*"She was called the Sunskirter — a gravity-assist freighter that skirted close to stars for acceleration, using their gravity wells like slingshots. Not fast, not pretty, but she got close enough to stars that her hull glowed. The crew named her for the shapes they saw from orbit on approach — things racing along the terminator of the world below. They thought it was atmospheric distortion. It wasn't."*

**In-game presence:**
- The crash site is the colony origin. The wreck provides starting scrap metal, wire, a damaged battery.
- Ship logs are lore items (from LORE_AND_RESEARCH): "Day 1: Touchdown harder than planned. Drive core cracked. We're not leaving." ... "Day 15: Named the things we keep seeing at dawn. Sunskirters." ... "Day 47: Found something in the ruins. It looks like it was built to catch one."
- The ship's name appears on the main menu: **RAYWORLD — *The Sunskirter***
- A game subtitle that doubles as the colony's origin story.

### 3. The Phenomenon: Sunskirting

What the locals (if there were any) would call the optical effect at dawn/dusk on this planet. The atmosphere has a high particulate content (alien dust, mineral-rich air) that creates an unusually sharp and colorful terminator. Dawn and dusk aren't gradual — they're a bright ribbon of prismatic light that sweeps across the surface in minutes.

**Visual in-game:** The existing dawn/dusk lighting could be sharpened — instead of a smooth fade, a bright band of golden-pink light sweeps east-to-west during dusk, west-to-east during dawn. The band is where the sunskirter creatures appear. The band itself is beautiful and brief.

**Gameplay hook:** During sunskirting, visibility is momentarily *enhanced* in the terminator band (the light is incredibly sharp and directional). Colonists in the band can see further than normal. But colonists looking INTO the band from the dark side are blinded. Dawn raids from the east → attackers silhouetted in the sunskirting light, visible from far away. Dusk raids from the west → same advantage for defenders facing west.

### The Layered Name

```
The planet has a phenomenon:   sunskirting (the sharp terminator)
The phenomenon has a creature: sunskirters (that ride the terminator)
The crew named their ship:     The Sunskirter (after what they saw from orbit)
The ship crashed:              becoming the colony's origin
The colonists see the creatures: twice daily, a reminder of what brought them here
```

One word, woven through the world at every level.

---

## Connections

| Idea | Connects to |
|------|------------|
| Thumper → creature luring | Creature system, DN-010 burrows |
| Thumper → seismic survey | DN-010 discovery layer, metal detector |
| Thumper → long-range comms | Future: multi-settlement, trade |
| Surveillance drone → fog of war | Existing fog system |
| Surveillance drone → night vision | Thermal sim |
| Killer drone → combat | DN-011 bullet system, DN-012 wounds |
| Drone tech gate → artifacts | DN-010 discovery, LORE_AND_RESEARCH |
| Drone ethics → lore | LORE_AND_RESEARCH dangerous knowledge |
| Sunskirter creature → terminator | Day/night cycle, thermal sim |
| Sunskirter creature → lure via fire | Campfire creates artificial thermal edge |
| Sunskirter ship → origin story | Main menu, starting resources, lore items |
| Sunskirter phenomenon → combat | Dawn/dusk visibility bands, raid timing |
| Sunskirter carapace → crafting | Rarest material, requires trapping one |
| Sunskirter lore → previous civ | DN-010 artifacts, "they tried to catch one" |

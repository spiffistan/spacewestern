# Alien Fauna

Nocturnal and ambient creatures native to the frontier planet. These are NOT earth animals — the colonists crashed here; nothing is familiar. Earth livestock (chickens, cattle, dogs) may arrive later via supply drops or trade caravans as **imported domestics**, creating a clean narrative split: domestic earth animals are resources you manage, alien fauna is the world pushing back.

## Design Principles

**Familiar behavior, unfamiliar form.** Players understand "thing that stalks and pounces" or "swarm that eats your stuff" without a bestiary. The alien-ness is in the *how* — what it looks like, what it sounds like, how it interacts with the physics systems.

**The sound IS the creature.** You hear them long before you see them. The GPU sound propagation means their calls reflect off walls, diffract through doorways, muffle behind closed doors. Players develop emotional associations with sounds: "that clicking means something bad is close." Each creature's identity lives in its acoustic signature.

**Creatures interact with unique systems.** Every colony sim has "wolf attacks colonist." Only Rayworld has creatures that respond to heat (thermal buffer), gas composition (dye texture), sound pressure (sound sim), and light (raytracer/fog of war). Alien fauna should be designed around these hooks — they justify the physics.

**Night is their domain.** Most alien fauna is nocturnal or crepuscular. Daylight suppresses them. This makes the day/night cycle a survival mechanic, not just a lighting change. Darkness isn't empty — it's populated.

---

## Creature Roster

### Duskweaver

**Role:** Pack scavenger (the "coyote" niche — cowardly alone, dangerous in numbers)

**Description:** Spindly, multi-legged things, waist-high, that move in groups of 3-7. Thin limbs, quick lateral movement, unsettling gait.

**Behavior:**
- Emerge at dusk, retreat at dawn
- Scavenge unguarded food stockpiles (crates, berry bushes)
- Individually cowardly — flee from torchlight and groups of 2+ colonists
- In packs of 4+, will attack a lone colonist in the dark
- Avoid enclosed spaces; they operate in the open perimeter

**Sound sim interaction:**
Produce a rhythmic chittering that *synchronizes across the pack*. When scattered (probing, circling), the clicks are out of phase — a scattered ticking from multiple directions. When they lock onto a target, the clicks phase-lock into unison — a single coordinated pulse. The sound sim carries this physically: the player can gauge pack cohesion by listening. Synchronized chittering from one direction = imminent charge.

**Sound design:** Rhythmic clicking/ratcheting, like a mechanical winder. Fast tempo = agitated. Synchronized = attacking. Each individual is a separate sound source in the sim.

**Gameplay moment:** You hear scattered clicking outside the walls at night. It's just duskweavers circling — happens every few nights. Then the clicks synchronize. Something's wrong. You check the stockpile — the gate was left open.

---

### Thermogast

**Role:** Large solo threat (the "bear" niche — rare, devastating, drawn to your infrastructure)

**Description:** Massive, slow-moving creature with dense plating. Bioluminescent vents along its flanks that glow faintly with absorbed heat. Quadrupedal, heavy.

**Behavior:**
- Drawn to heat sources via the thermal simulation — reads `block_temps` buffer
- A roaring fireplace, active kiln, or smelter attracts it from across the map
- Not aggressive by nature — it wants warmth, not combat
- Will walk *through* structures to reach heat. Smashes walls, doors, furniture.
- Insulating your buildings (existing thermal conductivity) reduces your thermal signature
- An uninsulated colony with a roaring fire on a cold night is ringing a dinner bell
- Rare spawn — one every few nights at most, only in cold weather
- Can be deterred by extinguishing heat sources (costly) or by building double-insulated walls

**Sound sim interaction:**
Deep, resonant droning — almost subsonic. Felt through the ground before heard through air. The sound sim's low-frequency propagation makes this *rumble through walls*. Colonists inside hear a vibration in the structure before they hear the creature. Increases in volume as it approaches a heat source.

**Sound design:** Subsonic hum layered with a grinding/scraping overtone. Think: slowed-down whale song crossed with tectonic rumble. The bioluminescent vents hiss faintly when it absorbs heat.

**Gameplay moment:** Winter night, cold snap event. Your colony is huddled around fireplaces. A low rumble starts — barely audible. The walls vibrate. The thermogast is coming for your kiln. Do you extinguish the fires and freeze, or let it come and fight it?

---

### Glintcrawler

**Role:** Ambush hazard (the "rattlesnake" niche — step on it and you're in trouble)

**Description:** Small, low to the ground, segmented body with reflective carapace. Hides in tall grass, rocky terrain, and rubble. Nearly invisible in the raytracer among terrain clutter.

**Behavior:**
- Attracted to vibration — detects footsteps through the sound sim
- When a colonist walks within 2 tiles, produces a sharp warning sound
- If the colonist continues moving toward it, it strikes — venomous bite
- Venom requires the Doc (medical skill) to treat; untreated = escalating health damage
- Nests in uncleared terrain; clearing brush around the colony is a practical defense
- Stepping on a nest (3-5 clustered) is very bad — multiple strikes
- Diurnal and nocturnal — a constant hazard, not just a night thing
- More common in rocky/gravel terrain, near mine entrances

**Sound sim interaction:**
Sharp electric crackling, like static discharge. A brief, distinctive warning sound injected into the sound sim at the creature's position. Propagates physically — a colonist near a wall might hear a glintcrawler on the other side. The warning is short-range (3-4 tiles) and high-frequency — doesn't travel far, doesn't penetrate thick walls. You have to be close to hear it.

**Sound design:** Electric arc / static crackle. Brief (0.5s), sharp, unmistakable once learned. The "freeze and back away" sound.

**Gameplay moment:** Your miner is walking to the quarry at dawn. A sharp crackle from the grass ahead. They stop. The experienced player pulls them back — a nest. The new player walks forward and learns the hard way.

---

### Hollowcall

**Role:** Ambient dread (the "owl" niche — atmospheric, never seen, defines the sound of night)

**Description:** Never actually seen. Lives in deep forest canopy or underground cave systems. No one knows what it looks like. It's a sound, not a creature, as far as gameplay goes.

**Behavior:**
- Produces a long, mournful, almost musical tone at night
- Completely harmless — no attack, no interaction
- Colonists who hear it get a small stress bump ("Heard the Hollowcall: -2 mood, 1 day")
- The call bounces off terrain features in the sound sim, so it seems to come from everywhere
- New players will spend hours trying to find the source — there is nothing to find
- Multiple Hollowcalls on a map create overlapping resonances — eerie harmonics
- More frequent during fog, rain, and cold snaps
- Rare variant: sometimes the call seems to respond to colony sounds (gunshots, alarm bells) — unclear if this is real or coincidental

**Sound sim interaction:**
A single sustained tone injected at a distant, hidden source point. The sound sim carries it across the full map — reflecting off hills, diffracting around forests, muffling through walls. The physical propagation creates natural echo and delay effects. A colonist indoors hears a faint, muffled version. A colonist outdoors hears it clearly with directional cues — but the direction shifts as the sound bounces off terrain.

**Sound design:** Low, resonant, whale-like call — a single tone that rises, holds, and slowly falls over 3-5 seconds. Eerie, not aggressive. Organic but wrong — like a voice trying to sing a note it can't quite find. The signature sound of a Rayworld night.

**Gameplay moment:** First night on the planet. Colony is quiet. The fire crackles. Then the Hollowcall rises from somewhere in the dark, bouncing off the hills. The colonists exchange glances. They're not alone, and they don't know what's out there.

---

### Mistmaw

**Role:** Apex predator (the "cougar" niche — silent, lethal, the reason you don't walk alone at night)

**Description:** Medium-large quadruped with matte-dark skin that absorbs light. Nearly invisible in the raytracer at night (very low albedo material). Lean, low to the ground, built for ambush.

**Behavior:**
- Hunts alone, completely silent — **no sound injection into the sound sim**
- Camouflaged at night; barely visible even with a torch at medium range
- The only warning is its breath: it exhales CO₂ at an elevated rate
- The fluid sim carries that CO₂ plume — a faint signature in the dye texture
- A colonist with a CO₂ sensor (late-game tech) or who checks the gas overlay has a chance
- Avoids groups of 3+ colonists — only attacks lone individuals
- Avoids strong light (floodlights, torches at close range)
- Attacks from behind; the colonist's limited vision arc (from COMBAT.md) is its weapon
- Very rare — one on the map at a time, doesn't always appear

**Fluid sim interaction:**
The mistmaw's position is betrayed by an elevated CO₂ plume in the dye texture. The existing fluid advection carries this signature downwind. An observant player watching the gas overlay might notice a CO₂ bloom moving through the forest with no fire or colonist nearby. The creature literally leaves a chemical trail in your simulation.

**Sound design:** Almost nothing. A faint exhalation — barely above ambient noise floor. The sound design IS the silence. When ambient sounds (borers humming, wind, distant hollowcall) suddenly have a gap in them — something is stalking nearby. The absence of sound is the cue.

**Gameplay moment:** Your scout is returning alone from an expedition, torchless (it burned out). The night is noisy — borers, wind, distant hollowcall. Then a pocket of silence. The experienced player pauses the game and checks the gas overlay. There's a CO₂ bloom 4 tiles behind the scout. They switch to sprint. The new player doesn't notice, and the first sign is the attack.

---

### Borer Swarm

**Role:** Ambient + structural threat (the "insects/bats" niche — atmospheric, occasional nuisance)

**Description:** Clouds of tiny flying things that emerge from underground fissures at dusk. Individually tiny — rendered as particle-like noise in the raytracer, not individual sprites. Bioluminescent at low intensity (faint greenish shimmer in clouds).

**Behavior:**
- Emerge from underground at dusk, return at dawn
- Attracted to light sources (floodlights, torches, lamps)
- Swarm around light — a visual effect in the raytracer, a faint buzzing in the sound sim
- Mostly atmosphere; landing on a colonist causes mild annoyance ("Borer swarm: -1 mood")
- If dense enough near wooden structures, they bore in and cause structural damage over time
- Floodlights on the perimeter attract borers AWAY from buildings — a defensive use of light
- Interior lights attract them TOWARD your walls — placement of light sources matters
- Smoke repels them — a fire near the colony edge pushes swarms away (fluid sim interaction)

**Fluid sim interaction:**
Borers avoid high smoke density. A campfire or smudge pot (craftable) at the colony perimeter creates a smoke barrier in the fluid sim that repels the swarm. Wind direction matters — if the smoke blows inward, the borers come from the other side. This turns the fluid sim into a pest control tool.

**Sound design:** High-pitched humming/buzzing that rises and falls with swarm density. Not a single source but a diffuse area effect in the sound sim. Like tinnitus at the edge of hearing — uncomfortable but not alarming. A dense swarm near your ear is louder and more grating.

**Gameplay moment:** Summer evening. The borers rise from the cracks near the mine. They drift toward the colony's perimeter floodlights — good, that keeps them off the buildings. Then the wind shifts. The campfire smoke blows east. The borers come from the west now, drawn to the workbench lamp. By morning, the workbench's support beam has bore-holes.

---

## Sound Sourcing

Since these are alien creatures, nothing sounds "wrong" — there's no reference for what a Thermogast *should* sound like. This is creative freedom.

### Approach 1: Freesound.org CC0 + Processing (Best for base layers)

Source real-world sounds and make them alien through processing:
- **Duskweavers:** Ratchet mechanisms, mechanical clicks, insect recordings → pitch-shift, layer, rhythmize
- **Thermogast:** Whale song + tectonic rumble + industrial drone → slow down, add sub-bass
- **Glintcrawler:** Electric arc recordings, static discharge, tesla coil → trim to short bursts
- **Hollowcall:** Whale song + wind in pipes + bowed metal → pitch and reverb processing
- **Mistmaw:** Near-silence. Faint breath recording, barely above noise floor
- **Borers:** Bee/wasp swarm recordings → pitch up, thin out, add shimmer

Filter by CC0 license on Freesound — these sounds are public domain, no attribution required, irrevocable. Safe for commercial use.

**Search tips:**
- Filter: License → Creative Commons 0
- Search: "field recording" + source sound for natural texture
- Prefer mono recordings (the sound sim handles spatialization)
- Avoid anything with pre-baked reverb or background music
- Short, clean samples (1-5 seconds) — the sim handles propagation and environment

### Approach 2: ElevenLabs AI Generation (Best for alien character)

AI-generated sounds are ideal for creatures that shouldn't sound like anything on Earth. Prompt examples:
- "Rhythmic chittering of insect-like creatures, clicking mandibles synchronizing into a unified pulse, organic and unsettling"
- "Deep subsonic humming of a massive creature, resonant vibration, almost below hearing, grinding overtone"
- "Sharp electric crackling, brief static discharge from a small alien creature, 0.5 seconds"
- "Long mournful alien call, whale-like but wrong, single tone rising and falling over 4 seconds, eerie and lonely"
- "Dense swarm of tiny flying creatures, high-pitched buzzing shimmer, rising and falling"

Paid plans ($5+/month) give royalty-free commercial use. Free tier requires attribution to elevenlabs.io. Generate multiple variations and pick the best.

### Approach 3: Hybrid (Recommended)

Use Freesound CC0 recordings as raw material base (organic texture), ElevenLabs to generate supplementary alien layers, then mix and process in Audacity (free). This gives you grounded organic texture + otherworldly character. Each creature's final sound is 2-3 layers blended together.

### Approach 4: Field Recording (Norway bonus)

Nordic forests and coastline provide great source material. Phone recordings of:
- Wind through pine trees → alien atmosphere
- Ice cracking on lakes → Glintcrawler warning base
- Distant fog horns → Hollowcall base
- Gravel underfoot → processing into creature footsteps

Pitch-shift and layer to taste. Gives Rayworld a distinctive sonic character rooted in real-world texture that no library sound can match.

---

## Implementation Architecture

Creatures require minimal new infrastructure. They plug into existing systems:

### Data Model

```toml
# creatures.toml (data-driven, like blocks.toml and items.toml)

[[creature]]
id = "duskweaver"
name = "Duskweaver"
sprite_id = 0
size = "medium"            # small, medium, large — affects sprite, collision, health
health = 30
speed = 3.5                # tiles/sec
damage = 8
nocturnal = true
pack_size = [3, 7]         # min, max group size
flee_light_radius = 6.0    # tiles — flees from light sources within this radius
flee_group_size = 2        # flees if this many colonists are nearby
sound_id = "duskweaver_chitter"
sound_amplitude = 0.4
sound_frequency = "high"
attracted_to = []
repelled_by = ["light", "groups"]
loot = []                  # drops nothing — scavenger, not prey (initially)

[[creature]]
id = "thermogast"
name = "Thermogast"
sprite_id = 1
size = "large"
health = 200
speed = 1.0
damage = 40
nocturnal = true
pack_size = [1, 1]
flee_light_radius = 0.0    # not afraid of light
flee_group_size = 0        # not afraid of groups
sound_id = "thermogast_drone"
sound_amplitude = 0.8
sound_frequency = "subsonic"
attracted_to = ["heat"]    # reads block_temps buffer
repelled_by = []
loot = ["thermogast_plate"] # armor crafting material — late game
```

### Behavior State Machine

Each creature runs a simple FSM:

```
         ┌──────────┐
    ┌───→│  IDLE     │───── day + nocturnal? ────→ DESPAWN
    │    └────┬─────┘
    │         │ detects target (food, heat, prey)
    │    ┌────▼─────┐
    │    │  STALK    │───── target lost ──────────→ IDLE
    │    └────┬─────┘
    │         │ in range
    │    ┌────▼─────┐
    │    │  ATTACK   │───── target dead/fled ────→ IDLE
    │    └────┬─────┘
    │         │ health < 30%
    │    ┌────▼─────┐
    └────│  FLEE     │───── safe distance ───────→ IDLE
         └──────────┘
```

Special behaviors per creature type override this base FSM:
- **Duskweaver:** STALK includes synchronization phase; pack coordination before ATTACK
- **Thermogast:** STALK is just "walk toward hottest tile"; ATTACK is "smash obstacle in path"
- **Glintcrawler:** No STALK; goes directly IDLE → warn → ATTACK if colonist doesn't retreat
- **Hollowcall:** Only has IDLE (emit sound) and DESPAWN. No interaction.
- **Mistmaw:** STALK is silent (no sound source); ATTACK is instant high-damage ambush
- **Borers:** Swarm behavior — no individual FSM, area-effect at spawn point

### System Integration

| Existing System | Creature Hook |
|----------------|---------------|
| **Sound sim** (`sound.wgsl`) | Each creature is a sound source at its grid position. Amplitude/frequency from `creatures.toml`. Duskweaver sync is multiple sources phase-locking. |
| **Thermal sim** (`thermal.wgsl`) | Thermogast reads `block_temps` buffer to pathfind toward heat. Simple: each tick, move toward adjacent tile with highest temperature. |
| **Fluid sim** (`fluid.wgsl`) | Mistmaw writes elevated CO₂ into dye texture at its position. Borers avoid tiles with high smoke density (dye.r). |
| **Fog of war** (`fog.rs`) | Creatures are hidden in unexplored/dark areas. Mistmaw has extra low visibility (material albedo near zero). |
| **Pathfinding** (`pleb.rs`) | Creatures use existing A* with inverted/modified cost weights. Thermogast: low cost = high temp. Duskweaver: high cost = light. |
| **Day/night** (`time.rs`) | Nocturnal creatures spawn at dusk (existing time system), despawn at dawn. Spawn points at map edges or underground. |
| **Combat** (future) | Creatures are valid combat targets. DDA bullet trace works on them. Health/damage from `creatures.toml`. |

### What Needs to Be Built

1. **`creatures.toml`** — Data file defining creature types (follows blocks.toml pattern)
2. **`Creature` struct** — Simplified Pleb: position, velocity, health, behavior state, creature_type_id
3. **`creature_defs.rs`** — Registry loader, like `block_defs.rs` and `item_defs.rs`
4. **Creature tick in `simulation.rs`** — FSM update, movement, spawn/despawn logic
5. **Creature sprites** — Entries in the sprite atlas (one per creature, maybe 2-3 animation frames)
6. **Sound injection** — Each creature writes to sound sim buffer at its position (existing pattern from pleb footsteps)
7. **Spawn manager** — Controls creature population: time of day, biome, map region, rarity

### What Does NOT Need to Be Built

- No new rendering pipeline (creatures are sprites, same as plebs/trees)
- No new physics (existing A*, DDA, collision)
- No new fluid sim code (just write to existing dye texture / read existing buffers)
- No new sound system (creatures are sound sources, same as blocks/plebs)
- No new AI framework (FSM is simpler than pleb utility AI)

---

## Encounter Design

### The Quiet Night (Tutorial/atmosphere)
Hollowcall sounds in the distance. Borers shimmer around the perimeter floodlights. Nothing attacks. The player learns: nights are alive, but not always dangerous. Builds tension for when something *does* come.

### The Scavenger Run (Early game)
Duskweavers circle the colony at night, chittering. If the player left a stockpile unguarded (no walls, no light), they steal food. The player learns: secure your food, light your perimeter. Low stakes, clear lesson.

### The Cold Visitor (Mid game)
Cold snap event + thermogast. The colony needs fires to survive, but fires attract the thermogast. The player faces a dilemma: freeze or fight. Insulated walls reduce the thermal signature — a reward for investing in better construction.

### The Silent Hunter (Mid-late game)
A colonist goes missing on the way back from the mine at night. No sound, no warning. The player finds the body. Next time, they check the gas overlay. A CO₂ bloom is tracking another colonist. They draft a squad with torches. The mistmaw retreats from the light. The player learns: buddy system, torches, perimeter lighting.

### The Perfect Storm (Late game)
Cold snap + wind shift + borer emergence + thermogast attracted by emergency fires + duskweavers emboldened by chaos. Every creature type interacting with every system simultaneously. The colony's preparation (insulation, lighting, stockpile security, cleared terrain, buddy system) determines whether this is manageable or catastrophic.

---

## Future: Earth Domestics

When imported livestock is added, the split is:

| Category | Examples | Source | Relationship |
|----------|----------|--------|-------------|
| **Alien fauna** | Duskweaver, Thermogast, etc. | Native to planet | Threat / atmosphere |
| **Earth domestics** | Chickens, cattle, dogs, horses | Imported via trade/drops | Resource / companion |

Earth animals are familiar, manageable, useful. Alien fauna is unknown, dangerous, atmospheric. The two categories never overlap. A dog barking at an approaching duskweaver pack — the familiar warning you about the alien — is the intended emotional dynamic.

Dogs as early warning (bark propagates through sound sim), chickens as food source (duskweavers target them), cattle as trade goods. These are the domestication layer on top of the wild alien frontier.

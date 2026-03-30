# Emergent Physics Gameplay

Ideas that exploit the simulation stack — fluid dynamics, wave-equation sound, per-pixel thermal, and raytraced lighting — to create gameplay that is impossible in traditional tile-based colony sims. These aren't new systems to build; they're new ways to *surface* what the simulation already does.

## Guiding Principle

Most of these ideas require tuning parameters and adding checks, not building new engines. Fire whirls emerge from vorticity confinement. Condensation emerges from thermal + fluid coupling. Room resonance emerges from the wave equation. The simulation generates gameplay by existing. The work is in making it legible to the player.

---

## Acoustic Ecology — The World Has a Soundscape

Every tile produces ambient sound: wind through grass, water lapping at pond edges, trees creaking, fire crackling, machinery humming. All injected as constant low-amplitude sources into the sound sim. Every location on the map has a unique acoustic character that emerges from the physics. Standing near the waterfall is loud. The deep forest is quiet except for hollowcalls. Inside the base, the kiln hums and the fan whirs.

**Gameplay hook: Noise tolerance.** Colonists have a noise sensitivity need. Living next to the smelter is stressful. A bedroom far from the workshop is restful. The player thinks about *acoustic zoning* — industrial area here, sleeping quarters there, thick walls between them. The thin wall system with variable thickness becomes a sound insulation mechanic.

A colony that ignores acoustic zoning has stressed colonists and doesn't understand why. A colony that separates noisy industry from quiet living spaces has happier people — and the reason is physically visible in the sound overlay.

**What it costs:** Near zero. Sound sources per block type (defined in blocks.toml), a noise tolerance stat on plebs, mood buff/debuff from sampled sound amplitude at sleeping/working position.

---

## Pressure Events — The Fluid Sim as Weapon

A sealed room with a fire builds pressure (hot gas expands). Open the door fast → pressure blast through the corridor, pushes smoke, knocks things over, affects the sound sim. The "door opening pressure release" already exists in CONTEXT.md. Take it further:

**Pressure trap.** Seal a room. Fill it with heat (or methane from the extended gas system). Open the door toward an enemy breach point. The fluid sim does the rest — a pressure wave blasts through the corridor. Pair with combat breaching: enemies kick down a door into your pressure trap. The physics does the damage.

**Reverse: enemies weaponize YOUR infrastructure.** Raiders seal your rooms and pump toxic gas through captured pipe inlets. Your ventilation system becomes a vulnerability. The pipe network that keeps you alive can be turned against you if you lose perimeter control.

**Vacuum rooms.** A room with all air pumped out (via pipe system) — opening the door creates a violent inrush. Used defensively or as an industrial process (vacuum drying, preservation).

---

## Echoes of the Crash — The Wreck as Degrading System

The starting ship wreck isn't a loot box. It's a *degrading system*. Functioning subsystems that slowly fail:

- Power cell provides electricity for the first 10 days, then dies
- Water recycler works until day 20
- Medical bay has dwindling supplies
- Structural hull provides shelter but corrodes (thermal + fluid: rain seeps in, insulation fails)

The early game is a ticking clock of the wreck failing. Each failure is a crisis that forces the player to build a replacement: power cell dies → need a generator. Water recycler stops → need a well. Hull breaches → need walls. The wreck gives a *grace period that expires*, with each expiration teaching a lesson about what to build.

This is PHILOSOPHY.md's "scarcity as teacher" in mechanical form. The wreck doesn't dump resources on you — it gives you time, and then takes it away.

---

## Wind as Strategic Force — The Weathervane Colony

Wind direction shifts with weather and seasons. It determines:

- Where smoke from fires goes (fluid sim)
- Where sound carries (sound sim)
- Which side of buildings stays warm (thermal sim + wind chill)
- Where scent travels (future, see Scent section below)
- Where to place industry vs. living quarters
- Where windmills generate power most efficiently

A **wind rose overlay** showing prevailing wind patterns becomes one of the most-consulted tools. Players orient their entire colony based on wind — industrial downwind, living quarters upwind, defensive walls on the windward side.

The colony that ignores wind builds a colony that smells, overheats on one side, freezes on the other, and chokes on smoke every time the wind shifts. The colony that respects wind builds something elegant. The wind is already simulated — this is about making the player *care* about a force that's already there.

---

## Echo Location — Mapping by Sound

A colonist shouts into a cave entrance. The sound wave propagates through the cave system, reflects off walls, returns. The delay and pattern of echoes reveal the cave's shape — partially filling in fog of war for underground areas without anyone going in.

**Gameplay:** Before sending miners into a new tunnel, "ping" it with a shout or bell. The sound sim physically propagates the ping. Quick echoes = wall nearby. Slow echoes = large open space. No echo in one direction = long passage or surface opening. Imperfect information from the simulation — a rough shape, not a full map. Enough to know "big cavern down there" or "dead end" without risking a colonist.

A craftable **sounder** device injects a calibrated pulse and displays the echo pattern as a UI overlay — low-tech radar from the wave equation.

---

## Smoke Signals as Real Communication

The fluid sim makes smoke signals a *physical communication system*. A signal fire produces a smoke column. The player shutters it (cover/uncover) to create puffs — short and long.

The smoke is physically simulated: wind blows it sideways, rain suppresses it, terrain blocks line-of-sight. A signal from the watchtower to a distant outpost only works if the wind isn't too strong, there's no rain, and line-of-sight is clear. The recipient needs a colonist watching. The smoke takes time to form and dissipate — a natural bandwidth limit from the physics.

For outpost gameplay: real inter-settlement communication with physical constraints. "We sent the signal but the wind shifted — they never saw it." Emergent storytelling from the fluid sim.

---

## Condensation and Dew — Thermodynamics as Water Source

Warm moist air hitting a cold surface condenses into water. The thermal sim tracks surface temperatures. The fluid sim advects water vapor (H₂O in gas texture 2, planned). When they meet: condensation.

**Arid biome gameplay:** No rivers. Colonists survive by building moisture traps — cold surfaces (stone walls on the night-facing side of a hill) that collect dew from humid air. The thermal sim determines which surfaces are cold enough. The fluid sim determines where humidity is. The combination determines where water appears each morning.

A desert colony that engineers its thermal profile to maximize condensation — stone walls oriented to cool at night, channels to collect runoff, cisterns to store it — is doing real engineering with the simulation. No other game has water generation from thermodynamics.

---

## Fire Whirls and Emergent Fire Weather

The fluid sim has vorticity confinement. Fire injects heat and velocity. Large fires in reality create their own weather — fire whirls, pyrocumulus, self-sustaining wind patterns.

If a fire gets large enough, vorticity confinement + buoyancy + heat injection should naturally produce vortical structures. A forest fire doesn't just spread tile-to-tile — it creates a convection column that pulls air from all directions (visible in the velocity overlay), potentially spawning fire whirls that move erratically.

The fluid sim does this *for free* if the parameters are tuned. No coding fire whirls — tuning vorticity confinement until they emerge.

**Scale changes behavior.** A campfire is manageable. A building fire is serious. A forest fire becomes its own weather system that's nearly impossible to stop. The scale of the fire changes the physics, not just the damage.

---

## Acoustic Camouflage — White Noise as Defense

The sound sim means enemies hear the colony. Mask the acoustic signature: a waterfall near the colony provides natural white noise. A craftable noise generator (waterwheel, steam vent, deliberately loud machine) creates a sound barrier.

Colonists working behind the noise barrier can craft, mine, and talk without sound propagating to enemies. But they also can't hear approaching threats through the barrier. Tradeoff: acoustic concealment vs. acoustic awareness.

The sound sim handles this physically — high-amplitude noise sources mask lower-amplitude sounds by wave interference. No masking code needed — the wave equation does it.

A colony next to a waterfall has a natural advantage. A colony on a quiet plain has to engineer its own noise. Site selection matters in a new way.

---

## Glass, Lenses, and Light Manipulation

Glass blocks (type 5) already exist. Per-pixel raytracing already exists. Extend: curved glass (crafted from sand at the kiln) refracts light.

- **Solar igniter:** Magnifying lens focuses sunlight to a point — starts fires without matches
- **Lighthouse lens:** Amplifies a lamp's range dramatically in one direction
- **Greenhouse:** Glass roof panels trap heat via actual greenhouse effect (solar energy enters through glass, infrared radiation doesn't escape). The thermal sim keeps the interior warm. Crops grow year-round.
- **Mirror arrays:** Angled glass redirects light into dark corridors without electricity
- **Heliograph:** Mirror-based communication between outposts — faster than smoke signals, requires clear sky and line-of-sight

The greenhouse is the killer app here. Physically motivated year-round farming from the thermal simulation + glass material properties. A survival essential in cold biomes.

---

## Resonance and Standing Waves

The wave equation in enclosed spaces produces standing waves. Different room sizes have different resonant frequencies. Certain sounds at the right pitch make a room *vibrate*.

- A hollowcall at exactly the right pitch makes a specific room resonate — objects rattle, colonists feel unsettled
- Changing the room's dimensions detunes the resonance
- A room deliberately tuned to amplify the alarm bell creates a colony-wide alert amplifier
- A concert hall shape provides the best mood buffs from the musician colonist
- A whispering gallery (long curved corridor) carries whispers from end to end
- A square room creates acoustic dead spots

This is emergent from the wave equation and room geometry. Players who understand acoustics exploit it. Players who don't just notice "this room feels weird when the hollowcall sounds." The simulation teaches through experience.

---

## Scent as Fluid — A Dye Channel for Animals

Animals navigate by scent. The fluid sim already advects multiple density fields. Add a "scent" channel:

- Colonists emit scent (low amplitude)
- Cooking emits stronger scent
- Blood emits scent (attracts predators)
- Butchering leaves a scent beacon on the colonist
- Washing in water reduces scent
- Aromatic herbs at the perimeter confuse predators

The fluid sim advects scent downwind — the same wind that blows smoke. Duskweavers track scent plumes to food. The mistmaw follows a colonist's scent trail. Upwind is safe, downwind is danger. All physical, all directional, all driven by the existing velocity field.

Connects directly to ALIEN_FAUNA.md creature behaviors and the wind-as-strategic-force concept above.

---

## The Underground as Acoustic Horror

Underground, there are no ambient wind sounds, no borers, no hollowcalls. Just dripping water, your own footsteps, and... something else. A distant rhythmic thumping from deeper levels.

The sound sim carries it through stone with heavy attenuation and low-pass filtering (stone conducts low frequencies better than high). The deeper you mine, the more you hear things you can't explain. Reflections off tunnel walls create confusing echoes — footsteps that might be yours bouncing back, or might be something following you.

PHILOSOPHY.md: "The world is the teacher. Failure is the curriculum." The player learns what's safe by listening. A quiet tunnel is safe. A tunnel where you hear your own echoes coming back wrong is not.

---

## Thermal Footprints and Tracking

A colonist walking across cold ground leaves faint thermal footprints (briefly warmer tiles). A thermogast leaves very warm footprints. At night, thermal goggles (crafted, late-game) let a colonist *track creatures and people by heat signatures*.

- A trail of fading warm spots leading into the forest — something came through here
- A mistmaw's thermal trail visible in the morning overlay — it was circling the colony
- Enemy scout footprints lead back to the raider camp — follow them
- A colonist who's been missing — their thermal trail shows where they went

Forensic gameplay — reading the physics simulation for information. No minimap, no quest markers. Just thermal signatures in the sim.

---

## Acoustic Triggers for Automation

The pipe system has valves. The power grid has switches. Wire triggers to the sound sim: a loud sound (gunshot, explosion, alarm bell) triggers a mechanism.

**Alarm bell cascade:** Bell at the watchtower, wired to perimeter floodlights via acoustic trigger. Bell rings → sound wave propagates → hits trigger → lights turn on. The distance the sound travels means there's a real delay — far lights activate later than near ones. A cascading activation wave that you can *see and hear* propagating through the colony.

**Trap trigger:** Tripwire alarm bell triggers a valve that floods a corridor with pressurized gas. Or opens a gate that drops rocks. The trigger is acoustic — the sound sim determines when it fires based on physical propagation.

**Sound-activated doors:** A door that opens when it hears the right signal (specific bell frequency). Crude voice-activated security from the wave equation.

---

## Corpse Ecology — Death Feeds the Sim

A decomposing body produces methane and CO₂ (gas texture channels). In a sealed room, this builds up — reopening a crypt produces a toxic gas burst. In the open, the gas plume attracts scavengers (duskweavers follow the scent).

- Mass grave after a raid = environmental hazard — methane accumulation could ignite from a careless torch
- Proper graveyard with ventilation (pipe system) manages off-gassing safely
- Impromptu body dump in an unused room = ticking time bomb of toxic gas
- Composting corpses (compost block, type 13) converts them to fertilizer, producing CO₂ but not methane

Dark, physically motivated, creates real infrastructure decisions. Where do you put the dead? How do you manage the gas? The answer involves pipes, fluid sim, and fans.

---

## Sympathetic Fire — The Colony as Kindling Network

Fire spreads not just by contact but by *radiation*. A burning building heats nearby buildings via the thermal sim until they reach ignition temperature (250°C from the spec) and spontaneously combust. No direct flame contact needed.

**Building spacing matters.** Packed-tight buildings are a firetrap — one fire radiates enough heat to chain-ignite neighbors. Widely spaced buildings with stone firebreaks are safer. Fire gaps become a real architectural concern, just like in real frontier towns (which burned down constantly).

The thermal sim does the work. The only new code: `if block_temp > ignition_temp && material.flammable → fire`. Cascading ignition emerges from the physics.

---

## Light Pollution and Astronomy

The colony glows at night. Light scatters into the atmosphere (the volumetric rendering already does atmospheric scattering). A heavily-lit colony creates a dome of light pollution visible from far away.

**Stealth vs. safety tension:** Redskulls spot your colony from across the map at night — the brighter you are, the farther away. A colony running blackout protocols (kill exterior lights, shutter windows) is harder to raid — but colonists can't see, and alien fauna moves in. Direct tension: lights-on = safe from predators, exposed to raiders. Lights-off = hidden from raiders, exposed to predators.

**Star-gazing:** A colonist with a telescope (crafted, placed on high ground) observes the night sky from dark locations only. Provides a mood buff and over time reveals navigational information for expeditions. A colony drowning in its own light can't see the stars. The frontier astronomy angle fits the space western — stranded people trying to figure out where they are.

---

## Thermal Inversion and Ground Fog

Temperature inversions (cold air trapped under warm air) produce fog and trap pollutants at ground level. The thermal sim + fluid sim can model this. On calm, cold mornings, a temperature inversion forms — cold dense air settles in low areas, trapping smoke and gas near the ground.

**Visually:** The raytracer renders trapped smoke as ground-level fog filling valleys while hilltops are clear. **Gameplay:** Valley colonies choke on their own smoke during inversions. Toxic gas pools in low areas instead of dispersing. Hilltop colonies have clear air but are exposed to wind and visible from far away.

Elevation matters for air quality, not just defense. A smelter in the valley fills it with smoke during inversions. A smelter on the ridge blows its smoke away. Environmental engineering from the physics.

---

## The Pipe Organ — Sound Sim Showcase

A craftable musical instrument using the pipe system and sound sim. Metal pipes of different lengths, connected to an air supply (bellows or fan). Each pipe produces a specific frequency based on physical length — calculated from the wave equation, not a lookup table.

The sound physically propagates through the colony. Colonists hear it at different volumes based on distance and wall attenuation. Mood buff scales with sound amplitude at the listener's position (sampled from the actual sound sim). A well-placed organ in a central hall with good acoustics buffs the whole colony. A poorly placed one in a thick-walled corner is barely heard.

Pure technical showcase. A YouTube clip of someone playing a pipe organ in Rayworld, with sound physically propagating through their base in real-time, would go viral in the indie dev community. The moment people understand that the sound is *simulated, not sampled*.

---

## Erosion and Terrain Deformation

Rain erodes terrain over time. Water flows downhill (fluid sim applied to terrain), carrying soil. Exposed hillsides without vegetation erode faster. Compacted paths erode into gullies during heavy rain. River banks slowly widen.

Over a game-year, terrain changes shape:
- A hillside colony sees the slope erode if trees are cleared (roots anchored the soil)
- A road cut into a hillside becomes a gulley after rainy season
- A dam (player-built wall across drainage) creates a pond that slowly silts up
- Deforested areas lose topsoil, reducing farm fertility

Long-term geological gameplay. Short-term: weather is an event. Long-term: weather reshapes the map. The colony manages drainage, plants windbreaks, terraces hillsides, or accepts that the land will change under it. The map doesn't just record history — it *evolves*.

---

## Heat Shimmer as Visual Deception

On hot days, the ground radiates heat. The raytracer distorts vision through hot air — distant objects waver and blur. Mirage effects at long range.

**Combat application:** A sniper shooting across a hot open field has reduced accuracy — the target wavers in the heat shimmer. Fighting at dawn or dusk (cool air, no shimmer) is tactically advantageous. The thermal sim determines where shimmer is worst (hot ground, low wind). The raytracer renders distortion. The combat system samples it for accuracy penalties.

A visual and mechanical effect that emerges purely from the thermal sim interacting with the raytracer. Rewards players who think about *when* and *where* to fight.

---

## Acoustic Archaeology — Listening to the Past

Underground ruins could have acoustic properties revealing layout. An ancient hall has different reverb than a tight corridor. A sealed chamber resonates distinctly when you knock on the adjacent wall.

**Mining by ear.** Instead of randomly digging, systematically tap and listen. The sound sim physically models the response:
- Hollow space behind a wall = different echo than solid rock
- Large buried chamber resonates when tapped
- Underground river produces faint rushing audible through rock
- A sealed vault reflects sound sharply — a "bright" echo vs. stone's dull thud

The archaeology system becomes an acoustic puzzle: tap, listen, map, dig. The information is physics-based but imperfect — you're interpreting sound, not seeing X-rays. Expert players learn to read signatures. This makes exploration *skill-based* in a way unique to the simulation.

---

## Sympathetic Vibration — Buildings That Sing

Certain room geometries resonate at specific frequencies. When a sound source (hollowcall, alarm bell, explosion) matches a room's resonant frequency, the room amplifies it:

- Objects rattle, colonists inside feel unsettled (mood debuff)
- Changing room dimensions detunes the resonance — the fix is architectural
- Deliberately designed: a "war room" that amplifies alarm bell frequency for colony-wide alerts
- An interrogation room with focused acoustics
- A music hall tuned for maximum mood buff from instruments

Emergent from the wave equation — specific resonances fall out of the math. Not coded, discovered. Players who understand acoustics exploit it.

---

## The Map as Forensic Record

PHILOSOPHY.md talks about "the map as memory." Take it literally. The simulation accumulates physical traces:

- Terrain compaction from walking (already exists)
- Scorch marks from fire (permanent terrain modification)
- Blood stains from combat (fade over days)
- Collapsed structures leave rubble piles
- Erosion from water flow
- Tree stumps that slowly rot
- Chemical discoloration from gas exposure
- Thermal scarring on stone from sustained heat

After a game-year, a new player arriving at the colony can *read its history from the ground*. The worn path from mine to smelter. The scorched clearing where the year-1 fire happened. The rubble of the wall the thermogast smashed. The graveyard on the hill.

Every mark is a consequence of system interaction — fire left a scar, feet wore a path, combat left blood, rain eroded the hillside. None scripted. All simulation residue.

---

## Priority and Effort Estimates

Ideas roughly ordered by impact-to-effort ratio:

**Near-free (parameter tuning, data changes):**
- Acoustic ecology / noise tolerance (add sound amplitude per block type, mood check)
- Sympathetic fire / radiant ignition (temperature threshold check on flammable blocks)
- Light pollution visibility (raiders spot colony based on aggregate light output)
- Heat shimmer accuracy penalty (sample thermal at target position)

**Low effort (small features using existing systems):**
- Wind as strategic force (wind rose overlay, colonist awareness of wind direction)
- Thermal footprints (write temp at pleb position, decay over time)
- Corpse ecology (decomposition gas injection into dye texture)
- Pressure events (amplify existing door-pressure-release mechanic)
- Scent channel (new dye channel, creature attraction logic)
- Fire whirls (tune vorticity confinement for large fire scenarios)
- Map as forensic record (terrain modification on events)

**Medium effort (new mechanics, some new code):**
- Echo location / sounder device (sound pulse injection + echo analysis UI)
- Acoustic triggers (sound amplitude threshold → switch/valve activation)
- Smoke signals (shutter mechanic on signal fires)
- Greenhouse / glass refraction (glass thermal properties, crop growth in warm enclosures)
- Crash wreck degradation (timed subsystem failures, tutorial pacing)
- Thermal inversion / ground fog (temperature layering in fluid sim)

**High effort (significant new features, but spectacular payoff):**
- Pipe organ (physical frequency generation, pipe length → pitch mapping)
- Acoustic archaeology (echo analysis for underground mapping)
- Erosion / terrain deformation (fluid-on-terrain simulation layer)
- Condensation / dew collection (thermal-fluid coupling for water generation)
- Resonant rooms (wave equation analysis for room acoustics feedback)
- Acoustic camouflage (white noise masking — may emerge naturally from sim)

---

## Connection to Other Docs

| This Idea | Connects To |
|-----------|-------------|
| Scent channel, thermal footprints | `ALIEN_FAUNA.md` — creature tracking and attraction |
| Acoustic ecology, pipe organ, resonance | `GAMEPLAY_SYSTEMS.md` — music and morale |
| Pressure events, acoustic triggers | `COMBAT.md` — traps, breaching, defense |
| Crash wreck degradation | `PHILOSOPHY.md` — scarcity as teacher |
| Wind strategy, thermal inversion | `GAMEPLAY_SYSTEMS.md` — weather events |
| Smoke signals, light pollution | `DEEPER_SYSTEMS.md` — communication systems |
| Greenhouse, condensation | `DEEPER_SYSTEMS.md` — food and water |
| Map as forensic record | `PHILOSOPHY.md` — the map as memory |
| Erosion, fire whirls | `GAMEPLAY_SYSTEMS.md` — seasons, fire as tool |
| Corpse ecology | `DEEPER_SYSTEMS.md` — medicine and disease |
| Echo location, acoustic archaeology | `MULTI_LEVEL.md` — underground exploration |

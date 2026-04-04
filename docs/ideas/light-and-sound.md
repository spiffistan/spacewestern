# Light and Sound as Gameplay

How the raytracer and the sound sim go beyond rendering to become gameplay systems. Light reveals the present. Sound reveals the hidden. Together they create a world with depth no colony sim has attempted.

## The Existing Foundation

The raytracer (6,292 lines of WGSL) already does: directional sunlight with per-pixel shadows, lightmap-based point lights (torches, lamps, campfires), glass blocks with color tinting and transparency, fire overlay as emissive, rain with streaks and splashes, temporal accumulation for smooth shadows, fog of war, fluid dye overlay, cloud dimming, and rain haze. The sun has direction, elevation, intensity, and color. Materials have per-block properties.

The sound sim (wave equation) already does: pressure waves from sources propagating through air, reflection off walls, absorption by materials, diffraction through openings, attenuation with distance. Sound is physical — it bends around corners, passes through thin walls muffled, and dies in sealed rooms.

Both systems operate on the same 512×512 grid, every frame. The question isn't "can we do more?" — it's "what does more MEAN for gameplay?"

---

## Part 1: Light as Gameplay

### Light the Player Already Reads

The raytracer communicates state through light without any UI:

- A building with power has warm artificial lighting. One that lost power goes dark. You see the power outage.
- Fire throws flickering amber light visible across the map. An instant alarm.
- Night falls and darkness advances. The colony's lit perimeter is visible — and so is everything beyond it.
- Glass windows glow from interior light — a warm beacon in the dark landscape.

These are already present or trivially achievable. The following ideas push further.

### Colored Light as Material Language

Right now, light sources are mostly white or warm amber. What if different materials and sources produced distinctively colored light, and those colors carried meaning?

**Campfire light:** Warm amber-orange. The familiar color of safety. Colonists near campfire light get a subconscious comfort signal. This is the "home" color.

**Torch light:** Slightly redder than campfire — more orange-red. Portable warmth. A colonist carrying a torch leaves a moving pool of warm light that says "someone is here."

**Electric light (powered lamps):** Cool white. Brighter, steadier, more coverage. The color of technological capability. A colony that transitions from torchlight to electric light LOOKS different — cooler, more ordered, more modern. The transition from amber to white is the visual story of progress.

**Kiln / smelter light:** Deep red-orange, flickering with intensity. Industrial heat. You see the smelter's glow through the workshop window from across the colony. It pulses with the work rhythm.

**Ancient infrastructure light:** Something wrong. A faint blue-green that doesn't match any human light source. When a colonist breaks into an ancient chamber, the first thing they (and the player) notice is the light is WRONG. The color temperature doesn't exist in the human-built colony. It's unsettling. And beautiful. As the colony integrates alien power (DN-019 alien tech track), patches of this alien light start appearing in human buildings — a visual marker of how deeply you've connected to the ancient systems.

**Moonlight:** Cool silver-blue. Bright enough on clear nights to see terrain but not detail. Duskweavers are visible as dark silhouettes against moonlit ground. Full moon nights are safer not because the creatures are different, but because you can SEE them. New moon nights are the most dangerous — pure darkness beyond torchlight. The moon cycle creates a second rhythm within the day/night cycle.

**Bioluminescence:** Some alien organisms (the glintcrawler's warning crackle, certain fungi in caves) produce faint light. These are tiny points of cold light in the darkness — beautiful and alarming. A glintcrawler nest in the grass at night is a cluster of faint blue-white sparks. The player learns: those pretty lights are where the stingers are.

### Light Defines Territory

At night, light defines the colony's boundary more powerfully than any wall. The perimeter of torchlight and lamp-light IS the colony. Beyond it: darkness, sound, and whatever lives there.

**The lightline:** The border between lit colony and dark wilderness. Duskweavers won't cross the lightline (their flee radius from alien-fauna.md is calibrated against light intensity at the tile level). A power outage that kills the perimeter lights is an invasion event — the lightline retreats to the campfires and torches, and the duskweavers advance.

**Light as defense:** Floodlights (existing block BT_FLOODLIGHT) pointed outward create a broad, bright perimeter. The power cost is significant — defending with light burns watts that could run the smelter. But it's silent defense that works every night without ammunition. The colony chooses: weapons or watts? Walls or light?

**Darkness as concealment:** During a raid, turning off your lights makes you harder to find — but also makes YOU blind. A colony that kills its lights and fights by moonlight has an advantage only if its colonists know the terrain (Competent+ in local knowledge, DN-019). The darkness is bilateral.

### Glass and Light Architecture

Glass blocks already exist. But glass could be much more than transparent walls:

**Colored glass.** Craftable with dyes (chemistry domain, DN-019). Red glass, blue glass, green glass. Light passing through colored glass becomes tinted — the interior of a room with stained glass windows takes on that color. A chapel (Chapel Hill aesthetic) with colored glass windows fills with warm multicolored light in the morning. A greenhouse with green-tinted glass filters light for optimal plant growth (actual gameplay bonus — the thermal sim traps heat and the light filter reduces certain spoilage).

**Lenses.** Ground glass lenses (glass making domain, Expert level) that focus light. A lighthouse lens that concentrates a lamp into a directional beam — illuminating a long narrow cone across the landscape. Useful for: watchtower spotlight (scan for threats), signaling (other settlements see the beam), crop enhancement (concentrated light on greenhouses in winter).

**Prisms.** A glass prism block that splits white light into spectral components. Purely aesthetic — but a room with a prism in the window fills with rainbow refractions on clear mornings. A mood buff for the room: "Beautiful light: +2 mood." The preacher builds a chapel with prisms. The colony has a place that's beautiful for no practical reason. PHILOSOPHY.md's "the things you can't build" — some beauty comes from engineering light through geometry.

**Mirrors.** Polished metal surface (forging domain, Competent+). Reflects light directionally. Uses: redirect sunlight into underground rooms (real mining technique — called a "light shaft"). A mirror at the surface bounces daylight down a shaft into an underground chamber. The underground farm (char-cap from food-and-survival.md) gets natural light without power. A mirror in a dark corridor bounces torchlight around a corner. Mirrors make light a manipulable resource, not just an ambient condition.

### Shadows as Information

The raytracer already casts per-pixel shadows from directional sunlight. Shadows carry information:

**Time of day from shadow angle.** The sun moves. Shadows rotate. A colonist can tell time by shadow direction — long shadows at dawn/dusk, short shadows at noon. This is already implicit in the renderer. Making it explicit: a sundial block that colonists check (diegetic clock from DN-020).

**Movement detection.** A shadow moving across a sunlit patch of ground means something is moving above — a creature on a wall, a colonist on a roof, debris falling. The player who watches shadows in sunlit areas detects movement before seeing the source directly. In a raid, a raider's shadow precedes them around a corner.

**Shadow as threat gauge.** At night, a duskweaver approaching the lightline casts a long shadow AWAY from the light source — the shadow enters the lit area before the creature does. The player sees the shadow first. A moment of warning. The length of the shadow tells you how close the creature is.

**Cloud shadows.** Large-scale shadows from cloud cover sweeping across the landscape (weather system from world-and-seasons.md). Not per-pixel raytraced — a large soft shadow region that moves with the wind. Dramatic and atmospheric. A cloud shadow darkening the colony for a minute creates a momentary chill — both thermal (slight temperature dip) and emotional.

### The Visual Moments

Some light scenarios that the raytracer should specifically nail because they ARE the game's identity:

**The first night.** The crash wreck still has emergency lighting — a faint, failing red blink. The campfire you built is the only warm light in a vast dark landscape. The duskweavers are clicks and rustles in the darkness just beyond the firelight. The light circle is tiny. The dark is enormous. This IS the game's opening statement.

**Winter firelight on snow.** The colony at night in deep winter. Blue-white moonlight on snow. Golden-amber firelight spilling through glass windows onto the snow outside, creating warm rectangles in the blue world. Inside: colonists gathered in the saloon, warm light, chat bubbles, laughter (sound sim). Outside: silence, cold, the occasional distant click of a duskweaver. The contrast between warm interior and cold exterior, rendered per-pixel by the raytracer, is the game's emotional signature.

**Dawn after a storm.** The storm has passed. First light breaks through clearing clouds — golden rays hitting wet surfaces. Everything gleams. Puddles reflect the sky (water surface rendering). Damaged structures are visible for the first time — what the storm took from you. But the light says: it's over. You survived. The relief is visual.

**The smelter at night.** The workshop with the smelter running: deep red-orange glow pulsing through the window. Sparks visible as tiny emissive particles. The mechanic's silhouette against the glow. From across the colony, you see the heartbeat of industry — the smelter light tells you production is happening without checking any panel.

**The alien chamber.** A colonist breaks through a wall underground and enters an ancient space. The first thing: the light is WRONG. A cold blue-green luminescence from surfaces that shouldn't glow. No flicker — perfectly steady, unlike every human light source. The colonist's warm torch casts amber light that conflicts with the alien blue-green. Two color temperatures fighting in the same space. The visual dissonance IS the discovery — something was here before you, and it's still running.

**The signal fire.** Your colony lights a signal fire on the hilltop to attract a trader. The fire is enormous — a pillar of flame visible for kilometers (on the world map, other settlements see it). The smoke column rises through the fluid sim. At night, the fire turns the hilltop into a beacon, casting long shadows radiating outward. Everything in the colony is lit from above for once. It's dramatic, beautiful, and dangerous — the Redskulls can see it too.

### Seasonal Light Identity

Each season should have a distinct light palette baked into the sun color, ambient color, and sky rendering (extending world-and-seasons.md visual mood):

| Season | Sun Color | Ambient | Sky Character | Shadow Quality |
|--------|-----------|---------|---------------|---------------|
| Spring | Warm gold, lower intensity | Cool blue-green | High, scattered clouds, soft | Soft, diffuse — frequent overcast |
| Summer | White-hot, high intensity | Warm yellow | Big, empty, intense | Sharp, high-contrast — clear skies |
| Autumn | Deep amber, low angle | Warm amber-gray | Low, layered, dramatic | Long, golden — sun sits low all day |
| Winter | Cold white-blue, low intensity | Blue-gray | Pale, close, featureless | Faint when overcast, blue-tinted when clear |

The sun_color_r/g/b and ambient_r/g/b uniforms already exist in the camera struct. Seasonal modulation is a simple interpolation on the CPU side before upload. The raytracer just renders whatever color it's given — the seasonal mood comes from the data, not from shader changes.

**The golden hour.** In autumn, the sun angle is so low that everything is perpetually in golden hour for the first and last hour of daylight. The entire colony glows amber. This should be the most visually beautiful season — and the most emotionally bittersweet, because winter follows.

**The blue hour.** In winter, the brief twilight after sunset and before sunrise is a deep saturated blue. The snow reflects it. The colony's warm interior lights are the only non-blue color in the world. This contrast — ten minutes of blue twilight with golden windows — should be a screenshot moment every day.

---

## Part 2: Sound as Gameplay

### What the Sound Sim Already Does

The wave equation propagates pressure from sources through the grid. Sound reflects off walls, diffracts through openings, attenuates with distance, and is absorbed by materials. This is already physically correct enough for:

- A conversation in one room being audible (muffled) through the wall in the next room
- An explosion being loud nearby and quiet far away
- Sound bending around corners through open doorways
- Sealed rooms being silent

### Sound as Archaeology

The most distinctive gameplay idea: using the sound simulation to discover what's underground.

**The principle.** Different subsurface structures have different acoustic responses. Solid rock reflects sound sharply. A hollow chamber resonates. A fluid-filled pipe conducts sound along its length. Loose rubble absorbs sound. A colonist who taps the ground with a hammer and listens to the response can detect underground features without digging.

**How it works mechanically.** A colonist with the prospecting or construction domain (DN-019) at Familiar+ can perform the "acoustic survey" activity. They walk to a tile, strike it (generates a short impulse in the sound sim), and listen (the sim propagates the impulse, and the return signal is analyzed).

What the game checks:
- Sound velocity through the subsurface at that tile (different for solid rock, hollow void, wet gravel, ancient metal)
- Reflection strength (a strong echo means a hard boundary — a chamber wall, a metal pipe, a void)
- Resonance frequency (a large chamber rings at a lower frequency than a small one)
- Attenuation pattern (sound that disappears fast = absorptive material. Sound that lingers = reflective chamber)

**What the player sees.** When the acoustic survey completes, the tile gets a marker:
- "Solid" — nothing unusual below. Dense material, no voids.
- "Hollow" — an underground void detected. Could be a natural cave, a collapsed tunnel, or an ancient chamber. Size estimate (small/medium/large) from resonance frequency.
- "Metallic return" — something dense and reflective, not natural rock. Ancient infrastructure.
- "Fluid" — liquid detected below. Underground stream, ancient pipe with fluid, or flooded chamber.
- "Uncertain" — the signal was complex or degraded. Needs a higher-skill colonist or a different approach.

**Skill level affects accuracy.** A Familiar geologist gets coarse readings — "something down there, maybe." A Competent one gets reliable void detection. An Expert reads the acoustic signature like sheet music: "Large chamber, roughly 8 meters below, metal walls, partially flooded on the east side." A Master can estimate what's IN the chamber from secondary reflections.

**The sound overlay.** During an acoustic survey, the sound sim overlay (already available as a debug view) lights up with the propagating impulse. The player literally watches the sound wave expand from the impact point, bounce off subsurface features, and return. A hollow chamber shows as a bright persistent ringing. A solid mass shows as a dark shadow. The visualization is real — it's the actual wave equation solving the actual geometry. No fake overlay. The physics IS the display.

**This is genuinely novel.** No other game uses a physically-simulated sound field for prospecting. The technology exists in Rayworld because the sound sim already runs on every tile every frame. The acoustic survey is a QUERY against the simulation, not a separate system. The infrastructure cost is zero — it's already there.

### The Hollowcall as Standing Wave

The hollowcall (alien-fauna.md) isn't a creature. It's a signal — an ancient beacon broadcasting from buried infrastructure (the-human-layer.md). What if the sound sim reveals this?

The hollowcall creates a **standing wave pattern** across the terrain. The wave equation naturally produces standing waves from a continuous source in a bounded space. The ancient infrastructure IS the bounded space — pipes, chambers, resonant cavities. The hollowcall's pattern, visible in the sound overlay, maps the shape of the underground infrastructure.

A colonist who studies the hollowcall's propagation pattern (xenobiology or alien tech domain, Expert level) can read the underground geometry from the surface. "The signal is strongest here and here — there's a corridor running northeast." "The resonance changes near the ridge — something large is beneath it." The hollowcall is doing the acoustic survey for you, constantly, if you know how to listen.

This reframes the hollowcall from ambient horror to scientific resource. The creature that terrified your colonists on night one becomes the key to mapping the ancient infrastructure by year two. Familiarity doesn't just reduce fear — it unlocks understanding. The knowledge gradient (DN-019) in action: Unaware (terror) → Aware (unease) → Familiar (curiosity) → Competent (analysis) → Expert (cartography from sound).

### Echo Mapping

An extension of acoustic surveying: mapping CAVES and RUINS from their entrances using echo.

A colonist at a cave mouth generates a loud impulse (shout, horn blast, fired weapon). The sound enters the cave, bounces off every surface, and returns as a complex echo. An Expert listener reads the echo:

- A single sharp echo = a flat wall at X distance
- A diffuse echo = a rough or branching space
- Multiple distinct echoes = multiple chambers or corridors
- No echo = the space continues beyond hearing range (deep cave, long corridor)
- A metallic ring in the echo = artificial surfaces (ancient infrastructure)

The player doesn't need to send colonists into unlit, dangerous underground spaces to get a rough map. They stand at the entrance and LISTEN. The information is coarse — it doesn't replace exploration — but it tells you whether the cave is worth entering, how big it is, and whether it contains something artificial.

This connects to combat: a colonist cornered in a building can listen for the echo of approaching footsteps to estimate how many enemies are in the next room and how far away they are. Sound as tactical intelligence.

### Noise Discipline

The colony makes sound. A lot of it. The smelter, the saw horse, the construction, the conversations, the livestock, the machinery. All of these are sound sources that propagate through the sound sim out into the wilderness.

**Creatures respond to colony noise.** The colony's acoustic footprint attracts attention:
- Duskweavers are drawn to investigate unfamiliar sounds (but flee from bright light — the tension between sound attraction and light repulsion)
- Thermogasts don't care about sound (they track heat)
- Borers are attracted to vibrations in the ground (heavy machinery = borer problem)
- Redskulls can hear your colony from a distance. A louder colony is found sooner.

**Noise reduction as strategy.** Placing loud workshops away from the perimeter. Building sound-absorbing walls (insulated walls from the build menu). Scheduling loud work during daytime when creatures are dormant. A colony that manages its acoustic footprint is harder to find and attracts fewer threats. A colony that runs the smelter 24/7 with no insulation is a beacon.

**The silent colony.** In extreme cases — preparing for a Redskull raid, hiding from a thermogast pack — the colony goes SILENT. All work stops. Fires are dampened (reducing crackle). Colonists whisper. The sound sim shows the colony's acoustic signature shrinking to nearly nothing. The darkness outside is silent. The tension is physical. This is the Rayworld equivalent of "turning off the lights and holding your breath."

### Acoustic Camouflage

The inverse of noise discipline: using sound deliberately.

**White noise generators.** A waterfall, a windmill, a noisy machine placed at the perimeter creates a constant ambient sound that masks the colony's specific noises. Creatures hear "waterfall" instead of "people working." The masking works because the sound sim is physical — the white noise fills the same frequency space as the colony's noise, reducing the signal-to-noise ratio for anything listening from outside.

**Decoy sounds.** A bell or noisemaker placed away from the colony draws attention. Duskweavers investigate the sound source instead of the colony. A colonist with a hunting horn leads creatures away from a work party. Sound as a lure — physical, simulated, interactive.

**The signal fire horn.** When lighting a signal fire to attract traders, the colony also blows a horn — a loud, low-frequency sound that carries across the world map. Settlements in range hear it. Traders orient toward it. But so does everything else. The horn is a calculated risk: help is coming, but so is attention.

### Music as Physical Phenomenon

Not background music. A colonist who can play an instrument (found in wreckage, carved from wood, traded) creates actual sound in the sound sim.

**The saloon musician.** Some evenings, a colonist with the right skill plays music. The sound propagates from the saloon. Colonists within range get a mood buff: "Hearing music: +3 mood." The range is physical — colonists in the next building hear it faintly through the wall. Colonists at the far workshop don't hear it at all. The saloon's position determines how much of the colony benefits. Build the saloon centrally → more people hear the music.

**Work songs.** A colonist singing while working boosts nearby workers' speed slightly (+5%). The song is a sound source. Other workers in range synchronize to it — a subtle boids-like coordination effect. Construction goes faster with a singer on site. The effect is physical, not abstract — if you build a wall between the singer and the workers, the buff fades because the sound is blocked.

**The instrument as artifact.** Instruments are rare. The first one might come from a wreck — a battered guitar, a dented horn. It's precious because it's irreplaceable until someone learns to carve one (woodworking Expert, specific knowledge). Losing the instrument to fire or theft silences the colony until a new one is made. A colony with music is measurably happier than one without. The instrument is one of the most valuable non-survival items in the game.

**Alien resonance.** The pipe organ idea from emergent-physics.md: the ancient infrastructure has resonant chambers. Wind passing through them produces sounds that are NOT human music — eerie, harmonic, vast. A colonist who hears them for the first time gets a stress bump. A colonist who's heard them many times finds them... not beautiful exactly, but significant. A colonist with the right knowledge (alien tech Familiar+) realizes the sounds encode information — the frequency shifts with the ancient system's activity. The planet is SINGING, and the song means something.

---

## Part 3: The Duality of Light and Sound

Light and sound are complementary senses. Each reveals what the other can't:

| Light | Sound |
|-------|-------|
| Shows the surface — what's visible | Reveals the hidden — what's behind walls, underground |
| Blocked completely by opaque walls | Leaks through walls, bends around corners |
| Instantaneous — you see everything at once | Propagates over time — you hear things arriving |
| Defines the colony's territory (lightline) | Defines the colony's footprint (noise radius) |
| Controlled by power (lamps, floodlights) | Controlled by behavior (silence, insulation) |
| Attracts creatures that flee it (duskweavers) | Attracts creatures that investigate it |
| Can be directed (mirrors, lenses) | Can be directed (architecture, resonance) |
| Carries beauty (seasons, glass, fire) | Carries emotion (music, laughter, grief) |
| The player's primary sense | The player's secondary sense — but the one that reveals what's truly hidden |

### The Night Illustrates the Duality

During the day, light dominates. You see everything. Sound is secondary — background ambience.

At night, the balance shifts. Light retreats to the colony's lamps and fires. Darkness covers 90% of the map. But sound doesn't stop — the sound sim still propagates from every source. At night, the player's awareness shifts from visual to auditory:

- You HEAR the duskweavers before you see them (clicks, chittering at the perimeter)
- You HEAR the thermogast's approach (a low vibration as it displaces air — the fluid sim)
- You HEAR a colonist in distress (a shout from the dark that propagates to the nearest listener)
- You HEAR the hollowcall change pitch (seasonal variation — what does it mean tonight?)

The night is when sound becomes the primary gameplay sense. The player who plays with headphones at night has a genuinely different experience from one who plays muted. This is the design opportunity no other colony sim has: **a game where listening is as important as looking.**

### Underground Illustrates the Duality

Underground spaces are the purest expression of the light-sound duality:

**Light underground** is precious and directional. A torch illuminates a cone. A mirror redirects sunlight down a shaft. Beyond the light: absolute darkness. The raytracer renders underground spaces as pools of warm light in total black. Every shadow is sharp. Every corner is a question mark.

**Sound underground** is rich and everywhere. Stone reflects sound efficiently. A footstep echoes. A hammer strike rings through corridors. A chamber's size and shape are readable from the echo. Ancient pipes conduct sound from distant sources — you hear the hum of alien machinery before you see it. The ancient infrastructure reveals itself through sound before light reaches it.

The colonist exploring underground carries a torch (light) and their ears (sound). The torch shows them what's immediately around them. Their ears tell them what's ahead, behind, above, below. The two senses together create a complete picture that neither provides alone. This is exploration that FEELS different from any other game — not because of special mechanics, but because the physics of light and sound in enclosed spaces naturally create this dual-sense experience.

---

## Part 4: Implementation Considerations

### Light Additions to raytrace.wgsl

Most light gameplay ideas require minimal shader changes:

**Colored light per source:** The lightmap already supports per-light color (the lightmap compute pass could encode RGB per light source instead of intensity-only). Materials already have color properties. The main raytrace shader already composites lightmap contribution — extending from scalar to RGB is a data-width change, not an architectural one.

**Moon cycle:** A uniform `moon_phase` (0.0–1.0) that modulates ambient night brightness and color. New moon = near-zero ambient. Full moon = soft silver-blue ambient. The shader already has ambient_r/g/b — moon phase modulates these on the CPU side.

**Mirrors:** A new block type that, in the lightmap compute pass, reflects incident light directionally. The lightmap already traces from sources — a mirror block redirects the trace. Moderate complexity but high payoff: mirrors enabling light shafts into underground rooms is both visually stunning and gameplay-meaningful.

**Colored glass tinting:** Glass blocks already have a color value in the rendering. Extending to affect transmitted light means the lightmap pass samples the glass color when light passes through. Light on the far side takes on the glass's tint. Small change, beautiful result.

**Bioluminescence:** Certain blocks (glintcrawler nest, alien fungi, ancient surfaces) emit very low-intensity light with specific color. Add to the existing emissive light system. No architectural change.

### Sound Additions to sound.wgsl / simulation.rs

**Acoustic survey:** The sound sim already propagates waves. The survey activity injects an impulse at a tile and reads the response after N ticks. The subsurface response could be modeled as: the tile's elevation data (already bound) determines material density below. Low elevation = more rock above a void. The acoustic response is computed from the elevation gradient and any underground feature flags (new data: a per-tile `subsurface_type` enum — solid, void, fluid, metal, ancient).

**Hollowcall standing wave analysis:** The hollowcall is already a persistent sound source. The standing wave pattern emerges naturally from the wave equation given the ancient infrastructure's geometry. If the underground features are encoded as boundary conditions in the sound sim (reflective walls = high impedance tiles), the standing wave pattern IS the map. An Expert colonist reading the sound overlay is doing legitimate physics.

**Colony noise radius:** Already implicit — the sum of all sound sources' propagation through the sim. Making it explicit: a "colony noise" metric computed by sampling sound amplitude at the map edges. Displayed as a subtle ring on the minimap showing how far the colony's sound carries. Quieter colony = smaller ring = fewer threats attracted.

---

## Connection to Other Systems

| System | Light & Sound Connection |
|--------|------------------------|
| **Creatures** (alien-fauna.md) | Duskweavers flee light, investigate sound. Thermogasts ignore both (track heat). Glintcrawlers produce bioluminescence. Hollowcall is a sound-sim phenomenon. Creature behavior emerges from physical light/sound interaction. |
| **Equipment** (DN-018) | Torch as light source. Horn and bell as sound sources. Spyglass extends visual range. Instruments as sound artifacts. |
| **Knowledge** (DN-019) | Acoustic survey skill. Hollowcall interpretation. Glass making for lenses/prisms. Understanding alien light and sound. |
| **UI** (DN-020) | Light IS the primary UI. Sound overlay as investigation tool. The diegetic bell tower. Music as mood system. |
| **Seasons** (world-and-seasons.md) | Sun color, angle, intensity per season. Day length defines light/dark ratio. Winter = more darkness = more sound-dependent gameplay. |
| **Combat** (DN-011) | Shadow-based movement detection. Noise discipline before raids. Acoustic intelligence (listening for enemy count). Gunshot sound propagation reveals shooter position. |
| **The Setting** (the-setting.md) | The Perdition broadcast as persistent sound. Ancient infrastructure light as alien color. Radio as sound-based narrative delivery. The signal fire as visible distress call. |
| **Exploration** (world-and-seasons.md) | Acoustic survey for prospecting. Echo mapping caves. The lightline defining the frontier between known and unknown. |
| **Building** | Sound-absorbing walls for noise discipline. Glass architecture for light manipulation. Mirror placement for underground illumination. Bell tower and lighthouse as communication infrastructure. |
| **The Ancient Layer** | Alien light (blue-green) as visual marker of contact with ancient systems. Alien resonance as sound-based information encoding. The hollowcall standing wave as underground cartography. |

---

## Summary

Light and sound are not rendering features. They're the game's twin sense systems — each revealing what the other can't, each carrying gameplay meaning beyond aesthetics.

**Light** defines the colony's territory, communicates state through color temperature (campfire amber = safety, electric white = progress, alien blue-green = the unknown), enables defense through the lightline, and becomes a manipulable resource through glass, lenses, and mirrors. The raytracer's per-pixel physically-correct lighting makes every light source a signal, every shadow a piece of information, and every season a different visual identity.

**Sound** reveals the hidden — underground voids through acoustic survey, creature positions through propagation, ancient infrastructure through standing wave analysis. Sound is the sense that works in the dark, through walls, beneath the surface. The wave-equation sound sim makes this physically correct: echoes are real geometry, standing waves map real structures, noise discipline has real spatial consequences.

**The duality** is most powerful at night and underground — the two situations where the game is most distinctive. At night, the player shifts from watching to listening. Underground, the torch shows what's near and the echo reveals what's far. No other game creates this dual-sense experience because no other game has both a per-pixel raytracer AND a physically-simulated sound field.

**The hollowcall** is the bridge between the two senses and between the two layers of the world. It's a sound that maps ancient geometry. It's a light (alien blue-green) that marks the boundary between human and alien. Understanding it is the knowledge system's deepest challenge and most rewarding payoff: the moment when the thing that terrified you becomes the key to understanding where you are.

The planet was here before you. It's been singing and glowing in the dark for millennia. Your job is to learn what it's saying.

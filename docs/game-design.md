# Game Design: Space Western Colony Sim

## Core Identity

This is not a colony sim with physics bolted on. This is a **physics sandbox where a colony emerges**. The physical simulation IS the gameplay — sound, light, heat, gas, and pressure are not abstracted but fully simulated, and every system creates emergent gameplay.

The setting is a **space western**: a harsh frontier world where crashed survivors, drifters, outlaws, and prospectors build a life on the edge of civilized space. Think Firefly meets RimWorld, but where the physics actually matter.

## What We Have That No One Else Does

### Real Fluid Dynamics
Smoke fills rooms. Temperature propagates through walls based on material conductivity. Wind blows through open doors. O2 depletes in sealed rooms with fire. This is Navier-Stokes, not tile-based gas rules.

### Physical Sound Propagation
Actual 2D wave equation. Sound reflects off walls, diffracts through doorways, creates interference patterns. A gunshot at one end of the map creates an expanding pressure wave that reaches colonists based on actual physics — blocked by walls, passing through open doors.

### Raytraced Lighting
Per-pixel shadows from a moving sun. Light bleeds through windows. Proximity glow from fireplaces. Dynamic day/night with real shadow casting from terrain elevation. Not "light level 7 per tile" but actual light transport.

### Terrain With Real Elevation
Hills cast shadows. Water table is lower on hilltops. Ambient occlusion from dawn/dusk ray traces. Pathfinding costs more uphill. Building requires flat ground.

## The Space Western Theme

### The Frontier
- You didn't plan this. You **crashed here**. Starting area is a wreck site with salvageable tech.
- Technology is **scarce and improvised**. You don't unlock tech tiers — you find busted equipment in wrecks and jury-rig it.
- The land is beautiful but hostile. Dust storms. Solar flares. Things that howl at night.
- Resources are finite and contested.

### The People
- Not workers with skill bars. **Characters with pasts.**
- A prospector who came for ore and got stranded. A deserter hiding from the military. A frontier doctor who drinks too much. A preacher with a shotgun.
- They have **opinions about each other**. Grudges. Loyalties. Romances.
- Around the campfire at night, they tell stories. Builds morale AND reveals lore.

### The Tension
- **Bounty hunters** come for your outlaw colonist.
- **Bandits** heard you found something valuable.
- A **marshal** shows up asking questions.
- **Moral choices**: wounded stranger at the gate. Help, rob, or ignore?
- Colony develops a **reputation** (lawful ↔ outlaw) that determines who visits, attacks, and trades.

## Physics As Gameplay

### Sound As Intelligence
- Enemies **hear** colonists through physical sound propagation.
- Gunshots alert nearby hostiles — suppressed weapons have lower dB.
- **Alarm bells** on watchtowers: ring one and the sound wave reaches colonists within earshot, physically simulated through walls and doors.
- **Stealth is real**: close the door and enemies can't hear crafting inside.
- Plebs hear sounds and can determine direction from the pressure gradient — physically correct directional hearing.

### Atmosphere As Threat
- Fire **consumes oxygen** in sealed rooms. Colonists suffocate if ventilation fails.
- **Toxic gas** from grenades or accidents spreads through ventilation ductwork.
- **Temperature** physically drains from buildings through walls based on conductivity.
- **Smoke signals**: burning green wood creates visible smoke — call for help or reveal position.

### Light As Cover
- Darkness is **real cover** at night. Unlit areas are genuinely dark in the raytrace.
- A sniper on a hilltop at dusk casts a long shadow that gives away position.
- **Floodlights** illuminate approaching enemies but advertise your location.
- Interior lights visible through windows at night — cozy but exposed.

### Explosions As Physics
- Grenades create **pressure waves** through the sound system that push gas, shatter glass, blow open doors.
- Shockwave damage proportional to dB at the target position.
- Sound-gas coupling means explosions physically move smoke and atmosphere.
- Structural damage from overpressure (future: structural integrity system).

## Differentiation

| Aspect | RimWorld / ONI | This Game |
|--------|---------------|-----------|
| Gas/Air | Grid tiles with O2 level | Navier-Stokes fluid sim |
| Sound | None / abstract alert | Wave equation with reflection and diffraction |
| Light | Light level per tile | Per-pixel raytraced shadows |
| Combat | Hit/miss RNG | Physical projectiles, pressure waves, dB damage |
| Temperature | Per-room average | Per-pixel heat propagation with material conductivity |
| Terrain | Flat grid | Elevation, hillshade, terrain AO, slope pathfinding |
| Theme | Generic sci-fi/frontier | Space western with reputation and moral choices |
| Characters | Skill bars + traits | Origins, relationships, moral alignment, stories |

## Character System

### Origins
Each colonist has a backstory explaining why they're on this frontier world:

| Origin | Arrival | Starting Bonus |
|--------|---------|----------------|
| Crash Survivor | Ship went down | +Construction, -Stress threshold |
| Prospector | Came for minerals, stranded | +Mining, resource detection |
| Exile | Banished from core worlds | +Combat, -Social |
| Drifter | Just passing through | +Speed, jack of all trades |
| Deserter | Military AWOL | +Combat, +Construction |
| Frontier Doc | Only medic for parsecs | +Medicine, -Combat |
| Preacher | Spreading the word | +Social, morale aura |
| Outlaw | Bounty somewhere | +Combat, +Night Owl |

### Traits (2-3 per character)

**Positive:**
- Hard Worker — 20% faster at tasks
- Night Owl — no mood penalty at night
- Green Thumb — crops 15% faster
- Iron Gut — eats anything without penalty
- Tough — 30% less damage
- Steady Hands — better aim, faster crafting
- Optimist — stress builds slower

**Negative:**
- Lazy — 15% slower, longer breaks
- Pyromaniac — mental break: lights fires
- Volatile — stress builds 25% faster
- Wimp — extra damage, panics in combat
- Insomniac — rest recovers slowly
- Loner — mood penalty near others
- Gourmand — eats twice as much

**Quirky:**
- Early Riser / Night Owl
- Fast Walker — 20% movement speed
- Scar Story — visible scar, +respect
- Hoarder — carries more, won't drop items

### Skills (improve with practice)
```
Construction — build speed, repair quality
Farming      — crop yield, planting speed
Combat       — accuracy, reload, melee damage
Cooking      — food quality, no poisoning
Medicine     — healing speed, surgery success
Mining       — dig speed, resource yield
Social       — trade prices, mood boost aura
Crafting     — recipe speed, item quality
```

### Western Nicknames
Generated from traits/appearance/origin:
- "Red" (red hair), "Slim" (thin), "Doc" (frontier doc)
- "Patches" (crash survivor), "Ghost" (loner), "Sparky" (pyromaniac)
- "Preacher", "Judge", "Cookie" (cook), "Ace" (marksman)

## Gameplay Systems (Future)

### The Radio
Build and power a radio tower to:
- Contact passing traders (schedule deliveries)
- Broadcast distress signal (brings help and attention)
- Intercept enemy communications
- Play music (physically audible via the sound system — morale boost in nearby rooms)

### The Saloon
Social building for evenings:
- Mood boost from socializing
- Rumors spread (learn about resources, threats)
- Conflicts can escalate (bar fights)
- Visitors gather here (traders, strangers)

### Prospecting
Send colonists to explore fog of war:
- Mineral deposits (iron, copper, rare metals)
- Old crash sites (salvageable tech)
- Water sources
- Other survivors (recruit or conflict)
- Environmental hazards

### Weather That Matters
The fluid + sound + terrain systems support:
- **Dust storms**: reduce visibility, clog pipes, erode walls (wind sim)
- **Thunderstorms**: lightning strikes, rain fills low terrain, thunder propagates through sound system
- **Heat waves**: temperature sim makes indoor cooling essential
- **Cold snaps**: heat drains faster, water freezes

### The Wanted System
Each colonist has a **notoriety** level (0-10):
- 0: Unknown — nobody comes looking
- 1-3: Person of interest — occasional bounty hunter
- 4-6: Wanted — regular pursuit, can't trade at honest posts
- 7-9: Notorious — military response
- 10: Legend — everyone knows your name

Colony reputation is the average. High notoriety attracts outlaws (recruitable but volatile). Low notoriety attracts settlers (stable but boring).

### Economy
- **Barter**: no currency, trade goods for goods
- **Salvage**: most valuable items are found, not made
- **Supply caravans**: arrive periodically, buy/sell based on reputation
- **Contraband**: profitable but risky
- **Crafting through experimentation**: combine materials, discover recipes

## Priority Order

### Near Term
1. Character system (origins, traits, skills, nicknames)
2. Sound-as-intelligence (enemies hear gunshots, alarm bells work physically)
3. Sprite rendering for characters (Blender pipeline, see sprite-pipeline.md)

### Medium Term
4. Reputation system (lawful ↔ outlaw)
5. Visitor/trader events
6. Radio tower
7. Prospecting/exploration

### Long Term
8. Structural integrity
9. Vehicles
10. Multiple biomes
11. Multiplayer?

## Art Direction

### Visual Style
- **Warm, dusty palette** — amber, ochre, sage green, weathered wood
- **Dynamic lighting is the star** — sunsets casting long shadows across the frontier, fireplaces glowing through windows at night, floodlights cutting through dust storms
- **Lived-in aesthetic** — nothing is clean or new. Patches, rust, improvisation
- **Nature is beautiful** — procedural terrain, rolling hills, scattered wildflowers. The world is worth protecting.

### Sprite Style (see sprite-pipeline.md)
- Flat albedo renders from Blender, game shader does all lighting
- 16-32px per tile, clean readable silhouettes
- Character customization through color tinting
- Accessories/traits visible as overlays (hats, scars, gear)

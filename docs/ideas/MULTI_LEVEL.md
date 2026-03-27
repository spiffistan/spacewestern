# Multi-Level: Depth as a Gradient

## Concept

The world extends downward through multiple underground levels. There's no artificial "bedrock" floor — depth is limited by escalating difficulty, cost, and danger. Each level deeper demands more infrastructure, better tools, and more colonist labor. The player stops digging when the cost exceeds the reward, not when the game says stop.

A struggling colony never goes past -1. A thriving one pushes to -3 for ore. Going to -5 is a late-game commitment that could drain an entire economy if something goes wrong. Depth is a measure of ambition and capability.

## Level Characteristics

### -1: Cellar (0-3m)

The first thing any colony digs. Easy, useful, low risk.

- **Material:** Dirt and clay. Dig with shovels.
- **Water:** Usually above the water table. Dry.
- **Air:** Fine with a stairway open to surface.
- **Temperature:** Cool and stable. Ideal for food storage.
- **Structure:** Self-supporting for small rooms. No pillars needed.
- **Resources:** Clay, roots, buried scrap.
- **Purpose:** Root cellar, storm shelter, basic storage, escape tunnel.
- **Every colony wants this.**

### -2: Basement (3-8m)

The first real engineering challenge. Worth it for materials and protection.

- **Material:** Packed earth, occasional stone. Need pickaxes.
- **Water:** Water table is close. Seepage in wet seasons. May need a pump.
- **Air:** Needs a ventilation shaft. Single stairway isn't enough for large spaces.
- **Temperature:** Warm. Stable but noticeably warmer than -1.
- **Structure:** Support pillars needed for rooms wider than 4-5 tiles.
- **Resources:** Stone (quarrying without surface pits), iron ore, clay deposits.
- **Purpose:** Workshop space, armory, expanded storage, protected living quarters.
- **Most colonies stop here.** It's comfortable and productive.

### -3: Mine (8-15m)

Serious mining territory. This is where "surviving" becomes "building something ambitious."

- **Material:** Mostly rock. Slow without good tools or blasting.
- **Water:** Water table is almost certainly above you. Constant pumping required. Power-dependent.
- **Air:** Bad. Need fan-driven ventilation with multiple shafts. CO2 accumulates in dead ends.
- **Temperature:** Hot. Colonists work slower, need water breaks.
- **Structure:** Cave-in risk is real. Supports every 3-4 tiles. Wide caverns are dangerous.
- **Resources:** Copper (electrical components), coal (fuel), tin. The crafting chain demands these.
- **Purpose:** Industrial mining. Resource extraction drives the colony's advancement.
- **The transition point.** Getting here means your colony has surplus labor, reliable power, and engineering capability.

### -4: Deep (15-25m)

Only for colonies with serious infrastructure and ambition.

- **Material:** Hard rock. Very slow without powered drilling or explosives.
- **Water:** Relentless flooding. Multiple pumps on dedicated power circuits. A single pump failure cascades.
- **Air:** Extremely poor. Need pressurized ventilation or multiple fan stages. Colonists can't work long shifts.
- **Temperature:** Dangerously hot. Need cooling infrastructure (pipe cold water down? ventilation from surface?).
- **Structure:** Support every 2-3 tiles. The weight above is immense. One miscalculation = catastrophic collapse.
- **Resources:** Silver (trade value, decoration), gems (luxury, morale boost), rare minerals.
- **Purpose:** Wealth extraction. Status. The colony is powerful enough to attempt this.
- **High reward, high risk.** A power outage here means flooding, suffocation, and potential total loss of the level.

### -5+: Abyss

Theoretically possible. Practically a gamble.

- **Material:** Dense rock. Crawling pace without advanced technology.
- **Water:** Essentially submerged without constant industrial pumping.
- **Air:** Requires engineered pressure differential. Natural ventilation can't reach this deep.
- **Temperature:** Extreme. Active cooling mandatory.
- **Structure:** Constant reinforcement. The tunnel walls close in without maintenance.
- **Resources:** Ancient ruins. Alien artifacts. Blueprint cards for technology that changes the game.
- **Purpose:** Discovery. The reason to go this deep isn't resources — it's answers. What's buried on this planet?
- **This is the endgame content.** Getting here means mastering every system simultaneously.

## The Escalation Curve

| Factor | -1 | -2 | -3 | -4 | -5 |
|--------|----|----|----|----|-----|
| **Dig speed** | Fast | Medium | Slow | Very slow | Crawling |
| **Tool needed** | Shovel | Pickaxe | Pick + blasting | Powered drill | Advanced drill |
| **Water table** | Above | At level | Below (flooding) | Deep flooding | Submerged |
| **Air quality** | Open stairway | Shaft needed | Fan required | Multiple fans | Pressurized |
| **Temperature** | Cool (15°C) | Warm (22°C) | Hot (32°C) | Very hot (42°C) | Extreme (55°C) |
| **Cave-in risk** | None | Wide rooms | Real threat | Constant | Extreme |
| **Support spacing** | None | 5+ tiles | 3-4 tiles | 2-3 tiles | Every tile |
| **Light** | Torch | Torch | Electric | Electric + backup | Redundant systems |
| **Pump demand** | None | Occasional | Constant (1 pump) | Heavy (2-3 pumps) | Industrial (4+ pumps) |
| **Labor efficiency** | 100% | 90% | 70% | 50% | 30% |
| **Walk time** | 2 flights | 4 flights | 6 flights | 8 flights | 10+ flights |

The curve is exponential, not linear. Each level deeper isn't "a bit harder" — it's a fundamentally different engineering challenge.

## What Drives the Player Deeper

### The Crafting Chain

The resource progression encourages depth:

```
Surface:  Wood, berries, fiber, rock → basic survival
-1:       Clay, scrap → pottery, early crafting
-2:       Stone, iron → tools, construction, weapons
-3:       Copper, coal, tin → electrical, advanced crafting
-4:       Silver, gems → trade, luxury, morale
-5:       Ancient tech → game-changing blueprints
```

You can survive on the surface forever. But to advance — better tools, electricity, trade goods — you need to go down.

### Events and Discovery

The card system connects naturally:

- **-2:** "Iron Vein Discovered" → rich deposit, worth building a proper mine
- **-3:** "Underground Stream" → natural water source (or flooding risk)
- **-3:** "Gas Pocket" → explosive danger, but natural gas = fuel source
- **-4:** "Collapse in the Deep Mine" → crisis event, rescue mission
- **-5:** "Ancient Chamber Breached" → discovery event, blueprint card, possibly danger

### Colonist Traits

The chargen system connects:

- **Prospector** backstory: mines faster, spots ore veins earlier
- **Claustrophobic** trait: stress builds underground
- **Mole** trait: works faster underground, stressed on surface
- **Steady Nerve** trait: no panic during cave-ins

## Infrastructure Requirements

### Ventilation Chain

```
-1: Open stairway to surface (passive)
-2: Ventilation shaft + passive draft (two openings create airflow)
-3: Fan on shaft (forced ventilation, needs power)
-4: Multiple fans + dedicated shafts (redundant ventilation)
-5: Pressurized system (sealed shafts + high-power fans)
```

The existing fan/pipe system handles this. Each level just needs more of it. The failure mode is the interesting part — fan breaks, air goes bad, colonists have minutes to evacuate.

### Drainage Chain

```
-1: No drainage needed (above water table)
-2: Occasional bucket brigade or small pump
-3: Constant liquid pump (needs power)
-4: Multiple pumps on separate power circuits
-5: Industrial pumping station (dedicated power source)
```

The existing liquid pipe/pump system handles this. The water table mechanic creates the pressure. Deeper = more water = more pumps = more power.

### Structural Support

```
-1: Dirt walls hold for small rooms
-2: Wooden supports for spans > 5 tiles
-3: Stone pillars every 3-4 tiles + wooden supports
-4: Reinforced pillars every 2-3 tiles
-5: Continuous support (arches? steel beams?)
```

New block types: mine support (wood), stone pillar, reinforced beam. Without them, cave-in probability increases over time. Not instant — it's a gradually worsening creaking sound, then dust falling, then collapse. Time to react if you're paying attention.

### Vertical Transport

Getting colonists and materials up and down:

```
Stairs: Slow, reliable, manual. Every colony has these.
Ladder: Faster for colonists, can't carry heavy items. Compact.
Elevator shaft: Powered, carries materials. Mid-game investment.
Mine cart track: Horizontal + inclined, bulk material transport. Late-game.
```

Walk time matters. At -4, a colonist walks down 8 flights of stairs, works for a bit, walks back up. Most of their time is commuting. An elevator shaft saves labor hours dramatically — but costs power and materials to build.

## The Fluid Sim Connection

Each level runs its own 2D fluid simulation. Levels are coupled at connection tiles:

### Air Coupling (at stairs, shafts, vents)

- **Temperature-driven vertical flow:** hot air rises through connections, cool air sinks
- **Fan-forced flow:** fans on shafts create directed airflow between levels
- **Chimney effect:** fire on a lower level → hot air rises → pulls fresh air in through other openings
- **Gas sinking:** CO2 is heavier, accumulates at the bottom. Natural gas rises.
- **Sealed sections:** close a trapdoor → air exchange stops → whatever's below is on its own

### Water Coupling (at stairs, shafts, flooded connections)

- **Gravity-driven:** water flows downward through any opening
- **Water table pressure:** deeper levels have higher water pressure against walls
- **Pump chains:** liquid pumps move water upward through pipes
- **Cascade flooding:** level -3 floods → water flows down stairs to -4 → both levels lost

### Emergent Scenarios

These arise naturally from the coupled systems:

**The Chimney:** Fire at -3 → hot air/smoke rises through stairway to -2, -1, surface. Colonists above see smoke coming up the stairs. Close the trapdoor to contain it — but now miners at -3 have no air.

**The Gas Pocket:** Mining at -3 breaks into a gas pocket. Gas flows upward through connected tunnels. Reaches the fireplace at -1. Explosion. The fire system + fluid sim + multi-level coupling create this without any special-case code.

**The Flood Cascade:** Power fails during storm → pumps at -3 stop → water rises → floods the -3 stairway → water pours down to -4 → -4 was your ore processing center → everything is underwater. Recovery takes days. Was the deep mine worth it?

**The Rescue Mission:** Cave-in at -4 traps two miners. Stairway is blocked by rubble. They have limited air (the fluid sim tracks this). You need to dig a rescue shaft from -3, which risks destabilizing more ceiling. The event card gives you options: dig carefully (slow, safe) or blast through (fast, might cause secondary collapse).

## User Interface

### Level Navigation

- **Page Down / Page Up:** step between discovered levels
- **Level indicator** in corner: shows all discovered levels with current highlighted
- **Double-click colonist** in the colonist bar: auto-navigate to their level
- **Ghost overlay:** viewing any level shows the level above as faint wall outlines

### Visual Feel Per Depth

| Level | Lighting | Background | Mood |
|-------|----------|------------|------|
| Surface | Sun + weather | Sky, clouds | Open, strategic |
| -1 | Stairway daylight + torches | Dark earth | Cozy, sheltered |
| -2 | Torches + lamps | Darker earth | Industrial, busy |
| -3 | Electric lights, darkness beyond | Rock, dampness | Tense, purposeful |
| -4 | Sparse electric, deep shadows | Dark rock, heat shimmer | Oppressive, risky |
| -5 | Alien glow? Ancient light sources? | Strange stone, ancient carvings | Mysterious, awe |

Each level has a distinct visual atmosphere. You know where you are by the feel, not just the UI indicator.

### Fog of War Underground

Vision radius decreases with depth (less ambient light to reflect):

| Level | With torch | Without | With electric |
|-------|-----------|---------|---------------|
| -1 | 8 tiles | 3 tiles | 10 tiles |
| -2 | 7 tiles | 2 tiles | 10 tiles |
| -3 | 6 tiles | 1 tile | 9 tiles |
| -4 | 5 tiles | 1 tile | 8 tiles |
| -5 | 4 tiles | 0 tiles | 7 tiles |

Exploring -5 without electric light is nearly impossible. The darkness is real.

### Notifications

Underground events notify the player regardless of which level is viewed:

- Notification cards with "Go To" button → switches to the relevant level
- Sound cues: muffled rumbles on the surface when things happen below
- Smoke rising from stairways when there's fire below (visible on surface)

## Why No Bedrock

The player stops digging because:

1. **Cost exceeds reward** — maintaining -4 costs more labor and power than the ore is worth
2. **Technology gate** — can't dig -4 without powered drills, can't power drills without copper from -3
3. **Risk assessment** — losing -4 to a flood would cripple the colony. Is it worth it?
4. **Labor economics** — miners at -4 spend half their time walking stairs. Need an elevator.
5. **Cascading systems** — each level adds another thing that can go wrong

Nobody tells you to stop. You decide. And when you find a hint of ancient ruins at -5, you decide whether the gamble is worth it. That's a story. A hard floor is a wall.

## Implementation Phases (If Built)

1. **-1 only:** Single basement level. Stairway block. Level toggle UI. Separate grid data. Test all system interactions.
2. **Fluid coupling:** Two-sim coupling at connection tiles. Ventilation gameplay.
3. **Water table integration:** Basement flooding. Pump drainage.
4. **Variable depth (-2 to -3):** Multiple levels. Ore veins. Mining tools. Support structures.
5. **Deep levels (-4+):** Heat. Powered drilling. Elevator. Ancient ruins.

Each phase is a complete, playable feature. Ship -1 alone and the game is better. Ship -3 and it's a different game.

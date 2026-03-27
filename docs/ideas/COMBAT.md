# Real-Time Tactical Combat

Inspired by Door Kickers (top-down CQB, breaching, room clearing) and Baldur's Gate 2 (real-time with pause, party abilities, positioning). The goal: combat that emerges from the colony's spatial design — your walls, doors, furniture, and lighting ARE the battlefield.

## What Already Exists

The game has strong foundations for tactical combat:
- Top-down view (same perspective as Door Kickers)
- Walls, doors, thin walls (cover, choke points, breaching opportunities)
- DDA bullet physics with wall collision
- Sound propagation on GPU (enemies hear gunshots, footsteps)
- Fog of war with shadowcasting (limited vision, darkness matters)
- Day/night cycle (night raids, torch dependency)
- Multiple colonists (squad potential)
- Toxic grenade (area denial)

What's missing: tactical control, cover mechanics, enemy AI, weapon variety, the pause-and-plan loop.

## Real-Time with Pause

The colony sim is already real-time. Combat adds a tactical layer:

**Pause** (Space when enemies present, or auto-pause on contact):
- Time freezes
- Issue movement orders (click to move, shift-click to queue waypoints)
- Assign targets (click enemy to focus fire)
- Queue abilities (grenade, suppression, breach)
- Draw movement paths (Door Kickers style — see the plan before executing)
- Set go-codes for synchronized actions

**Unpause:** Watch it play out. Pause again anytime to adjust.

**Auto-pause triggers** (toggleable in settings):
- Enemy spotted
- Colonist takes damage
- Colonist downed
- Weapon jammed / out of ammo
- Door breached

This is exactly BG2's flow: pause, think, order, execute, react.

## Peace → Combat Transition

Combat isn't a separate mode — it's an interruption of colony life.

**When enemies are spotted:**
1. Alert notification with "Go To" button
2. Auto-pause (if enabled)
3. Non-combat colonists flee toward shelter (nearest roofed building, basement)
4. Combat-capable colonists (sheriff, outlaw, scout backstories) draw weapons
5. Player issues tactical orders

**When combat ends:**
1. "All clear" after 15 seconds with no visible enemies
2. Colonists holster weapons, return to duties
3. Wounded crawl toward safety or wait for rescue
4. Dead need burial (connects to graveyard, mood system)
5. Structural damage needs repair (fires, broken doors, wall breaches)

The colony bears scars from every fight. Combat isn't clean.

## Cover System

Walls already block bullets. Cover adds granularity:

**Full cover:** Standing behind a wall, completely hidden. Can't shoot, can't be shot. Must lean out to engage.

**Partial cover:** At a wall edge, behind furniture (bench, crate, table). Reduced chance to be hit. Can shoot from behind cover with accuracy penalty.

**Thin wall cover:** Wall thickness matters (DN-004). A 1-thick thin wall might not stop a rifle round. A 4-thick wall stops everything. Bullet penetration = bullet damage reduced by wall thickness. A desperate shot through a thin wall might wound someone on the other side.

**Windows:** Shoot through them, be shot through them. Glass breaks on first hit (becomes an opening). Provides visual cover but no physical protection once broken.

**Furniture:** Crouching behind a bench or flipping a table gives partial cover. Furniture has health — a crate absorbs a few hits then breaks. Destructible cover adds dynamism.

**Cover indicators:** When paused, show green/yellow/red zones around each colonist — where they're protected, partially exposed, or fully exposed.

## Line of Sight and Vision

The fog of war shadowcasting becomes tactical:

**Vision cones:** Each colonist has a ~120° forward vision arc (not 360°). Enemies outside the arc can't be targeted. Turning takes time.

**Blind spots:** Corners, room interiors beyond doorways, the area behind you. A colonist facing north doesn't see the enemy approaching from the south.

**Sound reveals position:** The sound sim carries gunshots, footsteps, door slams. A colonist HEARS combat through walls. Hearing doesn't give targeting but gives approximate direction — shown as a "?" indicator on the edge of fog of war.

**Muzzle flash:** Firing a weapon briefly reveals the shooter's position to anyone with line of sight to that tile. Even in darkness.

**Night combat:** Vision radius shrinks dramatically (fog of war system). Torches make you visible but let you see. Fighting in darkness: you aim at muzzle flashes and sound.

## Door Kickers-Style Breaching

Doors are central to the game. Combat makes them tactical:

### Stacking Up

Select 2-4 colonists. Right-click a door → "Stack". They arrange on both sides of the door automatically, backs to the wall, weapons ready. This is the iconic Door Kickers moment.

### Breach Options

| Method | Speed | Noise | Effect |
|--------|-------|-------|--------|
| Open quietly | Slow (3s) | Silent | Door opens, no alert |
| Kick | Fast (0.5s) | Loud (sound propagates) | Door flies open, brief stun |
| Lock pick | Very slow (8s) | Silent | Unlocks locked doors |
| Explosive charge | Instant | Extreme (heard across map) | Destroys door + adjacent wall section |

### Room Clearing

After breach:
1. First colonists sweep left and right (cover corners)
2. Subsequent colonists push forward (cover center)
3. "Clear!" callout after all corners checked
4. The order happens automatically based on stack positions — player just says "go"

The sound sim matters enormously: a kicked door alerts everyone in the building. A quiet open only alerts people in the room. An explosive breach is heard by the entire map — every enemy knows where you are.

### Flash Grenade

Throw before entering. Blinds and stuns enemies in the room for 3-5 seconds. Doesn't work through walls (line of sight required). Window breach: throw flash through window, breach through door simultaneously.

## Suppression

Bullets passing near (within 1 tile) a character cause suppression:

- **Accuracy drops** (can't aim well while being shot at)
- **Movement slows** (instinct to stay down)
- **Stress increases** (combat is traumatic)
- **Suppressed AI takes cover** instead of advancing

Tactical use: one colonist lays suppressing fire at a doorway. The enemy can't advance through it. Meanwhile another colonist flanks through a different entrance. Classic fire-and-maneuver.

Suppression is shown visually: small "impact" indicators near the suppressed character. Dust kicks up. Suppressed characters crouch lower (body sprite variant).

## Weapon System

Currently there's one gun. A weapon system tied to the crafting chain:

### Ranged

| Weapon | Range | Damage | Noise | Rate of Fire | Ammo | Notes |
|--------|-------|--------|-------|-------------|------|-------|
| Bow | 8 tiles | Low | Quiet | Medium | Arrows (wood+stone) | Craftable early, stealth option |
| Revolver | 10 tiles | Medium | Loud | Medium | Bullets (iron) | Reliable sidearm |
| Rifle | 18 tiles | High | Very loud | Slow | Bullets (iron) | Best at range, poor in CQB |
| Shotgun | 5 tiles | Very high | Loud | Slow | Shells (iron+gunpowder) | Spread pattern, devastating up close |
| Repeater | 10 tiles | Medium | Loud | Fast | Bullets (iron) | Suppression weapon, burns ammo |

### Melee

| Weapon | Range | Damage | Noise | Speed | Notes |
|--------|-------|--------|-------|-------|-------|
| Knife | Adjacent | Medium | Silent | Fast | Stealth kills from behind |
| Hatchet | Adjacent | High | Quiet | Medium | Also a tool (dual use) |
| Pickaxe | Adjacent | High | Quiet | Slow | Also a mining tool |
| Fists | Adjacent | Low | Silent | Fast | Always available, last resort |

### Throwables

| Item | Range | Effect | Noise | Notes |
|------|-------|--------|-------|-------|
| Toxic grenade | 6 tiles | Gas cloud (existing) | Medium | Area denial |
| Flash grenade | 5 tiles | Blind + stun 3-5s | Loud | Breaching tool |
| Dynamite | 4 tiles | Explosion + wall destruction | Extreme | Mining + combat dual use |
| Molotov | 5 tiles | Fire (existing fire system) | Medium | Area denial, burns structures |

Weapons are **crafted items** from the existing item/recipe system. Better weapons need deeper mining materials (iron, copper, gunpowder). A colony with only bows is at a disadvantage against rifle-wielding Redskulls — motivation to advance the crafting chain.

## Abilities from Backstory

Each backstory (CHARGEN.md) grants a unique combat ability:

| Backstory | Ability | Effect | Cooldown |
|-----------|---------|--------|----------|
| Sheriff | **Rally** | Nearby allies: +30% accuracy, +stress resistance for 20s | Long (120s) |
| Outlaw | **Quick Draw** | Instant shot, guaranteed hit on first attack | Long (90s) |
| Scout | **Eagle Eye** | Reveals all enemies in huge radius for 15s | Medium (60s) |
| Drifter | **Dirty Fight** | Next melee attack: 3× damage + stun | Medium (45s) |
| Ranch Hand | **Lasso** | Immobilize one enemy for 5s (ranged, silent) | Medium (60s) |
| Mechanic | **Jury Rig** | Instantly repair a door or wall section | Medium (60s) |
| Doc | **Field Medic** | Stabilize a downed colonist from 3 tiles away | Short (30s) |
| Preacher | **Calm** | Remove suppression from all nearby allies | Medium (45s) |
| Engineer | **Fortify** | Temporarily reinforces a wall section (double HP) | Long (90s) |
| Convict | **Berserker** | Double move speed + melee damage, can't use ranged | Long (120s) |

These are the same ability cards from CHARGEN.md. They create distinct combat roles — a sheriff leads, a doc supports, a scout provides intel, an outlaw strikes first.

## Enemy AI

Replace random-walk with tactical behavior:

### Awareness States

| State | Behavior | Trigger |
|-------|----------|---------|
| **Patrol** | Walk a route near their camp | Default |
| **Alerted** | Move toward sound source, weapon ready | Hears gunshot, door kick, explosion |
| **Engaged** | Take cover, return fire, attempt flanking | Sees a colonist, takes damage |
| **Retreating** | Fall back toward reinforcements | Outnumbered, critically wounded |
| **Surrendering** | Drops weapon, hands up | Alone, low health, surrounded |

### Tactical Behaviors

- **Take cover:** Move to nearest wall edge or furniture when shot at
- **Peek and shoot:** Lean from behind cover, fire, duck back
- **Flank:** If pinned from one direction, send members around another route
- **Call out:** Shout to alert nearby allies (sound propagates through sound sim — you can hear them coordinating)
- **Breach:** Enemies can kick your doors too. A Redskull raiding party stacks on your front door at dawn.
- **Grenade:** Toss explosives through windows or around corners

### Enemy Types

| Type | Speed | Health | Weapon | Behavior |
|------|-------|--------|--------|----------|
| Scout | Fast | Low | Bow/knife | Spots colony, reports back, avoids combat |
| Raider | Medium | Medium | Revolver | Standard threat, fights in groups |
| Heavy | Slow | High | Shotgun | Pushes through doors, absorbs damage |
| Sharpshooter | Slow | Low | Rifle | Stays far back, picks off exposed colonists |
| Demolitionist | Medium | Medium | Dynamite | Breaches walls, creates new entry points |
| Leader | Medium | High | Repeater | Buffs nearby raiders, coordinates assault |

A raid isn't just "enemies appear." It's an organized assault: scouts found you, raiders approach from a planned direction, the heavy breaches the door, sharpshooter covers from a hill, demolitionist blows a wall when the front is contested.

## Wounds and Down System

No instant death. Combat creates medical emergencies:

### Damage Model
- Each hit reduces health
- Armor (clothing system from CHARACTER_VISUALS.md) absorbs some damage
- Hit location matters loosely: torso hits = serious, limb hits = mobility/work impact

### Down States
1. **Healthy** (100-50% HP): Normal function, increasing pain debuff
2. **Wounded** (50-25% HP): Slower movement, reduced accuracy, visible blood on sprite
3. **Critical** (25-1% HP): Crawling, can't fight, cries for help (sound sim carries the call)
4. **Downed** (0% HP): Unconscious, bleeding out. 60-120 seconds to stabilize or death
5. **Dead**: Permanent. Corpse remains. Colonists who witness it: major mood penalty.

### Rescue
A colonist can stabilize a downed ally:
- Must reach them (might mean fighting through enemies)
- Takes 5-10 seconds (vulnerable during)
- Stabilized colonist stops bleeding, can be carried to medical bed
- The Doc backstory does this faster and from range (ability)

This creates the most dramatic combat moments: fighting through a doorway to reach a downed friend before they bleed out. Do you risk another colonist to save them?

### Permanent Injuries
Surviving critical wounds may leave lasting effects:
- Limp (reduced movement speed — visible in walk animation)
- Scarred (visible on sprite — from CHARACTER_VISUALS.md)
- Lost finger (reduced craft speed)
- Concussion (periodic headaches — brief work pauses)
- PTSD (combat sounds trigger stress spikes)

These make combat COSTLY even when you win. A pyrrhic victory against a raid leaves your colony weakened for months.

## Defensive Structures

Buildings you construct for defense:

- **Sandbags:** Low wall (half-height), provides partial cover, fast to build, no roof
- **Watchtower:** Elevated position, extended vision radius, rifle bonus. The scout's home.
- **Kill box:** Corridor with murder holes — thin walls with windows facing inward. Classic Rimworld tactic, but here the thin wall system makes it architecturally natural.
- **Traps:** Spike pit (dug ground + stakes), tripwire alarm (sound sim — alerts you without alerting enemies)
- **Gate:** Heavy door that takes longer to breach. Drawbridge version for settlements with moats?
- **Armory:** Storage room for weapons and ammo near the colony entrance. Quick access during raids.

## The Shootout

The western fantasy moment: two people facing each other in the open. How to make this feel iconic?

When a colonist and enemy are in the open, facing each other within 8 tiles, with no other combatants nearby — trigger a **duel** event:

- Brief slowdown (not full pause — just 0.5× speed for 2-3 seconds)
- Camera zooms slightly
- Both characters square up (face each other, hands near weapons)
- First shot determined by: weapon draw speed + character's reaction stat
- The "Quick Draw" ability (Outlaw) guarantees winning the draw

This is pure flavor — mechanically it's just two people shooting at each other. But the presentation makes it a MOMENT. The Morricone moment.

## Sound as Tactical Tool

The existing sound propagation becomes a weapon:

- **Diversion:** Fire a shot away from your approach. Enemies investigate the sound. You breach from the other side.
- **Stealth approach:** Bows and knives are quiet. A scout with a bow can eliminate a sentry without alerting the camp.
- **Sound trap:** Ring an alarm bell (crafted item) to draw enemies to a kill zone.
- **Noise discipline:** Colonists ordered to "hold fire" don't shoot until commanded. Prevents premature engagement.
- **Enemy communication:** You can HEAR enemies calling to each other. Their shouts propagate through the sound sim. A perceptive player can gauge enemy numbers and positions from sound alone.

## Integration with Colony Systems

Combat isn't separate from colony life — it emerges from and affects every system:

| System | Combat Connection |
|--------|------------------|
| Thin walls | Cover quality based on thickness. Bullets penetrate thin walls. |
| Doors/windows | Breaching points. Windows = firing positions. |
| Sound sim | Enemies hear your actions. You hear theirs. Diversion tactics. |
| Fog of war | Vision cones, darkness, torch reveals position. |
| Fire system | Molotov cocktails. Burning buildings. Smoke concealment. |
| Fluid sim | Smoke from fire/grenades affects visibility and breathing. |
| Crafting | Weapons and ammo are crafted items. Better gear needs better materials. |
| Medical | Wounds need treatment. Doc backstory is critical. |
| Stress | Combat is traumatic. PTSD from repeated fights. |
| Chargen | Backstory determines combat role and unique ability. |
| Cards | Raid events dealt from the frontier deck. Intensity scales with wealth. |
| Basements | Flee to shelter. Underground ambush. Tunnel warfare. |

# Rayworld — Pleb AI System Design

## Philosophy

Plebs are not units to be micromanaged. They are *people* with needs, personalities, skills, moods, and relationships. The player provides direction — assigning priorities, designating zones, queuing tasks — but plebs make their own moment-to-moment decisions. In crises, they may override the player entirely: a pleb won't walk into a burning room just because you told them to haul stone from it. A panicking pleb might flee through a door you wanted kept sealed.

The emergent stories come from this tension: the player's plans vs. the plebs' humanity.

---

## Needs System

Every pleb has **6 core needs**, each a float from 0.0 (desperate) to 1.0 (satisfied). Needs decay over time and are restored by specific actions. When a need drops below a threshold, the pleb's behavior changes.

| Need | Decay Rate | Restored By | Crisis Threshold |
|------|-----------|-------------|-----------------|
| **Hunger** | -0.003/sec | Eating food | 0.15 |
| **Rest** | -0.002/sec | Sleeping on bed/bedroll | 0.10 |
| **Warmth** | variable (from air temp) | Being in warm area (>15°C) | 0.20 |
| **Oxygen** | variable (from O2 level) | Being in area with O2 > 0.5 | 0.25 |
| **Safety** | variable (from threats) | No nearby threats | 0.20 |
| **Comfort** | -0.001/sec | Being in lit, furnished room | 0.10 |

### Warmth Need
Directly driven by the fluid simulation's temperature field (dye.a channel):
- Air temp > 20°C: warmth increases toward 1.0
- Air temp 10-20°C: warmth slowly decays
- Air temp < 10°C: warmth decays fast
- Air temp < 0°C: warmth plummets, health damage

### Oxygen Need
Directly driven by the O2 field (dye.g channel):
- O2 > 0.8: oxygen need stays at 1.0
- O2 0.5-0.8: oxygen slowly decays
- O2 0.3-0.5: oxygen decays fast, pleb coughs (visual)
- O2 < 0.3: oxygen plummets, health damage, pleb panics

### Safety Need
Computed from nearby threats:
- Fire within 3 tiles: safety drops
- Toxic CO2 > 0.5: safety drops
- Smoke density > 1.0: safety drops (can't see)
- Hostile entities nearby: safety drops
- No threats: safety recovers toward 1.0

---

## Mood System

Mood is a rolling average of need satisfaction, personality modifiers, and recent events. Range: -100 (mental break) to +100 (inspired).

```
mood = base_mood
     + (hunger - 0.5) * 30
     + (rest - 0.5) * 25
     + (warmth - 0.5) * 20
     + (oxygen - 0.5) * 40    ← O2 deprivation is very distressing
     + (safety - 0.5) * 35
     + (comfort - 0.5) * 10
     + personality_offset
     + sum(recent_events)
```

### Mood Thresholds

| Range | State | Behavior |
|-------|-------|----------|
| 80-100 | **Inspired** | +20% work speed, may do creative tasks |
| 40-80 | **Happy** | Normal behavior |
| 0-40 | **Okay** | Slightly slower, occasional sighs |
| -20-0 | **Stressed** | Slower work, may refuse low-priority tasks |
| -50 to -20 | **Unhappy** | Very slow work, complains, may refuse orders |
| -80 to -50 | **Breaking** | Stops working, may wander aimlessly |
| -100 to -80 | **Mental Break** | See Mental Breaks below |

### Recent Events (mood modifiers, decay over 60 seconds game-time)

| Event | Mood Impact | Duration |
|-------|------------|----------|
| Ate a good meal | +10 | 60s |
| Slept in a bed | +8 | 60s |
| Slept on floor | -5 | 60s |
| Witnessed death | -30 | 120s |
| Near fire (scared) | -15 | 30s |
| Suffocating (low O2) | -25 | while occurring |
| Completed a task | +3 | 30s |
| Idle (nothing to do) | -2 | while occurring |
| In darkness | -5 | while occurring |
| Beautiful room | +5 | while in room |

---

## Personality Traits

Each pleb has 2-3 randomly assigned traits that modify behavior and mood. Traits are permanent.

### Positive Traits
- **Hardworker**: +15% work speed, rarely refuses tasks
- **Optimist**: +10 base mood
- **Brave**: safety need decays slower, won't panic until O2 < 0.15
- **Night Owl**: no mood penalty for darkness, +10% speed at night
- **Green Thumb**: +30% plant work speed
- **Iron Stomach**: hunger decays 50% slower

### Negative Traits
- **Lazy**: -10% work speed, may refuse tasks when mood < 20
- **Pessimist**: -10 base mood
- **Pyromaniac**: may set fires during mental break (!!!)
- **Claustrophobic**: mood penalty in small/roofed rooms
- **Insomniac**: rest need decays 50% faster
- **Wimp**: safety need decays 2x faster, panics easily

### Neutral Traits
- **Loner**: mood penalty near other plebs, bonus when alone
- **Social**: mood bonus near others, penalty when alone
- **Teetotaler**: won't consume alcohol (future)
- **Ascetic**: doesn't need comfort, but also doesn't benefit from luxury

---

## Decision Making: Utility AI

Plebs use a **utility-based AI** system. Each possible action has a utility score computed from the pleb's current needs, mood, personality, and the world state. The pleb picks the highest-utility action each tick.

### Action Categories (priority order)

**1. Crisis Actions (override everything, including player orders)**
- **Flee fire**: utility = 1000 if fire within 2 tiles
- **Gasp for air**: utility = 900 if O2 < 0.2 (pleb runs toward highest O2)
- **Mental break**: utility = 800 if mood < -80

**2. Critical Needs (override player work orders)**
- **Eat**: utility = `(1.0 - hunger) * 200` when hunger < 0.3
- **Sleep**: utility = `(1.0 - rest) * 180` when rest < 0.2
- **Seek warmth**: utility = `(1.0 - warmth) * 250` when warmth < 0.25
- **Seek air**: utility = `(1.0 - oxygen) * 300` when oxygen < 0.4

**3. Player-Assigned Work (respects priority queue)**
- **Haul**: utility = 50 + priority_bonus
- **Build**: utility = 55 + priority_bonus
- **Craft**: utility = 45 + priority_bonus
- **Chop wood**: utility = 40 + priority_bonus
- **Mine stone**: utility = 40 + priority_bonus

**4. Self-Care (fills remaining needs)**
- **Eat (not critical)**: utility = `(1.0 - hunger) * 60`
- **Rest (not critical)**: utility = `(1.0 - rest) * 50`
- **Socialize**: utility = 20 (if Social trait)
- **Seek comfort**: utility = `(1.0 - comfort) * 30`

**5. Idle**
- **Wander**: utility = 5
- **Stand idle**: utility = 1

### Action Execution

Each action is a state machine:
```
IDLE → PATHFINDING → WALKING → WORKING → DONE → (next action)
```

States:
- **IDLE**: evaluating next action
- **PATHFINDING**: computing A* path to target
- **WALKING**: following path (continuous movement, auto-opens doors)
- **WORKING**: at target, performing action (time-based, skill-dependent)
- **FLEEING**: crisis override, moving toward safety (ignores player orders)

### Interruption Rules

| Current State | Can Be Interrupted By |
|--------------|----------------------|
| Working | Crisis, Critical Need (if need < threshold) |
| Walking | Crisis, Critical Need, Higher-Priority Work |
| Idle | Anything |
| Fleeing | Nothing (must reach safety first) |

---

## Mental Breaks

When mood drops below -80, a pleb enters a mental break. The break type is influenced by personality traits. Breaks last 30-120 seconds and cannot be player-cancelled.

| Break Type | Trigger Trait | Behavior |
|-----------|--------------|----------|
| **Berserk** | (any) | Attacks nearby objects/plebs |
| **Wander** | Pessimist | Walks aimlessly, ignores everything |
| **Fire Starting** | Pyromaniac | Sets fires in buildings (!!!!) |
| **Catatonic** | (any) | Collapses on floor, unresponsive |
| **Binge Eating** | (any) | Eats all available food |
| **Flee Map** | Claustrophobic | Runs toward map edge and leaves |
| **Hide** | Wimp | Finds a dark corner and cowers |

A pyromaniac's fire-starting break is particularly dangerous because it interacts with the fluid sim — the fire will consume O2, produce smoke, and potentially suffocate other plebs in sealed rooms. This creates cascading emergencies.

---

## Pathfinding Integration

Plebs use A* pathfinding on the block grid with **fluid-aware cost modifiers**:

| Condition | Path Cost Modifier |
|-----------|-------------------|
| Normal floor | 1.0 |
| Through door (closed) | 1.5 (has to open it) |
| Through smoke (density > 0.5) | 3.0 (avoids if possible) |
| Low O2 area (< 0.5) | 5.0 (strongly avoids) |
| High CO2 area (> 0.3) | 4.0 (avoids) |
| Near fire (< 3 tiles) | 8.0 (very strongly avoids) |
| Cold area (< 5°C) | 2.0 |
| Dark area (lightmap < 0.05) | 1.5 |

This means plebs naturally route around hazards. A pleb told to haul from a smoky room will take the long way around rather than walk through the smoke — unless that's the only path.

### Emergency Pathfinding

In crisis mode (fleeing), the pleb uses a special pathfinder:
- **Flee fire**: A* toward the tile with lowest fire threat
- **Seek air**: gradient ascent on O2 field (move toward highest O2)
- **Seek warmth**: gradient ascent on temperature field

These can sample the fluid sim directly — the dye texture gives O2/CO2 at any position.

---

## Skills System

Each pleb has skill levels (0-20) that improve with use:

| Skill | Affects | Used By |
|-------|---------|---------|
| **Construction** | Build speed, quality | Building walls, furniture |
| **Mining** | Mine speed | Mining stone |
| **Woodcutting** | Chop speed | Felling trees |
| **Cooking** | Cook speed, food quality | Preparing meals |
| **Crafting** | Craft speed, quality | Making items |
| **Medical** | Heal speed, effectiveness | Treating injuries |
| **Social** | Trade prices, mood boost | Socializing, trading |
| **Athletics** | Move speed | Everything (base speed) |

Skill XP formula: `xp += 1.0 / (1.0 + current_level * 0.5)` per second of work. Diminishing returns at higher levels.

Work speed: `base_time / (1.0 + skill_level * 0.1)`. A level 10 pleb works 2x as fast as level 0.

---

## Health System

Health is a float from 0.0 (dead) to 1.0 (healthy). Damage sources:

| Source | Rate | Conditions |
|--------|------|------------|
| Suffocation | -0.05/sec | O2 < 0.2 at pleb position |
| CO2 poisoning | -0.02/sec | CO2 > 0.8 at pleb position |
| Hypothermia | -0.03/sec | Air temp < -5°C for > 10s |
| Burning | -0.10/sec | Adjacent to fire block |
| Starvation | -0.01/sec | Hunger need at 0.0 |
| Exhaustion | -0.005/sec | Rest need at 0.0 |

Healing: +0.01/sec when resting in bed with all needs > 0.5.

Death at health = 0.0. Body remains as a block (type 14?), decays over time, produces CO2 (decomposition interacts with fluid sim).

---

## Social System (Future)

Plebs have relationships with each other:
- **Opinion**: -100 to +100, modified by interactions
- **Bonding**: shared work, meals together, surviving crises
- **Rivalry**: competition for resources, personality clashes

Social actions:
- **Chat**: both plebs get +5 mood, builds relationship
- **Argue**: both plebs get -10 mood, damages relationship
- **Console**: high-social pleb helps low-mood pleb (+15 mood)

Personality compatibility:
- Optimist + Pessimist: tend to clash
- Social + Social: fast bonding
- Loner + anyone: slow bonding
- Hardworker + Lazy: tend to clash

---

## Player Control Interface

### Priority Queue
The player sets **work type priorities** per pleb (1-5 scale, or disabled):
```
Jeff:    Build[5] Mine[3] Haul[4] Cook[-] Craft[2]
Sarah:   Build[2] Mine[-] Haul[3] Cook[5] Craft[4]
```

Higher priority = higher utility score for that work type. Disabled = pleb won't do it.

### Direct Orders
- **Click pleb → Click target**: move to position (A* pathfinding)
- **Click pleb → Click object**: interact with object
- **Draft mode**: direct WASD control (overrides AI, pleb still has needs)
- Drafted plebs ignore work queue but still respond to critical needs

### Designations
- **Zone painting**: stockpile zones, sleeping areas, recreation areas
- **Harvest**: mark trees/rocks for gathering
- **Build**: place blueprints for construction
- **Forbid**: mark items/areas as off-limits

---

## Integration with Fluid Sim

The pleb AI is deeply coupled with the fluid simulation:

1. **Needs from fluid state**: warmth (temperature), oxygen (O2 level), safety (smoke/fire) are sampled directly from the dye texture at the pleb's position each frame.

2. **Pathfinding costs from fluid state**: A* path costs are modified by smoke density, O2 level, CO2 level, and temperature at each grid cell.

3. **Pleb as fluid source**: each pleb produces trace CO2 (breathing) and consumes trace O2. In a sealed room with many plebs, O2 depletes and CO2 rises — creating a suffocation crisis.

4. **Mental break interactions**: a pyromaniac pleb creates fire blocks, which then produce smoke, consume O2, and potentially cascade into a suffocation crisis for other plebs.

5. **Door management**: plebs open doors (creating gas exchange), then doors auto-close. The timing matters — a pleb fleeing a smoky room leaves the door open long enough for smoke to pour into the hallway.

6. **Death produces CO2**: decomposing bodies produce CO2, creating a secondary hazard in sealed spaces.

---

## Implementation Priority

### Phase 3a: Single Pleb AI (Jeff)
- [x] Position, movement, A* pathfinding
- [x] Auto-door opening
- [ ] Needs: hunger, rest (basic decay + restoration)
- [ ] Need-driven behavior: seek food, seek bed
- [ ] Fluid-driven needs: warmth (from temperature), oxygen (from O2)
- [ ] Crisis behavior: flee fire, seek O2
- [ ] Health system (damage from suffocation, fire, cold)
- [ ] Info panel: show needs, mood, current action

### Phase 3b: Multiple Plebs
- [ ] Spawn/manage multiple plebs
- [ ] Priority queue UI
- [ ] Work assignment system
- [ ] Each pleb as CO2 source in fluid sim
- [ ] Basic social interactions

### Phase 3c: Personality & Mood
- [ ] Trait system (random assignment on spawn)
- [ ] Mood computation with event log
- [ ] Mental breaks
- [ ] Skill system with XP
- [ ] Pleb info panel with full details

### Phase 3d: Advanced AI
- [ ] Fluid-aware pathfinding (cost modifiers from gas/temp)
- [ ] Emergency pathfinding (gradient ascent on O2/temp)
- [ ] Social system (relationships, opinions)
- [ ] Draft mode (player direct control overrides AI)

---

## Technical Notes

### Performance
- AI runs on CPU (not GPU) — decision making is branchy, not parallelizable
- Needs update: once per game tick (not per frame)
- Pathfinding: cached, recomputed only when destination changes or path blocked
- Fluid sampling: read pleb position → sample dye texture via GPU readback (existing debug readback pattern) or approximate on CPU from recent values

### Data Layout
```rust
struct Pleb {
    // Identity
    name: String,
    traits: Vec<Trait>,
    skills: [f32; 8],         // skill levels 0-20

    // Position & movement
    x: f32, y: f32,
    angle: f32,
    speed: f32,
    path: Vec<(i32, i32)>,
    path_idx: usize,

    // Equipment
    torch_on: bool,
    headlight_on: bool,

    // Needs (0.0 - 1.0)
    hunger: f32,
    rest: f32,
    warmth: f32,
    oxygen: f32,
    safety: f32,
    comfort: f32,

    // Health & mood
    health: f32,              // 0.0 = dead, 1.0 = healthy
    mood: f32,                // -100 to +100
    mood_events: Vec<(MoodEvent, f32)>,  // (event, time_remaining)

    // AI state
    current_action: Action,
    action_state: ActionState, // Idle, Pathfinding, Walking, Working, Fleeing
    action_progress: f32,      // 0.0 - 1.0 for timed actions

    // Social
    relationships: HashMap<usize, f32>,  // pleb_id → opinion
}
```

### Tick Rate
- AI evaluation: every 30 frames (~2x per second at 60fps)
- Need decay: every frame (scaled by dt)
- Mood computation: every 60 frames (~1x per second)
- Pathfinding: on demand (cached)
- Fluid sampling: every 10 frames (interpolate between samples)

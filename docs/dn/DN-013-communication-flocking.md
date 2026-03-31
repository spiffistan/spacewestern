# DN-013: Pleb Communication & Group Movement

## Overview

Two interconnected systems that make combat feel alive:
1. **Shout system** — plebs vocalize alerts, creating physical sound-based communication
2. **Flocking** — implicit squad cohesion from proximity, with boids-like spacing

Both are designed to be **decoupled from core simulation** — they read pleb state and write small adjustments, but don't own any critical game logic. All code lives in a new `src/comms.rs` module.

## 1. Shout System

### Data Model

```rust
// src/comms.rs

#[derive(Clone, Copy, Debug)]
pub enum ShoutKind {
    Alert,     // "Enemy spotted!" — first target acquisition
    Retreat,   // "Fall back!" — health < 35%
    Help,      // "I'm hit!" — health drops below 50%
    Covering,  // "Covering!" — behind cover and firing
    Clear,     // "All clear" — no enemies in range
}

#[derive(Clone, Debug)]
pub struct Shout {
    pub kind: ShoutKind,
    pub x: f32,
    pub y: f32,
    pub pleb_idx: usize,
    pub is_enemy: bool,
    pub range: f32, // hearing radius in tiles
}
```

### Shout Collection

Each frame, `collect_shouts()` scans all plebs and generates shouts based on state transitions:

| Trigger | ShoutKind | Range | Condition |
|---------|-----------|-------|-----------|
| First target acquired | `Alert` | 20 | `aim_target` goes from None → Some |
| Health drops below 35% | `Retreat` | 15 | `health < 0.35 && was_above_last_frame` |
| Health drops below 50% | `Help` | 20 | `health < 0.5 && was_above_last_frame` |
| Firing from cover | `Covering` | 10 | `aim_progress >= fire_threshold && is_behind_cover` |
| No enemies in range | `Clear` | 15 | Had `aim_target`, now None for 2+ seconds |

Cooldown: each pleb has `last_shout_timer: f32` (decays each frame). A pleb can only shout once every 3 seconds to prevent spam.

### Shout Processing

`process_shouts()` iterates shouts × plebs. For each pleb within range of a shout from their own faction:

- **Distance check**: `dist < shout.range`
- **LOS check (optional)**: walls muffle — if `edge_blocked_wd` between shout origin and listener tile, halve the effective range
- **Same faction only**: enemies don't react to friendly shouts and vice versa

### Reactions

| Listener State | Shout Heard | Reaction |
|----------------|-------------|----------|
| Undrafted, idle | Alert | Flee to cover (existing crisis system) |
| Drafted, no target | Alert | Face shout direction, show "!" bubble |
| Drafted, firing | Help | If closer than 10 tiles, adjust aim_target to Help caller's target |
| Any, health > 35% | Retreat | No forced reaction (information only) |
| Any, in combat | Covering | No forced reaction (information only, reduces friendly fire) |
| Any, in combat | Clear | Clear aim_target, stop firing, show bubble |

### Audio

Each shout emits a sound source:
- Friendly shouts: pattern 1 (sine), 300Hz, 60dB, 0.15s duration
- Enemy shouts: pattern 1 (sine), 500Hz, 60dB, 0.15s duration
- Help shouts: pattern 2 (noise burst), 400Hz, 70dB, 0.2s — more urgent

### Sound Physics Integration (TODO)

**Current state**: Shout hearing uses a CPU-side distance + `edge_blocked_wd` check. This is fast but simplistic — it treats walls as binary blockers and doesn't account for diffraction, multi-bounce, or partial occlusion.

**Ideal state**: The GPU sound propagation system (`sound.wgsl`) already simulates realistic wave propagation — diffraction around corners, attenuation through walls, distance falloff. Shouts should feed into this system and plebs should "hear" by sampling the GPU sound field at their position.

**Blockers**: The GPU sound field is not currently read back to the CPU per-tile. Only the selected pleb's tile gets a readback (for the debug tooltip). Full-field readback would be expensive (~256×256 texture read per frame).

**Practical path forward**:
1. **Phase 1 (current)**: CPU distance + wall check. Good enough for gameplay — walls block, distance attenuates. Feels right 90% of the time.
2. **Phase 2**: Selective GPU readback — when a shout fires, read back sound pressure at a handful of pleb positions (not the whole field). Queue async readback for ~16 tiles max per frame.
3. **Phase 3**: Full integration — shout emits a sound source into the GPU sim at a unique frequency band. Plebs listen for that frequency in the readback. True physical propagation including diffraction, echo, and material absorption. Shouts around corners would work naturally; shouting into a cave would echo and reach further.

Phase 2 is the sweet spot — physically grounded hearing with minimal GPU overhead.

### Bubbles

Shouts display as text bubbles on the shouting pleb:
- Alert → "Contact!"
- Retreat → "Fall back!"
- Help → "I'm hit!"
- Covering → "Covering!"
- Clear → "Clear!"

## 2. Flocking / Group Movement

### Design Principle

No explicit squad assignment. Groups emerge from proximity + shared movement direction. Three boids-like forces gently modulate velocity.

### Detection

A pleb is "in a group" if there are 1+ friendly drafted plebs within `GROUP_RADIUS` (8 tiles) who are also moving (have a non-empty path).

### Forces (applied in `apply_flocking()`)

All forces are small velocity adjustments (max ±15% of base speed):

**Separation** (prevent stacking):
- If any friendly < `MIN_SPACING` (1.2 tiles): nudge away
- Force magnitude: `(MIN_SPACING - dist) / MIN_SPACING * 0.15 * base_speed`

**Cohesion** (prevent stragglers):
- If nearest friendly > `MAX_SPACING` (5.0 tiles) and moving same direction: nudge toward group center
- Force magnitude: `(dist - MAX_SPACING) / MAX_SPACING * 0.10 * base_speed`

**Alignment** (smooth movement):
- Blend pleb's movement angle slightly toward average group angle
- Blend factor: `0.05` per frame (very gentle)

### Phase Transitions

Phases are not explicit state — they emerge from conditions:

| Condition | Emergent Behavior |
|-----------|-------------------|
| No enemies in 25 tiles | Cohesion active, tight spacing → "form up" |
| Enemies 15-25 tiles | Cohesion weaker, speed reduces 20% → "cautious approach" |
| Enemies < 15 tiles | Cohesion off, plebs seek cover independently → "disperse" |
| In cover, firing | All forces off, pure individual AI → "independent combat" |

### Implementation

Flocking is computed AFTER pathfinding but BEFORE the final position update. It modifies `vx`/`vy` (or equivalently, adjusts the effective speed and angle during path following). It never overrides pathfinding — it nudges within the path-following movement.

## Module Structure

```
src/comms.rs
├── ShoutKind enum
├── Shout struct
├── collect_shouts(plebs, dt) -> Vec<Shout>
├── process_shouts(plebs, shouts, grid, wall_data)
├── apply_flocking(plebs, dt) -> Vec<(usize, f32, f32)>  // (pleb_idx, dx_adjust, dy_adjust)
└── constants: GROUP_RADIUS, MIN_SPACING, MAX_SPACING, SHOUT_COOLDOWN
```

### Integration Points (in simulation.rs)

```rust
// After combat loop, before pleb movement:
let shouts = comms::collect_shouts(&self.plebs, dt);
comms::process_shouts(&mut self.plebs, &shouts, &self.grid_data, &self.wall_data);

// In the pleb movement section, after computing effective_speed:
let flock_adjustments = comms::apply_flocking(&self.plebs, dt);
// Apply adjustments to pleb positions during path following
```

### New Fields on Pleb

```rust
pub last_shout_timer: f32,   // cooldown between shouts (decays, shout when <= 0)
pub prev_health_band: u8,    // tracks health threshold crossings (0=full, 1=<50%, 2=<35%)
```

## Why This Design

- **Physical grounding**: shouts use the sound system, not magic telepathy
- **Emergent behavior**: no squad commands needed, groups form naturally
- **Decoupled**: comms.rs reads pleb state, writes small adjustments. Remove it and the game works exactly as before.
- **Audible to the player**: you hear your plebs calling out, hear enemy chatter before an attack
- **Scalable**: add new ShoutKinds without touching other systems

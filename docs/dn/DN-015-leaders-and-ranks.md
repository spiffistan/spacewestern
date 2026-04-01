# DN-015: Leaders, Ranks, and Command Shouts

## Overview

A lightweight RPG layer on top of the existing pleb system. Leaders are named characters with command abilities; ranks emerge from combat experience. Commands are shouts issued through the action bar that affect nearby allies' morale and behavior.

## Ranks

Ranks are earned through combat experience tracked per-pleb. Two counters: `firefights_survived` (increments when a firefight ends and pleb is alive) and `kills` (enemies killed by this pleb).

| Rank | Firefights | Kills | Stress modifier | Accuracy modifier |
|------|-----------|-------|----------------|-------------------|
| Green | 0-2 | any | +20% stress gain | -10% aim speed |
| Trained | 3-7 | any | normal | normal |
| Veteran | 8-14 | 2+ | -15% stress gain | +10% aim speed |
| Hardened | 15+ | any | -30% stress gain | passive rally aura |

Rank is computed from the counters, not stored — no desync risk.

## Leaders

A pleb is a leader if `pleb.is_leader == true`. Starting crew are leaders. Future: promote via UI.

### Leader abilities
- **Rally shout** — reduces ally stress by 15-20 within 12 tiles. Cooldown 10s.
- **Command shouts** — issue tactical orders via action bar menu: Advance, Hold, Fall Back.
- **Command radius** — allies within 10 tiles of a leader gain +1/sec passive stress recovery.
- **Death impact** — leader death applies +30 stress to all allies within 12 tiles.

### Command shouts (action bar)

When a leader is selected, the action bar shows a "Command" tile. Clicking cycles or opens commands:

| Command | Shout text | Effect on nearby allies |
|---------|-----------|----------------------|
| Rally | "Hold the line!" | -15 stress, +0.3 suppression resistance for 5s |
| Advance | "Move up!" | Allies pathfind toward leader's facing, +5 stress (courage cost) |
| Fall Back | "Pull back!" | Allies disengage combat, pathfind behind leader, -10 stress |

Each command:
1. Emits a sound source (shout audio)
2. Shows a bubble on the leader
3. Affects all same-faction plebs within range (12 tiles)
4. Uses the existing shout system infrastructure (wall muffling, range)
5. Has a cooldown (shared across all commands, 8s)

## Experience Tracking

```rust
pub firefights_survived: u16,  // increments when combat ends (no enemies in 25 tiles for 5s)
pub kills: u16,                // increments on confirmed kill
```

A "firefight" is detected by tracking whether the pleb had `aim_target.is_some()` during the last combat encounter. When all enemies are dead or out of range for 5s, surviving plebs who participated get +1 firefight.

## Rank Effects Integration

Rank modifiers apply in `morale.rs`:
- Stress gain: `apply_stress` multiplies by rank modifier (0.7 for Hardened, 1.2 for Green)
- Aim speed: combat code multiplies by rank modifier

Hardened plebs passively rally nearby Greens: if a Hardened pleb is within 8 tiles of a Green, the Green gets -0.5 stress/sec (small but stacks with leader aura).

## Future: Emergent Traits

Not implemented now, but the design supports traits earned from experience:
- "Steady" — survived a rout → break threshold +7
- "Haunted" — lost squad member → +5 baseline stress
- "Inspiring" — rallied 3+ allies → rally effect 1.5x
- "Cold" — 10+ kills → no stress from deaths, allies -2 comfort near them

## Files Changed

| File | Changes |
|------|---------|
| `src/pleb.rs` | `is_leader`, `firefights_survived`, `kills`, `combat_timer`, rank computation |
| `src/morale.rs` | Rank modifier in `apply_stress`, `aim_speed_multiplier`; leader aura in `tick_recovery` |
| `src/comms.rs` | New `ShoutKind::Rally`, `ShoutKind::Advance`, `ShoutKind::FallBack`; command processing |
| `src/simulation.rs` | Firefight tracking, kill counting, leader death stress |
| `src/ui.rs` | Command tile in action bar for leaders |
| `src/input.rs` | Hotkey for command (V) |

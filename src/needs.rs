//! Pleb needs system — hunger, rest, warmth, oxygen, safety, comfort.
//! Each need is 0.0 (desperate) to 1.0 (fully satisfied).
//! Needs decay over time and are replenished by environment/actions.

use crate::grid::{GRID_W, GRID_H, block_type_rs, roof_height_rs};

/// All needs for a single pleb.
#[derive(Clone, Debug)]
pub struct PlebNeeds {
    pub hunger: f32,     // 1.0 = full, decays over time
    pub rest: f32,       // 1.0 = rested, decays faster when moving
    pub warmth: f32,     // 1.0 = comfortable temp, driven by environment
    pub oxygen: f32,     // 1.0 = fresh air, driven by environment
    pub safety: f32,     // 1.0 = safe, drops near fire/outdoors at night
    pub comfort: f32,    // 1.0 = comfy, indoors + furniture
    pub health: f32,     // 1.0 = full health, damaged by unmet needs
    pub mood: f32,       // -100 to +100, aggregate of all needs
}

impl Default for PlebNeeds {
    fn default() -> Self {
        PlebNeeds {
            hunger: 0.9,
            rest: 1.0,
            warmth: 0.8,
            oxygen: 1.0,
            safety: 1.0,
            comfort: 0.5,
            health: 1.0,
            mood: 50.0,
        }
    }
}

/// Environment snapshot at a pleb's position (sampled from CPU-side grid state).
pub struct EnvSample {
    pub is_indoors: bool,
    pub near_fire: bool,      // within 3 blocks of fireplace/campfire
    pub near_heater: bool,    // within 3 blocks of electric heater
    pub near_bed: bool,       // within 2 blocks of bed/sleeping spot
    pub near_furniture: bool, // within 3 blocks of bench/table/chair
    pub is_night: bool,
    pub fire_dist: f32,       // distance to nearest fire (for danger)
}

/// Sample the environment around a pleb position from the CPU grid.
pub fn sample_environment(grid: &[u32], px: f32, py: f32, day_frac: f32) -> EnvSample {
    let bx = px.floor() as i32;
    let by = py.floor() as i32;

    // Check if indoors (has roof)
    let is_indoors = if bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32 {
        let b = grid[(by as u32 * GRID_W + bx as u32) as usize];
        roof_height_rs(b) > 0
    } else {
        false
    };

    let is_night = day_frac < 0.15 || day_frac > 0.85;

    let mut near_fire = false;
    let mut near_heater = false;
    let mut near_bed = false;
    let mut near_furniture = false;
    let mut fire_dist = f32::MAX;

    // Scan nearby blocks (radius 4)
    let scan_r = 4i32;
    for dy in -scan_r..=scan_r {
        for dx in -scan_r..=scan_r {
            let sx = bx + dx;
            let sy = by + dy;
            if sx < 0 || sy < 0 || sx >= GRID_W as i32 || sy >= GRID_H as i32 { continue; }

            let b = grid[(sy as u32 * GRID_W + sx as u32) as usize];
            let bt = block_type_rs(b);
            let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();

            match bt {
                // Fireplace (type 6) or campfire-like
                6 => {
                    if dist < 4.0 { near_fire = true; }
                    if dist < fire_dist { fire_dist = dist; }
                }
                // Electric light (type 7) — treat as heater too
                7 => {
                    if dist < 3.0 { near_heater = true; }
                }
                // Bench (type 13)
                13 => {
                    if dist < 3.0 { near_furniture = true; }
                    if dist < 2.0 { near_bed = true; } // benches serve as rest spots for now
                }
                _ => {}
            }
        }
    }

    EnvSample {
        is_indoors,
        near_fire,
        near_heater,
        near_bed,
        near_furniture,
        is_night,
        fire_dist,
    }
}

// --- Need decay rates (per real second at 1x speed) ---
const HUNGER_DECAY: f32 = 0.003;       // ~5.5 min to starve from full
const REST_DECAY_IDLE: f32 = 0.002;    // ~8 min to exhaust while idle
const REST_DECAY_MOVING: f32 = 0.005;  // ~3.3 min while moving
const REST_RECOVER: f32 = 0.015;       // ~1 min to fully rest near bed

// --- Health damage rates (per real second) ---
const STARVE_DAMAGE: f32 = 0.008;      // ~2 min to die from starvation
const SUFFOCATE_DAMAGE: f32 = 0.025;   // ~40s to die from no oxygen
const FREEZE_DAMAGE: f32 = 0.012;      // ~1.3 min to die from cold
const FIRE_DAMAGE: f32 = 0.04;         // ~25s to die in fire

/// Update a single pleb's needs based on environment. Call once per frame.
/// `dt` = frame delta time, `time_speed` = game speed multiplier.
/// `is_moving` = true if pleb moved this frame.
pub fn tick_needs(needs: &mut PlebNeeds, env: &EnvSample, dt: f32, time_speed: f32, is_moving: bool) {
    let t = dt * time_speed;

    // --- Hunger: always decays ---
    needs.hunger = (needs.hunger - HUNGER_DECAY * t).max(0.0);

    // --- Rest: decays faster when moving, recovers near bed ---
    if env.near_bed && !is_moving {
        needs.rest = (needs.rest + REST_RECOVER * t).min(1.0);
    } else if is_moving {
        needs.rest = (needs.rest - REST_DECAY_MOVING * t).max(0.0);
    } else {
        needs.rest = (needs.rest - REST_DECAY_IDLE * t).max(0.0);
    }

    // --- Warmth: driven by environment ---
    let target_warmth = if env.near_fire || env.near_heater {
        1.0
    } else if env.is_indoors {
        0.7 // insulated but no heat source
    } else if env.is_night {
        0.2 // cold outside at night
    } else {
        0.5 // mild outside during day
    };
    // Smooth approach to target (faster cooling, slower heating)
    let rate = if target_warmth > needs.warmth { 0.3 } else { 0.5 };
    needs.warmth += (target_warmth - needs.warmth) * rate * t;
    needs.warmth = needs.warmth.clamp(0.0, 1.0);

    // --- Oxygen: indoors with fire consumes O2, outdoors always good ---
    if !env.is_indoors {
        // Outdoors: always fresh air
        needs.oxygen = (needs.oxygen + 0.5 * t).min(1.0);
    } else if env.near_fire {
        // Fire in enclosed space depletes oxygen slowly
        needs.oxygen = (needs.oxygen - 0.01 * t).max(0.0);
    } else {
        // Indoors without fire: slow depletion from breathing
        needs.oxygen = (needs.oxygen - 0.002 * t).max(0.0);
    }

    // --- Safety: threatened by fire proximity and being outside at night ---
    let mut safety_target = 1.0f32;
    if env.fire_dist < 2.0 {
        safety_target -= 0.5 * (1.0 - env.fire_dist / 2.0);
    }
    if env.is_night && !env.is_indoors {
        safety_target -= 0.3;
    }
    safety_target = safety_target.max(0.0);
    needs.safety += (safety_target - needs.safety) * 0.4 * t;
    needs.safety = needs.safety.clamp(0.0, 1.0);

    // --- Comfort: indoors + furniture = comfy ---
    let comfort_target = if env.is_indoors && env.near_furniture {
        1.0
    } else if env.is_indoors {
        0.6
    } else {
        0.3
    };
    needs.comfort += (comfort_target - needs.comfort) * 0.2 * t;
    needs.comfort = needs.comfort.clamp(0.0, 1.0);

    // --- Health damage from critical needs ---
    let mut damage = 0.0f32;
    if needs.hunger < 0.05 {
        damage += STARVE_DAMAGE;
    }
    if needs.oxygen < 0.15 {
        damage += SUFFOCATE_DAMAGE * (1.0 - needs.oxygen / 0.15);
    }
    if needs.warmth < 0.15 {
        damage += FREEZE_DAMAGE * (1.0 - needs.warmth / 0.15);
    }
    if env.fire_dist < 1.0 {
        damage += FIRE_DAMAGE;
    }

    // Apply damage
    if damage > 0.0 {
        needs.health = (needs.health - damage * t).max(0.0);
    } else {
        // Slow natural healing when all needs met above 0.5
        if needs.hunger > 0.5 && needs.rest > 0.5 && needs.warmth > 0.5 && needs.oxygen > 0.5 {
            needs.health = (needs.health + 0.002 * t).min(1.0);
        }
    }

    // --- Mood: weighted sum of all needs ---
    let weighted = needs.hunger * 20.0
        + needs.rest * 20.0
        + needs.warmth * 15.0
        + needs.oxygen * 25.0
        + needs.safety * 10.0
        + needs.comfort * 10.0;
    // weighted is 0-100, map to -100..+100
    let target_mood = weighted * 2.0 - 100.0;
    needs.mood += (target_mood - needs.mood) * 0.1 * t;
    needs.mood = needs.mood.clamp(-100.0, 100.0);
}

/// Get a descriptive mood label from mood value.
pub fn mood_label(mood: f32) -> &'static str {
    if mood > 60.0 { "Happy" }
    else if mood > 20.0 { "Content" }
    else if mood > -20.0 { "Neutral" }
    else if mood > -60.0 { "Stressed" }
    else { "Breaking" }
}

/// Get the most critical need (lowest value) for crisis behavior.
pub fn critical_need(needs: &PlebNeeds) -> Option<(&'static str, f32)> {
    let pairs = [
        ("oxygen", needs.oxygen),
        ("warmth", needs.warmth),
        ("hunger", needs.hunger),
        ("rest", needs.rest),
    ];
    pairs.iter()
        .filter(|(_, v)| *v < 0.2)
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|&(name, val)| (name, val))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::make_block;

    fn default_env() -> EnvSample {
        EnvSample {
            is_indoors: false,
            near_fire: false,
            near_heater: false,
            near_bed: false,
            near_furniture: false,
            is_night: false,
            fire_dist: f32::MAX,
        }
    }

    #[test]
    fn test_hunger_decays() {
        let mut needs = PlebNeeds::default();
        let env = default_env();
        let initial = needs.hunger;
        tick_needs(&mut needs, &env, 1.0, 1.0, false);
        assert!(needs.hunger < initial, "hunger should decay");
    }

    #[test]
    fn test_rest_recovers_near_bed() {
        let mut needs = PlebNeeds::default();
        needs.rest = 0.5;
        let mut env = default_env();
        env.near_bed = true;
        tick_needs(&mut needs, &env, 1.0, 1.0, false);
        assert!(needs.rest > 0.5, "rest should recover near bed");
    }

    #[test]
    fn test_warmth_from_fire() {
        let mut needs = PlebNeeds::default();
        needs.warmth = 0.2;
        let mut env = default_env();
        env.near_fire = true;
        // Tick several times to let warmth approach target
        for _ in 0..10 {
            tick_needs(&mut needs, &env, 0.5, 1.0, false);
        }
        assert!(needs.warmth > 0.8, "warmth should rise near fire, got {}", needs.warmth);
    }

    #[test]
    fn test_oxygen_depletes_indoors_with_fire() {
        let mut needs = PlebNeeds::default();
        let mut env = default_env();
        env.is_indoors = true;
        env.near_fire = true;
        let initial = needs.oxygen;
        tick_needs(&mut needs, &env, 1.0, 1.0, false);
        assert!(needs.oxygen < initial, "oxygen should deplete indoors with fire");
    }

    #[test]
    fn test_health_damage_from_starvation() {
        let mut needs = PlebNeeds::default();
        needs.hunger = 0.0;
        let env = default_env();
        let initial = needs.health;
        tick_needs(&mut needs, &env, 1.0, 1.0, false);
        assert!(needs.health < initial, "health should drop when starving");
    }

    #[test]
    fn test_health_heals_when_needs_met() {
        let mut needs = PlebNeeds::default();
        needs.health = 0.8;
        needs.hunger = 0.9;
        needs.rest = 0.9;
        needs.warmth = 0.9;
        needs.oxygen = 1.0;
        let env = default_env();
        tick_needs(&mut needs, &env, 1.0, 1.0, false);
        assert!(needs.health > 0.8, "health should slowly heal");
    }

    #[test]
    fn test_mood_reflects_needs() {
        let mut good = PlebNeeds::default();
        good.hunger = 1.0; good.rest = 1.0; good.warmth = 1.0;
        good.oxygen = 1.0; good.safety = 1.0; good.comfort = 1.0;
        good.mood = 0.0;
        let env = default_env();
        tick_needs(&mut good, &env, 1.0, 1.0, false);
        assert!(good.mood > 0.0, "mood should be positive with all needs met");

        let mut bad = PlebNeeds::default();
        bad.hunger = 0.0; bad.rest = 0.0; bad.warmth = 0.0;
        bad.oxygen = 0.0; bad.safety = 0.0; bad.comfort = 0.0;
        bad.mood = 0.0;
        tick_needs(&mut bad, &env, 1.0, 1.0, false);
        assert!(bad.mood < 0.0, "mood should be negative with no needs met");
    }

    #[test]
    fn test_mood_label() {
        assert_eq!(mood_label(80.0), "Happy");
        assert_eq!(mood_label(30.0), "Content");
        assert_eq!(mood_label(0.0), "Neutral");
        assert_eq!(mood_label(-40.0), "Stressed");
        assert_eq!(mood_label(-80.0), "Breaking");
    }

    #[test]
    fn test_critical_need() {
        let mut needs = PlebNeeds::default();
        assert!(critical_need(&needs).is_none(), "no critical need when all satisfied");

        needs.oxygen = 0.1;
        let crit = critical_need(&needs);
        assert!(crit.is_some());
        assert_eq!(crit.unwrap().0, "oxygen");
    }

    #[test]
    fn test_safety_drops_near_fire() {
        let mut needs = PlebNeeds::default();
        let mut env = default_env();
        env.fire_dist = 1.0;
        for _ in 0..10 {
            tick_needs(&mut needs, &env, 0.5, 1.0, false);
        }
        assert!(needs.safety < 0.9, "safety should drop near fire, got {}", needs.safety);
    }

    #[test]
    fn test_sample_environment_outdoors() {
        let grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize]; // all dirt, no roof
        let env = sample_environment(&grid, 128.5, 128.5, 0.5);
        assert!(!env.is_indoors);
        assert!(!env.near_fire);
        assert!(!env.is_night);
    }
}

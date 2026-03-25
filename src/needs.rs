//! Pleb needs system — hunger, rest, warmth, oxygen, safety, comfort.
//! Each need is 0.0 (desperate) to 1.0 (fully satisfied).
//! Needs decay over time and are replenished by environment/actions.
//!
//! Oxygen is driven by GPU fluid sim readback (O2/CO2 at pleb position).
//! Plebs can hold their breath when air is bad, and flee toward good air.

use crate::grid::*;
use crate::pleb::is_walkable_pos;

// --- Breathing constants (realistic human physiology) ---
/// Maximum breath-hold duration in game seconds (~30 real seconds at 1x)
const BREATH_HOLD_MAX: f32 = 30.0;
/// O2 level below which you can't breathe (even if you try)
const O2_UNBREATHABLE: f32 = 0.12;
/// O2 level below which breathing is labored (partial benefit)
const O2_LABORED: f32 = 0.35;
/// CO2 level above which you must hold your breath (toxic)
const CO2_TOXIC: f32 = 0.25;
/// CO2 level that causes irritation but is still breathable
const CO2_IRRITANT: f32 = 0.10;
/// Health damage rate from suffocation (per game second, breath depleted in bad air)
const SUFFOCATE_DAMAGE: f32 = 0.03;
/// Health damage rate from CO2 poisoning (per game second, breathing toxic CO2)
const CO2_POISON_DAMAGE: f32 = 0.015;
/// Breath recovery rate when breathing good air (seconds of breath per second)
const BREATH_RECOVERY_RATE: f32 = 3.0;

// --- Environment scan radii ---
const ENV_SCAN_RADIUS: i32 = 6;
const NEAR_FIRE_RADIUS: f32 = 4.0;
const NEAR_HEATER_RADIUS: f32 = 3.0;
const NEAR_FURNITURE_RADIUS: f32 = 3.0;
const NEAR_BED_RADIUS: f32 = 2.0;
const NEAR_INTERACT_RADIUS: f32 = 2.0;  // berry bush, crate
const CRATE_SCAN_RADIUS: i32 = 60;

// --- Temperature comfort range (°C) ---
const TEMP_COMFORTABLE_LOW: f32 = 18.0;
const TEMP_COMFORTABLE_HIGH: f32 = 28.0;
const TEMP_COOL_MIN: f32 = 5.0;
const TEMP_HOT_RANGE: f32 = 50.0;        // degrees above comfortable before warmth hits 0.2

// --- Warmth fallback (no fluid sim) ---
const WARMTH_INDOORS: f32 = 0.7;
const WARMTH_INDOORS_FIRE: f32 = 1.0;   // near fireplace indoors
const WARMTH_NIGHT_OUTDOORS: f32 = 0.05; // dangerous — below freeze threshold
const WARMTH_DAY_OUTDOORS: f32 = 0.5;
const WARMTH_DUSK_OUTDOORS: f32 = 0.25;  // transition period

// --- Night/day boundary ---
const NIGHT_START_FRAC: f32 = 0.85;
const NIGHT_END_FRAC: f32 = 0.15;

// --- Safety ---
const FIRE_DANGER_DIST: f32 = 2.0;
const FIRE_LETHAL_DIST: f32 = 1.0;

// --- Comfort ---
const COMFORT_INDOORS_FURNITURE: f32 = 1.0;
const COMFORT_INDOORS: f32 = 0.6;
const COMFORT_OUTDOORS: f32 = 0.3;

// --- Health ---
const NATURAL_HEAL_RATE: f32 = 0.002;
const NATURAL_HEAL_THRESHOLD: f32 = 0.5;
const STARVATION_THRESHOLD: f32 = 0.05;
const FREEZE_THRESHOLD: f32 = 0.15;
const TOXIC_GAS_CO2: f32 = 0.5;
const TOXIC_GAS_O2: f32 = 0.5;
const TOXIC_CONTACT_DAMAGE: f32 = 0.06;

// --- Mood weights ---
const MOOD_HUNGER_WEIGHT: f32 = 18.0;
const MOOD_THIRST_WEIGHT: f32 = 18.0;
const MOOD_REST_WEIGHT: f32 = 18.0;
const MOOD_WARMTH_WEIGHT: f32 = 15.0;
const MOOD_OXYGEN_WEIGHT: f32 = 25.0;
const MOOD_SAFETY_WEIGHT: f32 = 10.0;
const MOOD_COMFORT_WEIGHT: f32 = 10.0;

/// Breathing state machine for a pleb.
#[derive(Clone, Debug, PartialEq)]
pub enum BreathingState {
    /// Breathing normally — good air
    Normal,
    /// Holding breath — CO2 too high or O2 too low to breathe
    HoldingBreath,
    /// Gasping — breath ran out, forced to inhale bad air, taking damage
    Gasping,
}

/// All needs for a single pleb.
#[derive(Clone, Debug)]
pub struct PlebNeeds {
    pub hunger: f32,     // 1.0 = full, decays over time
    pub thirst: f32,     // 1.0 = hydrated, decays over time
    pub rest: f32,       // 1.0 = rested, decays faster when moving
    pub warmth: f32,     // 1.0 = comfortable temp, driven by environment
    pub oxygen: f32,     // 1.0 = fresh air, driven by actual O2/CO2 levels
    pub safety: f32,     // 1.0 = safe, drops near fire/outdoors at night
    pub comfort: f32,    // 1.0 = comfy, indoors + furniture
    pub health: f32,     // 1.0 = full health, damaged by unmet needs
    pub mood: f32,       // -100 to +100, aggregate of all needs
    pub stress: f32,     // 0-100, cumulative stress level

    // Stress tracking
    pub last_task_type: u8,   // for monotony detection (zones::WORK_*)
    pub task_duration: f32,   // seconds on current task type

    // Breathing system
    pub breath_remaining: f32,   // seconds of breath left (0..BREATH_HOLD_MAX)
    pub breathing_state: BreathingState,
    pub air_o2: f32,             // last sampled O2 at position (0-1, from fluid sim)
    pub air_co2: f32,            // last sampled CO2 at position (0-1.5, from fluid sim)
    pub air_temp: f32,           // last sampled temperature at position (°C, from fluid sim)
    pub flee_target: Option<(i32, i32)>, // crisis pathfind target (nearest good air)
}

impl Default for PlebNeeds {
    fn default() -> Self {
        PlebNeeds {
            hunger: 0.9,
            thirst: 0.9,
            rest: 1.0,
            warmth: 0.8,
            oxygen: 1.0,
            safety: 1.0,
            comfort: 0.5,
            health: 1.0,
            mood: 50.0,
            stress: 10.0,
            last_task_type: 255,
            task_duration: 0.0,
            breath_remaining: BREATH_HOLD_MAX,
            breathing_state: BreathingState::Normal,
            air_o2: 1.0,
            air_co2: 0.0,
            air_temp: 20.0,
            flee_target: None,
        }
    }
}

/// Environment snapshot at a pleb's position (sampled from CPU-side grid state).
pub struct EnvSample {
    pub is_indoors: bool,
    pub near_fire: bool,      // within 3 blocks of fireplace/campfire
    pub near_heater: bool,    // within 3 blocks of electric heater
    pub near_bed: bool,       // within 2 blocks of bed (type 30)
    pub near_furniture: bool, // within 3 blocks of bench/table/chair
    pub near_berry_bush: bool, // within 2 blocks of berry bush (type 31)
    pub near_crate: bool,      // within 2 blocks of storage crate (type 33)
    pub nearest_bed: Option<(i32, i32)>,       // coords of nearest bed
    pub nearest_berry_bush: Option<(i32, i32)>, // coords of nearest berry bush
    pub nearest_crate: Option<(i32, i32)>,     // coords of nearest storage crate
    pub is_night: bool,
    pub is_dusk: bool,        // transition period (getting cold)
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

    let is_night = day_frac < NIGHT_END_FRAC || day_frac > NIGHT_START_FRAC;
    let is_dusk = !is_night && (day_frac > 0.75 || day_frac < 0.20); // dusk/dawn transition

    let mut near_fire = false;
    let mut near_heater = false;
    let mut near_bed = false;
    let mut near_furniture = false;
    let mut near_berry_bush = false;
    let mut near_crate = false;
    let mut nearest_bed: Option<(i32, i32)> = None;
    let mut nearest_berry_bush: Option<(i32, i32)> = None;
    let mut nearest_crate: Option<(i32, i32)> = None;
    let mut fire_dist = f32::MAX;
    let mut bed_dist = f32::MAX;
    let mut bush_dist = f32::MAX;
    let mut crate_dist = f32::MAX;

    let scan_r = ENV_SCAN_RADIUS;
    for dy in -scan_r..=scan_r {
        for dx in -scan_r..=scan_r {
            let sx = bx + dx;
            let sy = by + dy;
            if sx < 0 || sy < 0 || sx >= GRID_W as i32 || sy >= GRID_H as i32 { continue; }

            let b = grid[(sy as u32 * GRID_W + sx as u32) as usize];
            let bt = block_type_rs(b);
            let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();

            if bt == BT_FIREPLACE {
                if dist < NEAR_FIRE_RADIUS { near_fire = true; }
                if dist < fire_dist { fire_dist = dist; }
            } else if bt == BT_CEILING_LIGHT {
                if dist < NEAR_HEATER_RADIUS { near_heater = true; }
            } else if bt_is!(bt, BT_BENCH, BT_COMPOST) {
                if dist < NEAR_FURNITURE_RADIUS { near_furniture = true; }
            } else if bt == BT_BED {
                if dist < NEAR_BED_RADIUS { near_bed = true; }
                if dist < NEAR_FURNITURE_RADIUS { near_furniture = true; }
                if dist < bed_dist {
                    bed_dist = dist;
                    nearest_bed = Some((sx, sy));
                }
            } else if bt == BT_BERRY_BUSH {
                if dist < NEAR_INTERACT_RADIUS { near_berry_bush = true; }
                if dist < bush_dist {
                    bush_dist = dist;
                    nearest_berry_bush = Some((sx, sy));
                }
            } else if bt == BT_CRATE {
                if dist < NEAR_INTERACT_RADIUS { near_crate = true; }
                if dist < crate_dist {
                    crate_dist = dist;
                    nearest_crate = Some((sx, sy));
                }
            }
        }
    }

    EnvSample {
        is_indoors,
        near_fire,
        near_heater,
        near_bed,
        near_furniture,
        near_berry_bush,
        near_crate,
        nearest_bed,
        nearest_berry_bush,
        nearest_crate,
        is_night,
        is_dusk,
        fire_dist,
    }
}

/// Air quality data from GPU fluid sim readback (per pleb, per frame).
#[derive(Clone, Debug, Default)]
pub struct AirReadback {
    pub o2: f32,    // 0.0-1.0, atmospheric is ~1.0
    pub co2: f32,   // 0.0-1.5, normal is ~0.0
    pub temp: f32,  // degrees C
    pub smoke: f32, // smoke density
}

// --- Need decay rates (per real second at 1x speed) ---
const HUNGER_DECAY: f32 = 0.003;       // ~5.5 min to starve from full
const THIRST_DECAY: f32 = 0.004;       // ~4 min to dehydrate from full (faster than hunger)
const REST_DECAY_IDLE: f32 = 0.002;    // ~8 min to exhaust while idle
const REST_DECAY_MOVING: f32 = 0.005;  // ~3.3 min while moving
const REST_RECOVER_BED: f32 = 0.02;    // ~50s to fully rest in bed
const REST_RECOVER_BENCH: f32 = 0.008; // ~2 min to fully rest on bench/ground
/// How much hunger one berry restores
pub const BERRY_HUNGER_RESTORE: f32 = 0.20;
/// How much thirst one drink at a well restores
pub const WELL_THIRST_RESTORE: f32 = 0.50;
/// Seconds to drink at a well
pub const WELL_DRINK_TIME: f32 = 4.0;

// --- Stress rates (per real second at 1x speed) ---
const STRESS_HUNGER: f32 = 2.0;        // per min when hunger < 0.3
const STRESS_THIRST: f32 = 3.0;        // per min when thirst < 0.3
const STRESS_FREEZING: f32 = 4.0;      // per min when warmth < 0.15
const STRESS_EXHAUSTION: f32 = 2.0;    // per min when rest < 0.2
const STRESS_GROUND_SLEEP: f32 = 1.0;  // per min when sleeping without bed
const STRESS_UGLY: f32 = 0.5;          // per min, no furniture/floor/roof
const STRESS_MONOTONY: f32 = 0.3;      // per min, same task > 5 min
const STRESS_DAMAGE: f32 = 10.0;       // instant, per hit
const STRESS_DEATH_WITNESS: f32 = 20.0; // instant, saw someone die

const STRESS_RELIEF_EAT: f32 = 5.0;     // instant, ate food
const STRESS_RELIEF_DRINK: f32 = 2.0;   // instant, drank
const STRESS_RELIEF_BED: f32 = 3.0;     // per min, sleeping in bed
const STRESS_RELIEF_BENCH: f32 = 2.0;   // per min, near furniture
const STRESS_RELIEF_FIRE: f32 = 1.0;    // per min, near fireplace
const STRESS_RELIEF_SOCIAL: f32 = 0.5;  // per min, near other pleb
const STRESS_RELIEF_VARIED: f32 = 0.5;  // per min, changed task recently

pub const STRESS_BREAK_THRESHOLD: f32 = 85.0;
pub const STRESS_POST_BREAK: f32 = 50.0; // stress after a mental break

/// Stress level label for UI.
pub fn stress_label(stress: f32) -> &'static str {
    if stress < 25.0 { "Calm" }
    else if stress < 50.0 { "Normal" }
    else if stress < 70.0 { "Stressed" }
    else if stress < 85.0 { "Distressed" }
    else { "Breaking" }
}

/// Work speed multiplier from stress level.
pub fn stress_work_speed(stress: f32) -> f32 {
    if stress < 25.0 { 1.1 }       // calm: bonus
    else if stress < 50.0 { 1.0 }  // normal
    else if stress < 70.0 { 0.85 } // stressed: penalty
    else if stress < 85.0 { 0.7 }  // distressed: major penalty
    else { 0.5 }                    // breaking: barely functioning
}

// --- Health damage rates (per real second) ---
const STARVE_DAMAGE: f32 = 0.008;      // ~2 min to die from starvation
const DEHYDRATE_DAMAGE: f32 = 0.010;  // ~1.7 min to die from dehydration
const DEHYDRATION_THRESHOLD: f32 = 0.05;
const FREEZE_DAMAGE: f32 = 0.012;      // ~1.3 min to die from cold
const FIRE_DAMAGE: f32 = 0.04;         // ~25s to die in fire
const HEAT_DAMAGE: f32 = 0.05;         // ~20s to die from extreme heat (was 0.015)
/// Temperature above which colonists take heat damage
const HEAT_DANGER_TEMP: f32 = 45.0;   // lowered from 50
/// Temperature above which colonists flee (crisis)
pub const HEAT_CRISIS_TEMP: f32 = 40.0; // lowered from 45 — flee earlier

/// Determine whether air is breathable at the pleb's position.
fn air_breathable(o2: f32, co2: f32) -> bool {
    o2 >= O2_UNBREATHABLE && co2 < CO2_TOXIC
}

/// Update a single pleb's needs based on environment. Call once per frame.
/// `dt` = frame delta time, `time_speed` = game speed multiplier.
/// `is_moving` = true if pleb moved this frame.
/// `air` = fluid sim readback (None if readback not available, e.g. WASM).
pub fn tick_needs(needs: &mut PlebNeeds, env: &EnvSample, dt: f32, time_speed: f32, is_moving: bool, is_sleeping: bool, air: Option<&AirReadback>) {
    let t = dt * time_speed;

    // --- Store air readings ---
    if let Some(air) = air {
        needs.air_o2 = air.o2;
        needs.air_co2 = air.co2;
        needs.air_temp = air.temp;
    }

    // --- Hunger: always decays ---
    needs.hunger = (needs.hunger - HUNGER_DECAY * t).max(0.0);

    // --- Thirst: decays faster than hunger ---
    needs.thirst = (needs.thirst - THIRST_DECAY * t).max(0.0);

    // --- Rest: decays faster when moving, recovers in bed (best) or near furniture ---
    if is_sleeping && env.near_bed {
        // Sleeping in a proper bed — best recovery
        let night_bonus = if env.is_night { 1.3 } else { 1.0 }; // slightly better rest at night
        needs.rest = (needs.rest + REST_RECOVER_BED * night_bonus * t).min(1.0);
    } else if env.near_furniture && !is_moving {
        // Resting near furniture (bench) — slow recovery
        needs.rest = (needs.rest + REST_RECOVER_BENCH * t).min(1.0);
    } else if is_moving {
        needs.rest = (needs.rest - REST_DECAY_MOVING * t).max(0.0);
    } else {
        needs.rest = (needs.rest - REST_DECAY_IDLE * t).max(0.0);
    }

    // --- Warmth: driven by fluid sim temperature when available ---
    let target_warmth = if let Some(air) = air {
        let temp = air.temp;
        if temp >= TEMP_COMFORTABLE_LOW && temp <= TEMP_COMFORTABLE_HIGH {
            1.0
        } else if temp > TEMP_COMFORTABLE_HIGH {
            (1.0 - (temp - TEMP_COMFORTABLE_HIGH) / TEMP_HOT_RANGE).max(0.2)
        } else if temp >= TEMP_COOL_MIN {
            (temp - TEMP_COOL_MIN) / (TEMP_COMFORTABLE_LOW - TEMP_COOL_MIN)
        } else {
            0.0
        }
    } else {
        // Fallback: grid-based approximation
        if env.near_fire || env.near_heater {
            if env.is_indoors { WARMTH_INDOORS_FIRE } else { 0.6 } // fire outdoors helps but not as much
        } else if env.is_indoors {
            WARMTH_INDOORS
        } else if env.is_night {
            WARMTH_NIGHT_OUTDOORS // dangerous cold
        } else if env.is_dusk {
            WARMTH_DUSK_OUTDOORS  // getting cold
        } else {
            WARMTH_DAY_OUTDOORS
        }
    };
    // Warming is slow, cooling is fast (hypothermia sets in quickly)
    let rate = if target_warmth > needs.warmth { 0.2 } else { 0.6 };
    needs.warmth += (target_warmth - needs.warmth) * rate * t;
    needs.warmth = needs.warmth.clamp(0.0, 1.0);

    // --- Breathing system (O2/CO2 from fluid sim) ---
    let (o2, co2) = if air.is_some() {
        (needs.air_o2, needs.air_co2)
    } else {
        // Fallback: estimate from grid
        let o2 = if !env.is_indoors { 1.0 }
            else if env.near_fire { 0.6 }
            else { 0.9 };
        let co2 = if env.is_indoors && env.near_fire { 0.15 } else { 0.0 };
        (o2, co2)
    };

    let can_breathe = air_breathable(o2, co2);

    match needs.breathing_state {
        BreathingState::Normal => {
            if can_breathe {
                // Good air — recover breath, replenish O2 need
                needs.breath_remaining = (needs.breath_remaining + BREATH_RECOVERY_RATE * t).min(BREATH_HOLD_MAX);

                // O2 need based on air quality
                let o2_quality = if o2 > O2_LABORED && co2 < CO2_IRRITANT {
                    1.0 // perfect air
                } else {
                    // Labored breathing — partial benefit
                    let o2_factor = ((o2 - O2_UNBREATHABLE) / (O2_LABORED - O2_UNBREATHABLE)).clamp(0.0, 1.0);
                    let co2_factor = (1.0 - (co2 - CO2_IRRITANT) / (CO2_TOXIC - CO2_IRRITANT)).clamp(0.0, 1.0);
                    o2_factor * co2_factor * 0.7
                };
                needs.oxygen = (needs.oxygen + o2_quality * 0.5 * t).min(1.0);

                // Transition to holding breath if air becomes bad
                if !can_breathe {
                    needs.breathing_state = BreathingState::HoldingBreath;
                    needs.flee_target = None; // will be computed in crisis behavior
                }
            } else {
                // Air just went bad — hold breath
                needs.breathing_state = BreathingState::HoldingBreath;
                needs.flee_target = None;
            }
        }
        BreathingState::HoldingBreath => {
            // Breath depletes — faster if moving (exertion)
            let depletion = if is_moving { 2.0 } else { 1.0 };
            needs.breath_remaining = (needs.breath_remaining - depletion * t).max(0.0);

            // O2 need slowly drops while holding breath
            needs.oxygen = (needs.oxygen - 0.02 * t).max(0.0);

            if can_breathe {
                // Air is good again — resume breathing
                needs.breathing_state = BreathingState::Normal;
                needs.flee_target = None;
            } else if needs.breath_remaining <= 0.0 {
                // Can't hold any longer — forced to gasp
                needs.breathing_state = BreathingState::Gasping;
            }
        }
        BreathingState::Gasping => {
            // Forced to inhale bad air — taking damage
            needs.oxygen = (needs.oxygen - 0.08 * t).max(0.0);

            // Periodic gasps — can get small amounts of whatever O2 is available
            if o2 > 0.05 {
                needs.breath_remaining = (needs.breath_remaining + o2 * 0.5 * t).min(3.0);
                // Brief respite allows another short hold
                if needs.breath_remaining > 2.0 && !can_breathe {
                    needs.breathing_state = BreathingState::HoldingBreath;
                }
            }

            if can_breathe {
                needs.breathing_state = BreathingState::Normal;
                needs.flee_target = None;
            }
        }
    }

    // --- Safety: threatened by fire proximity and being outside at night ---
    let mut safety_target = 1.0f32;
    if env.fire_dist < FIRE_DANGER_DIST {
        safety_target -= 0.5 * (1.0 - env.fire_dist / FIRE_DANGER_DIST);
    }
    if env.is_night && !env.is_indoors {
        safety_target -= 0.3;
    }
    // Bad air reduces safety feeling
    if needs.breathing_state != BreathingState::Normal {
        safety_target -= 0.4;
    }
    // High temperature reduces safety
    if needs.air_temp > HEAT_CRISIS_TEMP {
        safety_target -= 0.5 * ((needs.air_temp - HEAT_CRISIS_TEMP) / 30.0).min(1.0);
    }
    safety_target = safety_target.max(0.0);
    needs.safety += (safety_target - needs.safety) * 0.4 * t;
    needs.safety = needs.safety.clamp(0.0, 1.0);

    // --- Comfort: indoors + furniture = comfy ---
    let comfort_target = if env.is_indoors && env.near_furniture {
        COMFORT_INDOORS_FURNITURE
    } else if env.is_indoors {
        COMFORT_INDOORS
    } else {
        COMFORT_OUTDOORS
    };
    needs.comfort += (comfort_target - needs.comfort) * 0.2 * t;
    needs.comfort = needs.comfort.clamp(0.0, 1.0);

    // --- Health damage from critical needs ---
    let mut damage = 0.0f32;
    if needs.hunger < STARVATION_THRESHOLD {
        damage += STARVE_DAMAGE;
    }
    if needs.thirst < DEHYDRATION_THRESHOLD {
        damage += DEHYDRATE_DAMAGE;
    }

    // Suffocation: gasping in bad air
    if needs.breathing_state == BreathingState::Gasping {
        if o2 < O2_UNBREATHABLE {
            damage += SUFFOCATE_DAMAGE * (1.0 - o2 / O2_UNBREATHABLE);
        }
        if co2 > CO2_TOXIC {
            damage += CO2_POISON_DAMAGE * ((co2 - CO2_TOXIC) / 0.5).min(1.0);
        }
    }

    // Toxic gas: when low O2 AND high CO2 simultaneously (grenade cloud).
    // Damages even while holding breath (skin/eye contact).
    if needs.air_co2 > TOXIC_GAS_CO2 && needs.air_o2 < TOXIC_GAS_O2 {
        let toxic_str = ((needs.air_co2 - TOXIC_GAS_CO2) * 2.0).min(1.0);
        damage += TOXIC_CONTACT_DAMAGE * toxic_str;
    }

    if needs.warmth < FREEZE_THRESHOLD {
        damage += FREEZE_DAMAGE * (1.0 - needs.warmth / FREEZE_THRESHOLD);
    }
    // Heat damage: scales with temperature above danger threshold
    if needs.air_temp > HEAT_DANGER_TEMP {
        let heat_severity = ((needs.air_temp - HEAT_DANGER_TEMP) / 50.0).min(1.0);
        damage += HEAT_DAMAGE * heat_severity;
    }
    if env.fire_dist < FIRE_LETHAL_DIST {
        damage += FIRE_DAMAGE;
    }

    // Apply damage
    if damage > 0.0 {
        needs.health = (needs.health - damage * t).max(0.0);
    } else {
        // Slow natural healing when all needs met above 0.5
        if needs.hunger > NATURAL_HEAL_THRESHOLD && needs.rest > NATURAL_HEAL_THRESHOLD && needs.warmth > NATURAL_HEAL_THRESHOLD && needs.oxygen > NATURAL_HEAL_THRESHOLD {
            needs.health = (needs.health + NATURAL_HEAL_RATE * t).min(1.0);
        }
    }

    // --- Mood: weighted sum of all needs ---
    let weighted = needs.hunger * MOOD_HUNGER_WEIGHT
        + needs.thirst * MOOD_THIRST_WEIGHT
        + needs.rest * MOOD_REST_WEIGHT
        + needs.warmth * MOOD_WARMTH_WEIGHT
        + needs.oxygen * MOOD_OXYGEN_WEIGHT
        + needs.safety * MOOD_SAFETY_WEIGHT
        + needs.comfort * MOOD_COMFORT_WEIGHT;
    let target_mood = weighted * 2.0 - 100.0;
    needs.mood += (target_mood - needs.mood) * 0.1 * t;
    needs.mood = needs.mood.clamp(-100.0, 100.0);

    // --- Stress: cumulative, rises from bad conditions, falls from good ones ---
    let t_min = t / 60.0; // convert to per-minute rate
    let mut stress_delta: f32 = 0.0;

    // Stress sources
    if needs.hunger < 0.3 { stress_delta += STRESS_HUNGER * t_min; }
    if needs.thirst < 0.3 { stress_delta += STRESS_THIRST * t_min; }
    if needs.warmth < FREEZE_THRESHOLD { stress_delta += STRESS_FREEZING * t_min; }
    if needs.rest < 0.2 { stress_delta += STRESS_EXHAUSTION * t_min; }
    if is_sleeping && !env.near_bed { stress_delta += STRESS_GROUND_SLEEP * t_min; }
    if !env.is_indoors && !env.near_furniture { stress_delta += STRESS_UGLY * t_min; }
    if needs.task_duration > 300.0 { stress_delta += STRESS_MONOTONY * t_min; } // 5+ min same task

    // Stress relief
    if is_sleeping && env.near_bed { stress_delta -= STRESS_RELIEF_BED * t_min; }
    if env.near_furniture { stress_delta -= STRESS_RELIEF_BENCH * t_min; }
    if env.near_fire { stress_delta -= STRESS_RELIEF_FIRE * t_min; }
    if needs.task_duration < 60.0 && needs.last_task_type != 255 { stress_delta -= STRESS_RELIEF_VARIED * t_min; }

    // Natural baseline decay (always trends toward 20 slowly)
    stress_delta += (20.0 - needs.stress) * 0.01 * t_min;

    needs.stress = (needs.stress + stress_delta).clamp(0.0, 100.0);
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
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|&(name, val)| (name, val))
}

/// Breathing state label for UI display.
pub fn breathing_label(state: &BreathingState) -> &'static str {
    match state {
        BreathingState::Normal => "Breathing",
        BreathingState::HoldingBreath => "Holding breath",
        BreathingState::Gasping => "GASPING!",
    }
}

/// Find the nearest storage crate by scanning a large radius from (bx, by).
pub fn find_nearest_crate(grid: &[u32], bx: i32, by: i32) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32)> = None;
    let mut best_dist = f32::MAX;
    let scan_r = CRATE_SCAN_RADIUS;
    for dy in -scan_r..=scan_r {
        for dx in -scan_r..=scan_r {
            let sx = bx + dx;
            let sy = by + dy;
            if sx < 0 || sy < 0 || sx >= GRID_W as i32 || sy >= GRID_H as i32 { continue; }
            let b = grid[(sy as u32 * GRID_W + sx as u32) as usize];
            if block_type_rs(b) == BT_CRATE {
                let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
                if dist < best_dist {
                    best_dist = dist;
                    best = Some((sx, sy));
                }
            }
        }
    }
    best
}

/// Find the nearest well by scanning a radius from (bx, by).
pub fn find_nearest_well(grid: &[u32], bx: i32, by: i32) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32)> = None;
    let mut best_dist = f32::MAX;
    let scan_r = CRATE_SCAN_RADIUS;
    for dy in -scan_r..=scan_r {
        for dx in -scan_r..=scan_r {
            let sx = bx + dx;
            let sy = by + dy;
            if sx < 0 || sy < 0 || sx >= GRID_W as i32 || sy >= GRID_H as i32 { continue; }
            let b = grid[(sy as u32 * GRID_W + sx as u32) as usize];
            if block_type_rs(b) == BT_WELL {
                let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
                if dist < best_dist {
                    best_dist = dist;
                    best = Some((sx, sy));
                }
            }
        }
    }
    best
}

/// Find the nearest tile that is cooler (outdoors / no roof = likely cooler).
/// Used for heat crisis flee behavior.
pub fn find_cool_tile(grid: &[u32], bx: i32, by: i32, max_radius: i32) -> Option<(i32, i32)> {
    // Start at radius 3 to avoid pathing to an equally hot adjacent tile
    for r in 3..=max_radius {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r { continue; }
                let sx = bx + dx;
                let sy = by + dy;
                if sx < 0 || sy < 0 || sx >= GRID_W as i32 || sy >= GRID_H as i32 { continue; }

                let b = grid[(sy as u32 * GRID_W + sx as u32) as usize];
                // Outdoors (no roof) = likely cooler, and walkable
                if roof_height_rs(b) == 0 && is_walkable_pos(grid, sx as f32 + 0.5, sy as f32 + 0.5) {
                    // Also check no fire nearby
                    let bt = block_type_rs(b);
                    if bt != BT_FIREPLACE { // not a fireplace
                        return Some((sx, sy));
                    }
                }
            }
        }
    }
    None
}

/// Find the nearest tile with breathable air by scanning outward from (bx, by).
/// Returns grid coords of nearest breathable tile, or None if none found within radius.
/// Uses a simple distance-ordered search (not A* — just finds the target).
pub fn find_breathable_tile(grid: &[u32], bx: i32, by: i32, max_radius: i32) -> Option<(i32, i32)> {
    // Search in expanding rings
    for r in 1..=max_radius {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r { continue; } // only ring perimeter
                let sx = bx + dx;
                let sy = by + dy;
                if sx < 0 || sy < 0 || sx >= GRID_W as i32 || sy >= GRID_H as i32 { continue; }

                let b = grid[(sy as u32 * GRID_W + sx as u32) as usize];
                // Outdoors (no roof) = guaranteed breathable air
                if roof_height_rs(b) == 0 && is_walkable_pos(grid, sx as f32 + 0.5, sy as f32 + 0.5) {
                    return Some((sx, sy));
                }
            }
        }
    }
    None
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
            near_berry_bush: false,
            near_crate: false,
            nearest_bed: None,
            nearest_berry_bush: None,
            nearest_crate: None,
            is_night: false,
            is_dusk: false,
            fire_dist: f32::MAX,
        }
    }

    fn good_air() -> AirReadback {
        AirReadback { o2: 1.0, co2: 0.0, temp: 20.0, smoke: 0.0 }
    }

    fn toxic_air() -> AirReadback {
        AirReadback { o2: 0.08, co2: 0.5, temp: 40.0, smoke: 1.0 }
    }

    fn high_co2_air() -> AirReadback {
        AirReadback { o2: 0.8, co2: 0.4, temp: 22.0, smoke: 0.2 }
    }

    #[test]
    fn test_hunger_decays() {
        let mut needs = PlebNeeds::default();
        let env = default_env();
        let initial = needs.hunger;
        tick_needs(&mut needs, &env, 1.0, 1.0, false, false, Some(&good_air()));
        assert!(needs.hunger < initial, "hunger should decay");
    }

    #[test]
    fn test_rest_recovers_in_bed() {
        let mut needs = PlebNeeds::default();
        needs.rest = 0.5;
        let mut env = default_env();
        env.near_bed = true;
        // Sleeping in bed — best recovery
        tick_needs(&mut needs, &env, 1.0, 1.0, false, true, Some(&good_air()));
        assert!(needs.rest > 0.5, "rest should recover when sleeping in bed");
    }

    #[test]
    fn test_rest_recovers_near_furniture() {
        let mut needs = PlebNeeds::default();
        needs.rest = 0.5;
        let mut env = default_env();
        env.near_furniture = true;
        tick_needs(&mut needs, &env, 1.0, 1.0, false, false, Some(&good_air()));
        assert!(needs.rest > 0.5, "rest should recover near furniture");
    }

    #[test]
    fn test_warmth_from_temperature() {
        let mut needs = PlebNeeds::default();
        needs.warmth = 0.2;
        let env = default_env();
        let warm_air = AirReadback { o2: 1.0, co2: 0.0, temp: 22.0, smoke: 0.0 };
        for _ in 0..10 {
            tick_needs(&mut needs, &env, 0.5, 1.0, false, false, Some(&warm_air));
        }
        assert!(needs.warmth > 0.6, "warmth should rise in warm air, got {}", needs.warmth);
    }

    #[test]
    fn test_warmth_drops_in_cold() {
        let mut needs = PlebNeeds::default();
        needs.warmth = 1.0;
        let env = default_env();
        let cold_air = AirReadback { o2: 1.0, co2: 0.0, temp: 0.0, smoke: 0.0 };
        for _ in 0..20 {
            tick_needs(&mut needs, &env, 0.5, 1.0, false, false, Some(&cold_air));
        }
        assert!(needs.warmth < 0.2, "warmth should drop in freezing air, got {}", needs.warmth);
    }

    #[test]
    fn test_breathing_normal_in_good_air() {
        let mut needs = PlebNeeds::default();
        let env = default_env();
        tick_needs(&mut needs, &env, 1.0, 1.0, false, false, Some(&good_air()));
        assert_eq!(needs.breathing_state, BreathingState::Normal);
        assert!(needs.oxygen > 0.9);
    }

    #[test]
    fn test_hold_breath_in_co2() {
        let mut needs = PlebNeeds::default();
        let env = default_env();
        // Expose to high CO2 — should trigger breath hold
        tick_needs(&mut needs, &env, 1.0, 1.0, false, false, Some(&high_co2_air()));
        assert_eq!(needs.breathing_state, BreathingState::HoldingBreath,
            "should hold breath when CO2 > toxic threshold");
    }

    #[test]
    fn test_breath_depletes_while_holding() {
        let mut needs = PlebNeeds::default();
        needs.breathing_state = BreathingState::HoldingBreath;
        let env = default_env();
        let initial_breath = needs.breath_remaining;
        tick_needs(&mut needs, &env, 5.0, 1.0, false, false, Some(&toxic_air()));
        assert!(needs.breath_remaining < initial_breath, "breath should deplete while holding");
    }

    #[test]
    fn test_breath_depletes_faster_when_moving() {
        let mut needs1 = PlebNeeds::default();
        needs1.breathing_state = BreathingState::HoldingBreath;
        let mut needs2 = PlebNeeds::default();
        needs2.breathing_state = BreathingState::HoldingBreath;
        let env = default_env();

        tick_needs(&mut needs1, &env, 5.0, 1.0, false, false, Some(&toxic_air()));
        tick_needs(&mut needs2, &env, 5.0, 1.0, true, false, Some(&toxic_air()));
        assert!(needs2.breath_remaining < needs1.breath_remaining,
            "breath should deplete faster when moving (running uses more O2)");
    }

    #[test]
    fn test_gasping_when_breath_runs_out() {
        let mut needs = PlebNeeds::default();
        needs.breathing_state = BreathingState::HoldingBreath;
        needs.breath_remaining = 0.5;
        let env = default_env();
        tick_needs(&mut needs, &env, 1.0, 1.0, false, false, Some(&toxic_air()));
        assert_eq!(needs.breathing_state, BreathingState::Gasping,
            "should start gasping when breath runs out");
    }

    #[test]
    fn test_gasping_causes_health_damage() {
        let mut needs = PlebNeeds::default();
        needs.breathing_state = BreathingState::Gasping;
        needs.breath_remaining = 0.0;
        let env = default_env();
        let initial_health = needs.health;
        tick_needs(&mut needs, &env, 1.0, 1.0, false, false, Some(&toxic_air()));
        assert!(needs.health < initial_health,
            "gasping in toxic air should damage health");
    }

    #[test]
    fn test_resume_breathing_when_air_clears() {
        let mut needs = PlebNeeds::default();
        needs.breathing_state = BreathingState::HoldingBreath;
        needs.breath_remaining = 10.0;
        let env = default_env();
        tick_needs(&mut needs, &env, 1.0, 1.0, false, false, Some(&good_air()));
        assert_eq!(needs.breathing_state, BreathingState::Normal,
            "should resume breathing when air becomes good");
    }

    #[test]
    fn test_breath_recovers_in_good_air() {
        let mut needs = PlebNeeds::default();
        needs.breath_remaining = 10.0;
        let env = default_env();
        tick_needs(&mut needs, &env, 2.0, 1.0, false, false, Some(&good_air()));
        assert!(needs.breath_remaining > 10.0,
            "breath should recover in good air, got {}", needs.breath_remaining);
    }

    #[test]
    fn test_health_damage_from_starvation() {
        let mut needs = PlebNeeds::default();
        needs.hunger = 0.0;
        let env = default_env();
        let initial = needs.health;
        tick_needs(&mut needs, &env, 1.0, 1.0, false, false, Some(&good_air()));
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
        tick_needs(&mut needs, &env, 1.0, 1.0, false, false, Some(&good_air()));
        assert!(needs.health > 0.8, "health should slowly heal");
    }

    #[test]
    fn test_mood_reflects_needs() {
        let mut good = PlebNeeds::default();
        good.hunger = 1.0; good.rest = 1.0; good.warmth = 1.0;
        good.oxygen = 1.0; good.safety = 1.0; good.comfort = 1.0;
        good.mood = 0.0;
        let env = default_env();
        tick_needs(&mut good, &env, 1.0, 1.0, false, false, Some(&good_air()));
        assert!(good.mood > 0.0, "mood should be positive with all needs met");

        let mut bad = PlebNeeds::default();
        bad.hunger = 0.0; bad.rest = 0.0; bad.warmth = 0.0;
        bad.oxygen = 0.0; bad.safety = 0.0; bad.comfort = 0.0;
        bad.mood = 0.0;
        tick_needs(&mut bad, &env, 1.0, 1.0, false, false, Some(&good_air()));
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
            tick_needs(&mut needs, &env, 0.5, 1.0, false, false, Some(&good_air()));
        }
        assert!(needs.safety < 0.9, "safety should drop near fire, got {}", needs.safety);
    }

    #[test]
    fn test_safety_drops_when_holding_breath() {
        let mut needs = PlebNeeds::default();
        let env = default_env();
        // Tick in toxic air to trigger breath hold, then measure safety
        for _ in 0..5 {
            tick_needs(&mut needs, &env, 1.0, 1.0, false, false, Some(&high_co2_air()));
        }
        assert!(needs.safety < 0.8, "safety should drop when holding breath, got {}", needs.safety);
    }

    #[test]
    fn test_sample_environment_outdoors() {
        let grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize]; // all dirt, no roof
        let env = sample_environment(&grid, 128.5, 128.5, 0.5);
        assert!(!env.is_indoors);
        assert!(!env.near_fire);
        assert!(!env.is_night);
    }

    #[test]
    fn test_find_breathable_tile() {
        let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
        // Make a small indoor area (3x3 with roof)
        for y in 10..13 {
            for x in 10..13 {
                let idx = (y * GRID_W + x) as usize;
                // Set roof height to 3 (bits 24-31)
                grid[idx] = make_block(2, 0, 0) | (3 << 24);
            }
        }
        // Standing inside at (11, 11), should find outdoor tile nearby
        let result = find_breathable_tile(&grid, 11, 11, 10);
        assert!(result.is_some(), "should find a breathable tile");
        let (rx, ry) = result.unwrap();
        // Should be just outside the indoor area
        let dist = ((rx - 11) as f32).hypot((ry - 11) as f32);
        assert!(dist <= 3.0, "breathable tile should be nearby, dist={}", dist);
    }

    #[test]
    fn test_sleep_better_at_night() {
        let mut day_needs = PlebNeeds::default();
        day_needs.rest = 0.5;
        let mut night_needs = PlebNeeds::default();
        night_needs.rest = 0.5;

        let mut day_env = default_env();
        day_env.near_bed = true;
        day_env.is_night = false;

        let mut night_env = default_env();
        night_env.near_bed = true;
        night_env.is_night = true;

        tick_needs(&mut day_needs, &day_env, 1.0, 1.0, false, true, Some(&good_air()));
        tick_needs(&mut night_needs, &night_env, 1.0, 1.0, false, true, Some(&good_air()));

        assert!(night_needs.rest > day_needs.rest,
            "night sleep should recover faster: night={:.4} day={:.4}", night_needs.rest, day_needs.rest);
    }

    #[test]
    fn test_o2_need_rises_in_good_air() {
        let mut needs = PlebNeeds::default();
        needs.oxygen = 0.3; // low from previous bad air
        let env = default_env();
        for _ in 0..5 {
            tick_needs(&mut needs, &env, 1.0, 1.0, false, false, Some(&good_air()));
        }
        assert!(needs.oxygen > 0.8, "O2 need should recover in good air, got {}", needs.oxygen);
    }

    #[test]
    fn test_air_breathable() {
        assert!(air_breathable(1.0, 0.0));
        assert!(air_breathable(0.5, 0.1));
        assert!(!air_breathable(0.05, 0.0)); // too little O2
        assert!(!air_breathable(1.0, 0.3));  // too much CO2
        assert!(!air_breathable(0.05, 0.5)); // both bad
    }
}

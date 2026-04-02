//! Combat morale system — non-linear stress dynamics for firefights.
//!
//! Stress accumulates with a vulnerability spiral (stressed plebs crack faster),
//! recovers logarithmically (deep stress lingers), and combat effects follow an
//! S-curve (cliff around stress ~45, not a linear slide).
//!
//! See docs/ideas/group-ai-roadmap.md §3 (Morale Contagion) for design context.

use crate::pleb::Pleb;

// --- Tuning Constants ---

// Stress gains from combat events (base values, before vulnerability multiplier)
pub const STRESS_WOUNDED: f32 = 20.0; // self took damage (health band worsened)
pub const STRESS_ALLY_WOUNDED: f32 = 12.0; // heard Help shout from ally
pub const STRESS_ALLY_DIED: f32 = 25.0; // nearby ally died
pub const STRESS_SUPPRESSED_PER_SEC: f32 = 3.0; // per second under suppression > 0.3
pub const STRESS_EXPLOSION_NEARBY: f32 = 30.0; // grenade/explosion within 6 tiles
pub const STRESS_ALLY_PANICKING: f32 = 10.0; // nearby ally stress > 85 (contagion)

// Stress relief from combat events
pub const RELIEF_ENEMY_KILLED: f32 = 5.0; // enemy died nearby
pub const RELIEF_CLEAR_SHOUT: f32 = 8.0; // heard "Clear!" shout
pub const RELIEF_COVER_PER_SEC: f32 = 2.0; // per second behind cover, not suppressed
pub const RELIEF_ALLIES_NEARBY_PER_SEC: f32 = 1.0; // per second with 2+ allies in 8 tiles

// Vulnerability spiral: stress gains multiply by (1 + stress²/SPIRAL_DIVISOR)
pub const SPIRAL_DIVISOR: f32 = 10000.0;

// Leader/rank aura recovery bonuses (per second)
pub const LEADER_AURA_PER_SEC: f32 = 1.0; // near a leader
pub const HARDENED_CALM_PER_SEC: f32 = 0.5; // hardened veteran calms nearby Greens
pub const LEADER_DEATH_STRESS: f32 = 30.0; // stress when leader dies nearby
pub const RALLY_RELIEF: f32 = 18.0; // stress relief from Rally command
pub const COMMAND_COOLDOWN: f32 = 8.0; // seconds between command shouts

// Recovery drag: decay = BASE_DECAY / (1 + stress * RECOVERY_DRAG)
pub const BASE_DECAY: f32 = 3.0; // base stress decay per second (in safe conditions)
pub const RECOVERY_DRAG: f32 = 0.02; // higher = slower recovery when deeply stressed

// S-curve effect parameters
const SIGMOID_STEEPNESS: f32 = 4.5; // higher = sharper cliff
const SIGMOID_ONSET: f32 = 30.0; // stress below this has near-zero penalty
const SIGMOID_RANGE: f32 = 35.0; // width of the cliff zone (50% point at onset + range/2 ≈ 47.5)

// Breaking point
pub const BREAK_THRESHOLD: f32 = 85.0; // stress above this → flee/panic

/// Apply a stress event to a pleb, accounting for the vulnerability spiral and rank.
/// Higher current stress → larger effective gain. Green plebs crack faster.
pub fn apply_stress(pleb: &mut Pleb, base_amount: f32) {
    let vulnerability = 1.0 + (pleb.needs.stress * pleb.needs.stress) / SPIRAL_DIVISOR;
    let rank_mod = pleb.rank().stress_modifier();
    pleb.needs.stress =
        (pleb.needs.stress + base_amount * vulnerability * rank_mod).clamp(0.0, 100.0);
}

/// Apply stress relief (not affected by vulnerability spiral).
pub fn apply_relief(pleb: &mut Pleb, amount: f32) {
    pleb.needs.stress = (pleb.needs.stress - amount).max(0.0);
}

/// Tick stress recovery. Call once per frame.
/// Decay is slower when stress is high (deep stress lingers).
pub fn tick_recovery(
    pleb: &mut Pleb,
    dt: f32,
    in_cover: bool,
    allies_nearby: u32,
    near_leader: bool,
    near_hardened: bool,
) {
    // Base recovery (only when not under active suppression)
    if pleb.suppression < 0.3 {
        let stress = pleb.needs.stress;
        let drag = 1.0 + stress * RECOVERY_DRAG;
        let mut rate = BASE_DECAY / drag;

        // Bonus recovery in cover
        if in_cover {
            rate += RELIEF_COVER_PER_SEC;
        }
        // Bonus recovery near allies
        if allies_nearby >= 2 {
            rate += RELIEF_ALLIES_NEARBY_PER_SEC;
        }
        // Leader command aura: passive stress recovery boost
        if near_leader {
            rate += LEADER_AURA_PER_SEC;
        }
        // Hardened veterans calm nearby Greens
        if near_hardened && pleb.rank() == crate::pleb::CombatRank::Green {
            rate += HARDENED_CALM_PER_SEC;
        }

        pleb.needs.stress = (pleb.needs.stress - rate * dt).max(0.0);
    }
}

/// Tick suppression-driven stress accumulation. Call once per frame during combat.
pub fn tick_suppression_stress(pleb: &mut Pleb, dt: f32) {
    if pleb.suppression > 0.3 {
        apply_stress(pleb, STRESS_SUPPRESSED_PER_SEC * dt);
    }
}

/// Combat penalty from stress: 0.0 = no penalty, 1.0 = maximum.
/// Follows an S-curve with a cliff around stress ~45.
pub fn combat_penalty(stress: f32) -> f32 {
    let t = (stress - SIGMOID_ONSET) / SIGMOID_RANGE;
    let raw = 1.0 / (1.0 + (-t * SIGMOID_STEEPNESS).exp());
    raw.clamp(0.0, 1.0)
}

/// Spread multiplier from stress (for ranged accuracy).
/// Returns 1.0 at no stress, up to ~2.5 at max stress.
pub fn spread_multiplier(stress: f32) -> f32 {
    1.0 + combat_penalty(stress) * 1.5
}

/// Aim speed multiplier from stress (slower aiming when stressed).
/// Returns 1.0 at no stress, down to ~0.5 at max stress.
pub fn aim_speed_multiplier(stress: f32) -> f32 {
    1.0 - combat_penalty(stress) * 0.5
}

/// Whether the pleb should break and flee.
pub fn should_break(pleb: &Pleb) -> bool {
    pleb.needs.stress >= BREAK_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigmoid_curve_shape() {
        // Low stress: small penalty (not zero — sigmoid has tails)
        assert!(
            combat_penalty(10.0) < 0.15,
            "at 10: {}",
            combat_penalty(10.0)
        );
        assert!(
            combat_penalty(20.0) < 0.25,
            "at 20: {}",
            combat_penalty(20.0)
        );

        // At sigmoid center (onset where t=0): exactly 0.5
        let mid = combat_penalty(SIGMOID_ONSET);
        assert!(
            mid > 0.45 && mid < 0.55,
            "midpoint at {} was {}",
            SIGMOID_ONSET,
            mid
        );

        // Above cliff: near 1.0
        assert!(
            combat_penalty(70.0) > 0.85,
            "at 70: {}",
            combat_penalty(70.0)
        );
        assert!(
            combat_penalty(85.0) > 0.9,
            "at 85: {}",
            combat_penalty(85.0)
        );
    }

    #[test]
    fn vulnerability_spiral() {
        let mut calm = Pleb::new(0, "A".into(), 0.0, 0.0, 0);
        calm.needs.stress = 10.0;

        let mut stressed = Pleb::new(1, "B".into(), 0.0, 0.0, 1);
        stressed.needs.stress = 70.0;

        let before_calm = calm.needs.stress;
        let before_stressed = stressed.needs.stress;
        apply_stress(&mut calm, 10.0);
        apply_stress(&mut stressed, 10.0);

        let gain_calm = calm.needs.stress - before_calm;
        let gain_stressed = stressed.needs.stress - before_stressed;

        // Stressed pleb should gain MORE from the same base event
        assert!(
            gain_stressed > gain_calm * 1.3,
            "calm gained {}, stressed gained {}",
            gain_calm,
            gain_stressed
        );
    }

    #[test]
    fn recovery_slower_when_stressed() {
        let mut mild = Pleb::new(0, "A".into(), 0.0, 0.0, 0);
        mild.needs.stress = 20.0;

        let mut deep = Pleb::new(1, "B".into(), 0.0, 0.0, 1);
        deep.needs.stress = 80.0;

        let before_mild = mild.needs.stress;
        let before_deep = deep.needs.stress;
        tick_recovery(&mut mild, 1.0, false, 0, false, false);
        tick_recovery(&mut deep, 1.0, false, 0, false, false);

        let decay_mild = before_mild - mild.needs.stress;
        let decay_deep = before_deep - deep.needs.stress;

        // Mild stress should recover faster per second
        assert!(
            decay_mild > decay_deep,
            "mild decayed {}, deep decayed {}",
            decay_mild,
            decay_deep
        );
    }
}

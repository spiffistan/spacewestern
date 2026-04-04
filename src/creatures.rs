//! Alien fauna: lightweight creature system separate from plebs.

use crate::creature_defs::CreatureRegistry;

pub const MAX_CREATURES: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub enum CreatureState {
    Idle,
    Stalk(f32, f32),
    Attack(usize),
    Steal(i32, i32),
    Flee(f32, f32),
    Despawn,
    /// Passive fauna: wander in short hops, pause between moves
    Browse,
    /// Passive fauna: burst flee away from a threat, then resume browsing
    Scatter(f32, f32), // (away_x, away_y) direction
}

pub struct Creature {
    pub species_id: u8,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub health: f32,
    pub state: CreatureState,
    pub path: Vec<(i32, i32)>,
    pub path_idx: usize,
    pub state_timer: f32,
    pub sound_timer: f32,
    pub pack_id: u16,
    pub is_dead: bool,
    pub bleeding: f32,         // 0.0 = not bleeding, 1.0 = bleeding profusely
    pub blood_drop_timer: f32, // countdown to next blood drop on ground
    pub corpse_timer: f32,     // seconds remaining as visible corpse
    pub hop_phase: f32,        // 0.0-1.0 hop animation cycle (for hopping creatures)
    pub dropped_loot: bool,    // true once loot has been spawned on death
}

impl Creature {
    pub fn new(species_id: u8, x: f32, y: f32, pack_id: u16) -> Self {
        let reg = CreatureRegistry::cached();
        let def = reg.get(species_id);
        let health = def.map(|d| d.health).unwrap_or(10.0);
        let sound_interval = def.map(|d| d.sound_interval).unwrap_or(5.0);
        let is_passive = def.map(|d| !d.aggressive).unwrap_or(false);
        Self {
            species_id,
            x,
            y,
            angle: 0.0,
            health,
            state: if is_passive {
                CreatureState::Browse
            } else {
                CreatureState::Idle
            },
            path: Vec::new(),
            path_idx: 0,
            state_timer: 0.0,
            sound_timer: sound_interval,
            pack_id,
            is_dead: false,
            bleeding: 0.0,
            blood_drop_timer: 0.0,
            corpse_timer: 0.0,
            hop_phase: 0.0,
            dropped_loot: false,
        }
    }

    pub fn speed(&self) -> f32 {
        CreatureRegistry::cached()
            .get(self.species_id)
            .map(|d| d.speed)
            .unwrap_or(1.0)
    }

    pub fn to_gpu(&self) -> GpuCreature {
        let reg = CreatureRegistry::cached();
        let def = reg.get(self.species_id);
        let c = def.map(|d| d.color).unwrap_or([0.0; 3]);
        let ec = def.map(|d| d.eye_color).unwrap_or([0.0; 3]);
        let base_r = def.map(|d| d.body_radius).unwrap_or(0.0);

        if self.is_dead {
            return GpuCreature {
                x: self.x,
                y: self.y,
                angle: self.angle,
                health: 0.0,
                color_r: c[0] * 0.4,
                color_g: c[1] * 0.4,
                color_b: c[2] * 0.4,
                body_radius: base_r * 0.7,
                hop_offset: 0.0,
                eye_r: ec[0] * 0.3,
                eye_g: ec[1] * 0.3,
                eye_b: ec[2] * 0.3,
            };
        }

        let mut r = base_r;
        if self.state == CreatureState::Despawn {
            r *= (self.state_timer / 2.0).clamp(0.0, 1.0);
        }

        // Hop animation: sine arc during movement
        let hop = if def.map(|d| d.hop_creature).unwrap_or(false) {
            let phase = self.hop_phase;
            if phase > 0.0 && phase < 1.0 {
                (phase * std::f32::consts::PI).sin() * base_r * 0.8
            } else {
                0.0
            }
        } else {
            0.0
        };

        GpuCreature {
            x: self.x,
            y: self.y,
            angle: self.angle,
            health: self.health,
            color_r: c[0],
            color_g: c[1],
            color_b: c[2],
            body_radius: r,
            hop_offset: hop,
            eye_r: ec[0],
            eye_g: ec[1],
            eye_b: ec[2],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCreature {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub health: f32,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub body_radius: f32,
    pub hop_offset: f32, // vertical hop for animation (0 = grounded)
    pub eye_r: f32,
    pub eye_g: f32,
    pub eye_b: f32,
}

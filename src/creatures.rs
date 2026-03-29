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
}

impl Creature {
    pub fn new(species_id: u8, x: f32, y: f32, pack_id: u16) -> Self {
        let reg = CreatureRegistry::cached();
        let def = reg.get(species_id);
        let health = def.map(|d| d.health).unwrap_or(10.0);
        let sound_interval = def.map(|d| d.sound_interval).unwrap_or(5.0);
        Self {
            species_id,
            x,
            y,
            angle: 0.0,
            health,
            state: CreatureState::Idle,
            path: Vec::new(),
            path_idx: 0,
            state_timer: 0.0,
            sound_timer: sound_interval,
            pack_id,
            is_dead: false,
            bleeding: 0.0,
            blood_drop_timer: 0.0,
            corpse_timer: 0.0,
        }
    }

    pub fn speed(&self) -> f32 {
        CreatureRegistry::cached()
            .get(self.species_id)
            .map(|d| d.speed)
            .unwrap_or(1.0)
    }

    pub fn to_gpu(&self) -> GpuCreature {
        if self.is_dead {
            // Corpse: darkened, slightly smaller
            let reg = CreatureRegistry::cached();
            let def = reg.get(self.species_id);
            let r = def.map(|d| d.body_radius).unwrap_or(0.0);
            let c = def.map(|d| d.color).unwrap_or([0.0; 3]);
            return GpuCreature {
                x: self.x,
                y: self.y,
                angle: self.angle,
                health: 0.0,
                color_r: c[0] * 0.4,
                color_g: c[1] * 0.4,
                color_b: c[2] * 0.4,
                body_radius: r * 0.7,
            };
        }
        let reg = CreatureRegistry::cached();
        let def = reg.get(self.species_id);
        let c = def.map(|d| d.color).unwrap_or([0.0; 3]);
        let mut r = def.map(|d| d.body_radius).unwrap_or(0.0);
        // Despawn fade
        if self.state == CreatureState::Despawn {
            r *= (self.state_timer / 2.0).clamp(0.0, 1.0);
        }
        GpuCreature {
            x: self.x,
            y: self.y,
            angle: self.angle,
            health: self.health,
            color_r: c[0],
            color_g: c[1],
            color_b: c[2],
            body_radius: r,
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
}

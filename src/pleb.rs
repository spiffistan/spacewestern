//! Pleb (colonist) — struct, appearance, movement, A* pathfinding, activity.

use crate::grid::*;
use crate::materials::{NUM_MATERIALS, build_material_table};
use crate::needs::PlebNeeds;

/// Skill domain indices
pub const SKILL_SHOOTING: usize = 0;
pub const SKILL_MELEE: usize = 1;
pub const SKILL_CRAFTING: usize = 2;
pub const SKILL_FARMING: usize = 3;
pub const SKILL_MEDICAL: usize = 4;
pub const SKILL_CONSTRUCTION: usize = 5;
pub const NUM_SKILLS: usize = 6;

pub const SKILL_NAMES: [&str; NUM_SKILLS] = [
    "Shooting",
    "Melee",
    "Crafting",
    "Farming",
    "Medical",
    "Construction",
];

pub const SKILL_SHORT: [&str; NUM_SKILLS] = ["SHT", "MEL", "CRF", "FRM", "MED", "BLD"];

/// A single skill with continuous 0.0-10.0 level, XP tracking, and hidden aptitude.
#[derive(Clone, Debug)]
pub struct SkillLevel {
    /// Current level (0.0-10.0)
    pub value: f32,
    /// Accumulated XP toward next 0.1 increment
    pub xp: f32,
    /// Hidden aptitude (-2 to +3): determines learning speed and soft ceiling
    pub aptitude: i8,
    /// Whether the player has seen an aptitude reveal event for this skill
    pub aptitude_known: bool,
}

impl Default for SkillLevel {
    fn default() -> Self {
        Self {
            value: 0.0,
            xp: 0.0,
            aptitude: 0,
            aptitude_known: false,
        }
    }
}

impl SkillLevel {
    pub fn new(value: f32, aptitude: i8) -> Self {
        Self {
            value: value.clamp(0.0, 10.0),
            xp: 0.0,
            aptitude,
            aptitude_known: false,
        }
    }

    /// Create from old u8 skill value (1-10 → 1.0-7.0 range)
    pub fn from_legacy(old: u8, aptitude: i8) -> Self {
        // Old skill 1→1.0, 5→4.0, 10→7.0
        let value = if old <= 1 {
            1.0
        } else {
            1.0 + (old as f32 - 1.0) * 0.667
        };
        Self::new(value, aptitude)
    }

    /// Soft cap based on aptitude: 7.0 + aptitude * 1.0
    pub fn soft_cap(&self) -> f32 {
        (7.0 + self.aptitude as f32 * 1.0).clamp(4.0, 10.0)
    }

    /// XP multiplier based on proximity to soft cap (asymptotic slowdown)
    pub fn aptitude_xp_factor(&self) -> f32 {
        let cap = self.soft_cap();
        let ratio = self.value / cap;
        (1.0 - ratio.powi(4)).max(0.001)
    }

    /// Asymptotic cost multiplier near 10.0 (steep but finite)
    pub fn asymptotic_cost(&self) -> f32 {
        1.0 + (self.value / 10.0).powi(8) * 500.0
    }

    /// Total XP cost for the next 0.1 level increment
    pub fn xp_for_next(&self) -> f32 {
        let base = 10.0; // base XP per 0.1 level at level 0
        let level_cost = base * 2.0f32.powf(self.value); // exponential
        level_cost * self.asymptotic_cost() / self.aptitude_xp_factor().max(0.01)
    }

    /// Add XP, potentially leveling up. Returns the old value if a level-up occurred.
    pub fn add_xp(&mut self, raw_xp: f32) -> Option<f32> {
        let old_whole = self.value as u32;
        let needed = self.xp_for_next();
        self.xp += raw_xp;
        let mut leveled = false;
        while self.xp >= needed && self.value < 9.99 {
            self.xp -= needed;
            self.value = (self.value + 0.1).min(10.0);
            leveled = true;
            // Recompute for next increment (cost changes with level)
            break; // one increment per call to avoid runaway
        }
        if leveled && (self.value as u32 > old_whole) {
            Some(self.value)
        } else {
            None
        }
    }

    /// Speed multiplier for this skill level
    pub fn speed_mult(&self) -> f32 {
        let v = self.value;
        0.4 + v * 0.06 + (v - 5.0).max(0.0) * 0.04 + (v - 8.0).max(0.0) * 0.06
    }

    /// Failure chance for this skill level
    pub fn failure_chance(&self) -> f32 {
        (0.6 - self.value * 0.08 - (self.value - 4.0).max(0.0) * 0.04).max(0.0)
    }

    /// Descriptor string for this level
    pub fn descriptor(&self) -> &'static str {
        match self.value as u32 {
            0 => "Novice",
            1 => "Beginner",
            2..=3 => "Apprentice",
            4..=5 => "Journeyman",
            6 => "Skilled",
            7 => "Proficient",
            8 => "Expert",
            9 => "Master",
            _ => "Legendary",
        }
    }
}
use std::sync::OnceLock;

/// Combat rank — derived from firefight experience and kills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatRank {
    Green,    // 0-2 firefights: nervous, inaccurate
    Trained,  // 3-7 firefights: baseline
    Veteran,  // 8+ firefights, 2+ kills: steady, accurate
    Hardened, // 15+ firefights: unshakeable, passive rally aura
}

impl CombatRank {
    /// Stress gain multiplier (lower = more resilient).
    pub fn stress_modifier(self) -> f32 {
        match self {
            CombatRank::Green => 1.2,
            CombatRank::Trained => 1.0,
            CombatRank::Veteran => 0.85,
            CombatRank::Hardened => 0.7,
        }
    }

    /// Aim speed multiplier (higher = faster aiming).
    pub fn aim_modifier(self) -> f32 {
        match self {
            CombatRank::Green => 0.9,
            CombatRank::Trained => 1.0,
            CombatRank::Veteran => 1.1,
            CombatRank::Hardened => 1.15,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CombatRank::Green => "Green",
            CombatRank::Trained => "Trained",
            CombatRank::Veteran => "Veteran",
            CombatRank::Hardened => "Hardened",
        }
    }
}

/// Cached walkable lookup table indexed by block type. True = walkable at height 0.
static WALKABLE_TABLE: OnceLock<[bool; NUM_MATERIALS]> = OnceLock::new();

fn walkable_table() -> &'static [bool; NUM_MATERIALS] {
    WALKABLE_TABLE.get_or_init(|| {
        let mats = build_material_table();
        let mut table = [false; NUM_MATERIALS];
        for (i, m) in mats.iter().enumerate() {
            table[i] = m.walkable > 0.5;
        }
        table
    })
}

/// Check if a block type is walkable (from material table).
fn is_type_walkable(bt: u32) -> bool {
    if (bt as usize) < NUM_MATERIALS {
        walkable_table()[bt as usize]
    } else {
        false
    }
}

/// What the pleb is currently doing.
#[derive(Clone, Debug, PartialEq)]
pub enum MentalBreakKind {
    Daze,     // wanders aimlessly
    Binge,    // eats all available food
    Tantrum,  // destroys a nearby item
    Collapse, // sits on ground, won't move
}

/// A queued player command for a pleb. Shift-click appends to this queue.
#[derive(Clone, Debug)]
pub enum PlebCommand {
    MoveTo(f32, f32),
    Harvest(i32, i32),
    Haul(i32, i32),
    Eat(i32, i32),
    DigClay(i32, i32),
    HandCraft(u16),
    GatherBranches(i32, i32),
    Butcher(i32, i32),
    Fish(i32, i32),
    Mine(i32, i32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlebActivity {
    Idle,
    Walking,                           // following a path (player-ordered or auto)
    Sleeping,                          // in bed, recovering rest
    Harvesting(f32),                   // progress 0-1, harvesting a berry bush at nearby tile
    Eating,                            // consuming food (quick action)
    Hauling,                           // carrying item to a storage crate
    Farming(f32),                      // progress 0-1, planting or harvesting a crop
    Building(f32),                     // progress 0-1, constructing a blueprint
    Crafting(u16, f32),                // (recipe_id, progress 0-1)
    Digging,                           // digging terrain at work_target
    Filling,                           // dumping dirt at berm zone work_target
    Drinking(f32),                     // progress 0-1, drinking at a well
    Butchering(f32),                   // progress 0-1, butchering a dead creature
    Cooking(f32),                      // progress 0-1, cooking food at a campfire
    Fishing(f32),                      // progress 0-1, fishing at water's edge (~20s per attempt)
    Mining(f32),                       // progress 0-1, mining a sub-cell of rock
    Staggering(f32),                   // knockback recovery timer (seconds remaining)
    MentalBreak(MentalBreakKind, f32), // (kind, seconds remaining)
    /// Crisis override — pleb acts autonomously, ignoring player input.
    /// Inner activity is what they're doing (Walking to food/bed, Harvesting, Eating, Sleeping).
    Crisis(Box<PlebActivity>, &'static str), // (inner_activity, reason_label)
}

impl PlebActivity {
    /// Returns true if the pleb is in a crisis state (player input blocked).
    pub fn is_crisis(&self) -> bool {
        matches!(self, PlebActivity::Crisis(_, _))
    }

    /// Get the inner activity (unwraps crisis wrapper if present).
    pub fn inner(&self) -> &PlebActivity {
        match self {
            PlebActivity::Crisis(inner, _) => inner,
            other => other,
        }
    }

    /// Get the crisis reason label, if in crisis.
    pub fn crisis_reason(&self) -> Option<&'static str> {
        match self {
            PlebActivity::Crisis(_, reason) => Some(reason),
            _ => None,
        }
    }
}

pub use crate::resources::PlebInventory;

/// Maximum belt slots (reinforced belt)
pub const MAX_BELT_SLOTS: usize = 6;

/// Equipment worn on a pleb's body.
#[derive(Clone, Debug)]
pub struct PlebEquipment {
    /// Belt item slots (None = empty). Length = belt_capacity (0 if no belt).
    pub belt: [Option<u16>; MAX_BELT_SLOTS],
    /// How many belt slots are available (0 = no belt, 3 = fiber belt, 4 = leather, 6 = reinforced)
    pub belt_capacity: u8,
    /// Item ID of the belt itself (0 = no belt)
    pub belt_item: u16,
    /// Currently active (in-hand) item drawn from belt. None = bare hands.
    pub active_item: Option<u16>,
}

impl Default for PlebEquipment {
    fn default() -> Self {
        Self {
            belt: [None; MAX_BELT_SLOTS],
            belt_capacity: 0,
            belt_item: 0,
            active_item: None,
        }
    }
}

impl PlebEquipment {
    /// Equip a belt, setting capacity from item def.
    pub fn equip_belt(&mut self, belt_item_id: u16) {
        let reg = crate::item_defs::ItemRegistry::cached();
        if let Some(def) = reg.get(belt_item_id) {
            if def.is_belt {
                self.belt_item = belt_item_id;
                self.belt_capacity = def.belt_slots.max(1);
            }
        }
    }

    /// Try to add an item to the first empty belt slot. Returns true on success.
    pub fn add_to_belt(&mut self, item_id: u16) -> bool {
        for i in 0..self.belt_capacity as usize {
            if self.belt[i].is_none() {
                self.belt[i] = Some(item_id);
                return true;
            }
        }
        false
    }

    /// Remove an item from the belt by item_id. Returns true if found and removed.
    pub fn remove_from_belt(&mut self, item_id: u16) -> bool {
        for i in 0..self.belt_capacity as usize {
            if self.belt[i] == Some(item_id) {
                self.belt[i] = None;
                if self.active_item == Some(item_id) {
                    self.active_item = None;
                }
                return true;
            }
        }
        false
    }

    /// Check if a specific item is on the belt.
    pub fn has_on_belt(&self, item_id: u16) -> bool {
        self.belt[..self.belt_capacity as usize]
            .iter()
            .any(|s| *s == Some(item_id))
    }

    /// Check if any item with a given tool_type is on the belt.
    pub fn has_tool(&self, tool_type: &str) -> bool {
        let reg = crate::item_defs::ItemRegistry::cached();
        self.belt[..self.belt_capacity as usize].iter().any(|s| {
            s.and_then(|id| reg.get(id))
                .is_some_and(|d| d.has_tool_type(tool_type))
        })
    }

    /// Find and draw the best item for a tool_type. Returns item_id if found.
    pub fn draw_tool(&mut self, tool_type: &str) -> Option<u16> {
        let reg = crate::item_defs::ItemRegistry::cached();
        for i in 0..self.belt_capacity as usize {
            if let Some(id) = self.belt[i] {
                if let Some(def) = reg.get(id) {
                    if def.has_tool_type(tool_type) {
                        self.active_item = Some(id);
                        return Some(id);
                    }
                }
            }
        }
        None
    }

    /// Draw the best ranged weapon from belt.
    pub fn draw_ranged(&mut self) -> Option<u16> {
        let reg = crate::item_defs::ItemRegistry::cached();
        for i in 0..self.belt_capacity as usize {
            if let Some(id) = self.belt[i] {
                if let Some(def) = reg.get(id) {
                    if def.is_ranged_weapon() {
                        self.active_item = Some(id);
                        return Some(id);
                    }
                }
            }
        }
        None
    }

    /// Draw the best melee weapon from belt.
    pub fn draw_melee(&mut self) -> Option<u16> {
        let reg = crate::item_defs::ItemRegistry::cached();
        let mut best: Option<(u16, f32)> = None;
        for i in 0..self.belt_capacity as usize {
            if let Some(id) = self.belt[i] {
                if let Some(def) = reg.get(id) {
                    if def.is_melee_weapon() && best.map_or(true, |(_, bd)| def.melee_damage > bd) {
                        best = Some((id, def.melee_damage));
                    }
                }
            }
        }
        if let Some((id, _)) = best {
            self.active_item = Some(id);
            Some(id)
        } else {
            None
        }
    }

    /// Holster: put active item away.
    pub fn holster(&mut self) {
        self.active_item = None;
    }

    /// Check if any belt item matches an item_id (for haul protection).
    pub fn is_equipped(&self, item_id: u16) -> bool {
        self.belt_item == item_id
            || self.belt[..self.belt_capacity as usize]
                .iter()
                .any(|s| *s == Some(item_id))
    }

    /// Get all occupied belt slots as (slot_index, item_id).
    pub fn belt_items(&self) -> Vec<(usize, u16)> {
        self.belt[..self.belt_capacity as usize]
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|id| (i, id)))
            .collect()
    }

    /// Auto-equip: move weapons/tools from inventory to belt.
    pub fn auto_migrate_from_inventory(&mut self, inventory: &mut crate::resources::PlebInventory) {
        if self.belt_capacity == 0 {
            return;
        }
        let reg = crate::item_defs::ItemRegistry::cached();
        // Collect item IDs to move (scan inventory for belt-worthy items)
        let to_move: Vec<u16> = inventory
            .stacks
            .iter()
            .filter(|s| {
                reg.get(s.item_id)
                    .is_some_and(|d| d.is_belt_item() && s.count == 1)
            })
            .map(|s| s.item_id)
            .collect();
        for id in to_move {
            if self.add_to_belt(id) {
                inventory.remove(id, 1);
            }
        }
    }
}

/// Floating bubble above a pleb's head.
#[derive(Clone, Debug)]
pub enum BubbleKind {
    /// Large icon character (e.g. '!', '?') with a color
    Icon(char, [u8; 3]),
    /// Short text string
    Text(String),
    /// Thought bubble: internal feeling (rendered as cloud, not speech)
    Thought(String),
}

/// Priority tiers for bubbles (higher = more important, replaces lower).
impl BubbleKind {
    pub fn priority(&self) -> u8 {
        match self {
            BubbleKind::Icon('!', _) => 3, // danger
            BubbleKind::Icon('?', _) => 1, // curious
            BubbleKind::Text(_) => 2,      // action text
            BubbleKind::Icon(_, _) => 2,   // other icons
            BubbleKind::Thought(_) => 1,   // internal thoughts (low priority)
        }
    }
}

/// Appearance data for rendering a pleb (Rimworld-style).
#[derive(Clone, Debug)]
pub struct PlebAppearance {
    pub skin_r: f32,
    pub skin_g: f32,
    pub skin_b: f32,
    pub hair_r: f32,
    pub hair_g: f32,
    pub hair_b: f32,
    pub shirt_r: f32,
    pub shirt_g: f32,
    pub shirt_b: f32,
    pub pants_r: f32,
    pub pants_g: f32,
    pub pants_b: f32,
    pub hair_style: u32, // 0=bald, 1=short, 2=medium, 3=long
}

impl PlebAppearance {
    /// Generate random appearance from a seed.
    pub fn random(seed: u32) -> Self {
        let hash = |i: u32| -> f32 {
            let h = seed
                .wrapping_mul(2654435761)
                .wrapping_add(i.wrapping_mul(1013904223));
            (h & 0xFFFF) as f32 / 65535.0
        };

        // Skin tone range (warm tones)
        let skin_base = hash(0);
        let skin_r = 0.65 + skin_base * 0.30;
        let skin_g = 0.50 + skin_base * 0.25;
        let skin_b = 0.35 + skin_base * 0.20;

        // Hair color
        let hair_base = hash(1);
        let (hair_r, hair_g, hair_b) = if hair_base < 0.3 {
            (0.15 + hash(2) * 0.15, 0.10 + hash(2) * 0.10, 0.05) // dark brown/black
        } else if hair_base < 0.6 {
            (0.45 + hash(2) * 0.15, 0.30 + hash(2) * 0.10, 0.15) // brown
        } else if hair_base < 0.8 {
            (0.70 + hash(2) * 0.20, 0.55 + hash(2) * 0.15, 0.20) // blonde
        } else {
            (0.55 + hash(2) * 0.15, 0.15, 0.10) // red
        };

        // Shirt color (varied)
        let shirt_hue = hash(3);
        let (shirt_r, shirt_g, shirt_b) = if shirt_hue < 0.2 {
            (0.25, 0.40, 0.65) // blue
        } else if shirt_hue < 0.4 {
            (0.55, 0.30, 0.25) // red/brown
        } else if shirt_hue < 0.6 {
            (0.30, 0.50, 0.30) // green
        } else if shirt_hue < 0.8 {
            (0.55, 0.55, 0.50) // gray
        } else {
            (0.60, 0.50, 0.30) // tan
        };

        // Pants color (muted)
        let pants_hue = hash(4);
        let (pants_r, pants_g, pants_b) = if pants_hue < 0.4 {
            (0.25, 0.25, 0.35) // dark blue/gray
        } else if pants_hue < 0.7 {
            (0.35, 0.30, 0.20) // brown
        } else {
            (0.30, 0.30, 0.30) // dark gray
        };

        let hair_style = (hash(5) * 4.0) as u32;

        PlebAppearance {
            skin_r,
            skin_g,
            skin_b,
            hair_r,
            hair_g,
            hair_b,
            shirt_r,
            shirt_g,
            shirt_b,
            pants_r,
            pants_g,
            pants_b,
            hair_style,
        }
    }
}

/// GPU-side pleb data for rendering (packed for storage buffer).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuPleb {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub selected: f32,
    pub torch: f32,
    pub headlight: f32,
    pub carrying: f32,
    pub health: f32,
    pub skin_r: f32,
    pub skin_g: f32,
    pub skin_b: f32,
    pub hair_style: f32,
    pub hair_r: f32,
    pub hair_g: f32,
    pub hair_b: f32,
    pub aim_progress: f32, // 0.0 = not aiming, 0.01-0.99 = aiming, 1.0 = firing
    pub shirt_r: f32,
    pub shirt_g: f32,
    pub shirt_b: f32,
    pub weapon_type: f32, // 0=unarmed, 1=axe, 2=pick, 3=shovel
    pub pants_r: f32,
    pub pants_g: f32,
    pub pants_b: f32,
    pub swing_progress: f32, // 0.0=idle, 0.01-0.99=windup, 1.0=strike
    pub crouch: f32,         // 0.0=standing, 0.5=peeking, 1.0=crouching
    pub stress: f32,         // 0.0-1.0 normalized stress (0=calm, 1=breaking)
    pub wetness: f32,        // 0.0=dry, 1.0=soaked (darkens clothing)
    pub sleeping: f32,       // >0.5 = sleeping (lay flat + Z's)
}

pub struct Pleb {
    pub id: usize,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub path: Vec<(i32, i32)>,
    pub path_idx: usize,
    pub torch_on: bool,
    pub headlight_mode: u8, // 0=off, 1=wide, 2=normal, 3=focused
    pub appearance: PlebAppearance,
    pub needs: PlebNeeds,
    pub prev_x: f32, // previous frame position (for detecting movement)
    pub prev_y: f32,
    pub activity: PlebActivity,
    pub inventory: PlebInventory,
    pub harvest_target: Option<(i32, i32)>, // grid coords of bush being harvested
    pub haul_target: Option<(i32, i32)>,    // grid coords of storage crate to deliver to
    pub is_enemy: bool,
    pub backstory_name: String,           // display name from Backstory
    pub trait_name: Option<String>,       // display name from PlebTrait
    pub skills: [SkillLevel; NUM_SKILLS], // [shooting, melee, crafting, farming, medical, construction]
    pub wander_timer: f32,
    pub work_target: Option<(i32, i32)>, // position of current work task
    pub schedule: PlebSchedule,
    pub knockback_vx: f32, // explosion knockback velocity (decays over time)
    pub knockback_vy: f32,
    pub is_dead: bool,                     // corpse: stays in world but doesn't act
    pub drafted: bool,                     // true = player controls only, no autonomous behavior
    pub aim_target: Option<usize>,         // index of enemy pleb being targeted
    pub aim_pos: Option<(f32, f32)>,       // manual fire-at-position (overrides aim_target)
    pub aim_progress: f32,                 // 0.0 = just started aiming, 1.0 = ready to fire
    pub equipped_weapon: Option<u16>,      // DEPRECATED: delegates to equipment.active_item
    pub prefer_ranged: bool,               // true = use pistol, false = use melee
    pub equipment: PlebEquipment,          // belt, active item, tool access
    pub swing_progress: f32,               // melee swing: 0.0 = idle, 1.0 = strike
    pub bleeding: f32,                     // blood loss rate (0.0–1.0)
    pub blood_drop_timer: f32,             // countdown to next blood drop on ground
    pub stagger_timer: f32,                // seconds of movement freeze after being hit
    pub ammo_loaded: u8,                   // rounds left in magazine (0 = need reload)
    pub reload_timer: f32,                 // >0 = currently reloading (seconds remaining)
    pub bubble: Option<(BubbleKind, f32)>, // (kind, seconds_remaining)
    pub weapon_swap_timer: f32,            // >0 = switching weapons (can't attack)
    pub suppression: f32,                  // 0.0–1.0: accuracy penalty from near-miss bullets
    pub crouching: bool,                   // true = crouch target state
    pub crouch_progress: f32,              // 0.0=standing, 1.0=fully crouched (smooth transition)
    pub peek_timer: f32, // >0 = peeking up from crouch to fire (seconds remaining)
    pub last_shout_timer: f32, // cooldown between shouts (decays each frame)
    pub prev_health_band: u8, // health threshold tracking (0=full, 1=<50%, 2=<35%)
    pub group_id: Option<u8>, // explicit group membership (None = ungrouped)
    pub work_priorities: [u8; 4], // [haul, farm, build, craft] — 0=off, 1-3=priority
    pub command_queue: Vec<PlebCommand>, // shift-click queued commands
    pub is_leader: bool, // can issue rally/command shouts
    pub firefights_survived: u16, // combat experience counter
    pub kills: u16,      // confirmed enemy kills
    pub combat_participated: bool, // had aim_target during current engagement
    pub no_enemy_timer: f32, // seconds since last enemy in range (for firefight end detection)
    pub command_cooldown: f32, // seconds until next command shout allowed
    pub hunt_target: Option<usize>, // creature index being hunted (stalk + shoot)
    pub nauseous_timer: f32, // >0 = nauseous from raw food (seconds remaining)
    pub smoke_exposure: f32, // current smoke density at position (updated from air readback)
    pub wetness_emote: u8, // bit 0 = damp emote fired, bit 1 = soaked emote fired
    /// Tracks which need-emote thresholds have fired (reset when need recovers).
    /// Bits: 0=hunger_low, 1=hunger_crit, 2=thirst_low, 3=thirst_crit,
    ///       4=rest_low, 5=rest_crit, 6=warmth_low, 7=warmth_crit
    pub need_emote_flags: u8,
    /// Per-pleb event log (most recent first, capped)
    pub event_log: Vec<(f32, String)>, // (game_time, message)
}

/// Per-pleb 24-hour schedule. Each hour is either work (true) or sleep (false).
#[derive(Clone, Debug)]
pub struct PlebSchedule {
    pub hours: [bool; 24], // true = work, false = sleep
    pub preset: PlebShift,
}

impl Default for PlebSchedule {
    fn default() -> Self {
        let mut s = PlebSchedule {
            hours: [true; 24],
            preset: PlebShift::Day,
        };
        s.apply_preset(PlebShift::Day);
        s
    }
}

impl PlebSchedule {
    pub fn apply_preset(&mut self, shift: PlebShift) {
        self.preset = shift;
        for h in 0..24 {
            self.hours[h] = !shift.is_sleep_hour(h);
        }
    }

    pub fn is_sleep_time(&self, time_of_day: f32, day_duration: f32) -> bool {
        let hour = ((time_of_day / day_duration * 24.0) % 24.0) as usize;
        !self.hours[hour.min(23)]
    }
}

/// Shift presets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlebShift {
    Day,    // sleep 22:00-06:00
    Night,  // sleep 10:00-18:00
    Custom, // manually edited
}

impl PlebShift {
    pub fn is_sleep_hour(&self, h: usize) -> bool {
        match self {
            PlebShift::Day => !(6..22).contains(&h),
            PlebShift::Night => (10..18).contains(&h),
            PlebShift::Custom => false, // custom uses the hours array directly
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PlebShift::Day => "Day",
            PlebShift::Night => "Night",
            PlebShift::Custom => "Custom",
        }
    }
}

impl Pleb {
    pub fn new(id: usize, name: String, x: f32, y: f32, seed: u32) -> Self {
        // Random skills from seed (backstory/trait names set later by caller)
        let h = |s: u32, off: u32| -> u32 {
            s.wrapping_mul(2654435761)
                .wrapping_add(off.wrapping_mul(1013904223))
                >> 16
        };
        // Random skill levels (1-7 legacy → 1.0-5.0 new) + random aptitudes (-1 to +2)
        let skills = std::array::from_fn(|i| {
            let old_val = (h(seed, (i + 1) as u32) % 7 + 1) as u8;
            let apt_raw = h(seed, (i + 10) as u32) % 4; // 0,1,2,3 → -1,0,+1,+2
            let aptitude = apt_raw as i8 - 1;
            SkillLevel::from_legacy(old_val, aptitude)
        });

        Pleb {
            id,
            name,
            x,
            y,
            angle: 0.0,
            path: Vec::new(),
            path_idx: 0,
            torch_on: false,
            headlight_mode: 0,
            appearance: PlebAppearance::random(seed),
            needs: PlebNeeds::default(),
            prev_x: x,
            prev_y: y,
            activity: PlebActivity::Idle,
            inventory: {
                let mut inv = PlebInventory::default();
                inv.add(crate::item_defs::ITEM_STONE_AXE, 1);
                inv.add(crate::item_defs::ITEM_PISTOL, 1);
                inv
            },
            harvest_target: None,
            haul_target: None,
            is_enemy: false,
            backstory_name: String::new(),
            trait_name: None,
            skills,
            wander_timer: 0.0,
            work_target: None,
            schedule: PlebSchedule::default(),
            knockback_vx: 0.0,
            knockback_vy: 0.0,
            is_dead: false,
            drafted: false,
            aim_target: None,
            aim_pos: None,
            aim_progress: 0.0,
            equipped_weapon: None,
            prefer_ranged: true,
            equipment: PlebEquipment::default(),
            swing_progress: 0.0,
            bleeding: 0.0,
            blood_drop_timer: 0.0,
            stagger_timer: 0.0,
            ammo_loaded: 6,
            reload_timer: 0.0,
            bubble: None,
            weapon_swap_timer: 0.0,
            suppression: 0.0,
            crouching: false,
            crouch_progress: 0.0,
            peek_timer: 0.0,
            last_shout_timer: 0.0,
            prev_health_band: 0,
            group_id: None,
            work_priorities: crate::zones::default_work_priorities(),
            command_queue: Vec::new(),
            is_leader: false,
            firefights_survived: 0,
            kills: 0,
            combat_participated: false,
            no_enemy_timer: 0.0,
            command_cooldown: 0.0,
            hunt_target: None,
            nauseous_timer: 0.0,
            smoke_exposure: 0.0,
            wetness_emote: 0,
            need_emote_flags: 0,
            event_log: Vec::new(),
        }
    }

    /// Log an event to this pleb's personal log (capped at 50 entries).
    pub fn log_event(&mut self, time: f32, msg: String) {
        self.event_log.push((time, msg));
        if self.event_log.len() > 50 {
            self.event_log.remove(0);
        }
    }

    pub fn to_gpu(&self, selected: bool) -> GpuPleb {
        let a = &self.appearance;
        GpuPleb {
            x: self.x,
            y: self.y,
            angle: self.angle,
            selected: if selected { 1.0 } else { 0.0 },
            torch: if self.torch_on { 1.0 } else { 0.0 },
            headlight: self.headlight_mode as f32, // 0=off, 1=wide, 2=normal, 3=focused
            carrying: self
                .inventory
                .carrying_type()
                .map(|id| id as f32 + 1.0) // +1 so 0 = not carrying
                .unwrap_or(0.0),
            health: self.needs.health,
            skin_r: a.skin_r,
            skin_g: a.skin_g,
            skin_b: a.skin_b,
            hair_style: a.hair_style as f32,
            hair_r: a.hair_r,
            hair_g: a.hair_g,
            hair_b: a.hair_b,
            aim_progress: self.aim_progress,
            shirt_r: a.shirt_r,
            shirt_g: a.shirt_g,
            shirt_b: a.shirt_b,
            weapon_type: self
                .equipped_weapon
                .and_then(|id| crate::item_defs::ItemRegistry::cached().get(id))
                .map(|d| d.weapon_type as f32)
                .unwrap_or(0.0),
            pants_r: a.pants_r,
            pants_g: a.pants_g,
            pants_b: a.pants_b,
            swing_progress: self.swing_progress,
            crouch: if self.peek_timer > 0.0 {
                self.crouch_progress * 0.5 // peeking = half-crouch visually
            } else {
                self.crouch_progress
            },
            stress: (self.needs.stress / 100.0).clamp(0.0, 1.0),
            wetness: self.needs.wetness,
            sleeping: if matches!(self.activity.inner(), PlebActivity::Sleeping) {
                1.0
            } else {
                0.0
            },
        }
    }

    /// Auto-equip the best weapon based on prefer_ranged toggle.
    /// Draws from belt (equipment system). Syncs deprecated equipped_weapon field.
    pub fn update_equipped_weapon(&mut self) {
        if self.equipment.belt_capacity > 0 {
            // Use equipment system
            let drawn = if self.prefer_ranged {
                self.equipment
                    .draw_ranged()
                    .or_else(|| self.equipment.draw_melee())
            } else {
                self.equipment
                    .draw_melee()
                    .or_else(|| self.equipment.draw_ranged())
            };
            self.equipped_weapon = drawn;
        } else {
            // Fallback: scan inventory (for plebs without belts)
            let reg = crate::item_defs::ItemRegistry::cached();
            if self.prefer_ranged {
                for stack in &self.inventory.stacks {
                    if let Some(def) = reg.get(stack.item_id) {
                        if def.is_ranged_weapon() {
                            self.equipped_weapon = Some(stack.item_id);
                            return;
                        }
                    }
                }
            }
            let mut best: Option<(u16, f32)> = None;
            for stack in &self.inventory.stacks {
                if let Some(def) = reg.get(stack.item_id) {
                    if def.is_melee_weapon() && best.map_or(true, |(_, bd)| def.melee_damage > bd) {
                        best = Some((stack.item_id, def.melee_damage));
                    }
                }
            }
            self.equipped_weapon = best.map(|(id, _)| id);
        }
        if !self.drafted {
            self.swing_progress = 0.0;
            self.aim_progress = 0.0;
        }
    }

    /// Check if pleb has a specific trait (by name string).
    pub fn has_trait(&self, name: &str) -> bool {
        self.trait_name.as_deref() == Some(name)
    }

    /// Skill speed multiplier for a domain (uses DN-022 formula)
    pub fn skill_mult(&self, idx: usize) -> f32 {
        self.skills.get(idx).map(|s| s.speed_mult()).unwrap_or(0.7)
    }

    /// Raw skill value for a domain (0.0-10.0)
    pub fn skill_value(&self, idx: usize) -> f32 {
        self.skills.get(idx).map(|s| s.value).unwrap_or(0.0)
    }

    /// Grant XP to a skill domain, with mood modifier. Returns Some(new_level) on whole-number level-up.
    pub fn gain_xp(&mut self, skill_idx: usize, base_xp: f32) -> Option<f32> {
        if skill_idx >= NUM_SKILLS {
            return None;
        }
        // Mood modifier: happy +20%, stressed -30%, breaking 0%
        let mood_mult = if self.needs.stress > 85.0 {
            0.0 // can't learn while breaking
        } else if self.needs.mood > 30.0 {
            1.2
        } else if self.needs.mood < -20.0 {
            0.7
        } else {
            1.0
        };
        let xp = base_xp * mood_mult;
        self.skills[skill_idx].add_xp(xp)
    }

    /// Grant XP with failure bonus (50% more XP from failures)
    pub fn gain_xp_failure(&mut self, skill_idx: usize, base_xp: f32) -> Option<f32> {
        self.gain_xp(skill_idx, base_xp * 1.5)
    }

    /// Grant XP and auto-log level-ups. Combines gain_xp + event logging.
    pub fn gain_xp_logged(&mut self, skill_idx: usize, base_xp: f32, time: f32) {
        if let Some(new_level) = self.gain_xp(skill_idx, base_xp) {
            self.log_event(
                time,
                format!(
                    "{} improved to {:.1}",
                    SKILL_NAMES.get(skill_idx).unwrap_or(&"?"),
                    new_level,
                ),
            );
        }
    }

    /// Grant failure XP and auto-log level-ups.
    pub fn gain_xp_failure_logged(&mut self, skill_idx: usize, base_xp: f32, time: f32) {
        if let Some(new_level) = self.gain_xp_failure(skill_idx, base_xp) {
            self.log_event(
                time,
                format!(
                    "{} improved to {:.1}",
                    SKILL_NAMES.get(skill_idx).unwrap_or(&"?"),
                    new_level,
                ),
            );
        }
    }

    /// Set skills from legacy [u8; 6] (backstory), preserving existing aptitudes
    pub fn set_skills_from_legacy(&mut self, old: [u8; 6]) {
        for i in 0..NUM_SKILLS {
            let apt = self.skills[i].aptitude; // preserve aptitude
            self.skills[i] = SkillLevel::from_legacy(old[i], apt);
        }
    }

    /// Crafting speed multiplier (skill 2 + SteadyHands trait)
    pub fn crafting_speed(&self) -> f32 {
        let base = self.skill_mult(2);
        if self.has_trait("Steady Hands") {
            base * 1.2
        } else {
            base
        }
    }

    /// Farming/harvest speed multiplier (skill 3 + Frontier trait for outdoor work)
    pub fn farming_speed(&self) -> f32 {
        let base = self.skill_mult(3);
        if self.has_trait("Frontier Born") {
            base * 1.15
        } else {
            base
        }
    }

    /// Construction speed multiplier (skill 5 + Frontier trait)
    pub fn construction_speed(&self) -> f32 {
        let base = self.skill_mult(5);
        if self.has_trait("Frontier Born") {
            base * 1.15
        } else {
            base
        }
    }

    /// Mining yield multiplier (Salvager trait)
    pub fn mining_yield_mult(&self) -> f32 {
        if self.has_trait("Salvager") { 1.3 } else { 1.0 }
    }

    /// Thirst decay multiplier (DesertBlood trait)
    pub fn thirst_decay_mult(&self) -> f32 {
        if self.has_trait("Desert Blood") {
            0.7
        } else {
            1.0
        }
    }

    /// Food sickness immunity (StoneEater trait)
    pub fn immune_to_food_sickness(&self) -> bool {
        self.has_trait("Stone Eater")
    }

    /// Bleed resistance (Weathered trait): lower = slower bleed damage
    pub fn bleed_resist(&self) -> f32 {
        if self.has_trait("Weathered") {
            0.6
        } else {
            1.0
        }
    }

    /// Max health multiplier (Weathered trait)
    pub fn max_health_mult(&self) -> f32 {
        if self.has_trait("Weathered") {
            1.25
        } else {
            1.0
        }
    }

    /// Check if an item is equipped (protected from hauling/storing).
    /// Checks both belt items and the belt itself.
    pub fn is_equipped(&self, item_id: u16) -> bool {
        self.equipment.is_equipped(item_id) || self.equipped_weapon == Some(item_id)
    }

    /// Check if pleb has a specific tool type on their belt.
    pub fn has_tool(&self, tool_type: &str) -> bool {
        self.equipment.has_tool(tool_type)
    }

    /// Draw a specific tool from belt, setting it as active. Returns item_id.
    pub fn draw_tool(&mut self, tool_type: &str) -> Option<u16> {
        let id = self.equipment.draw_tool(tool_type);
        if id.is_some() {
            self.equipped_weapon = id;
        }
        id
    }

    /// Combat rank derived from experience.
    pub fn rank(&self) -> CombatRank {
        if self.firefights_survived >= 15 {
            CombatRank::Hardened
        } else if self.firefights_survived >= 8 && self.kills >= 2 {
            CombatRank::Veteran
        } else if self.firefights_survived >= 3 {
            CombatRank::Trained
        } else {
            CombatRank::Green
        }
    }

    /// Current Z-height for bullet collision. Smooth: 1.0 standing, 0.7 peeking, 0.4 crouched.
    pub fn z_height(&self) -> f32 {
        let base = 1.0 - self.crouch_progress * 0.6; // 1.0 → 0.4
        if self.peek_timer > 0.0 {
            base.max(0.7) // peeking raises to at least 0.7
        } else {
            base
        }
    }

    /// Clear all work/harvest/haul targets (common pattern after re-tasking).
    pub fn clear_targets(&mut self) {
        self.work_target = None;
        self.haul_target = None;
        self.harvest_target = None;
    }

    /// Set a bubble, respecting priority (higher priority replaces lower).
    pub fn set_bubble(&mut self, kind: BubbleKind, duration: f32) {
        let new_pri = kind.priority();
        if let Some((ref existing, remaining)) = self.bubble {
            if existing.priority() > new_pri && remaining > 0.2 {
                return; // don't override a more important active bubble
            }
        }
        self.bubble = Some((kind, duration));
    }
}

pub const MAX_PLEBS: usize = 16;

/// Names pool for random pleb names.
const NAMES: &[&str] = &[
    "Jeff", "Sarah", "Marcus", "Elena", "Dmitri", "Yuki", "Carlos", "Amara", "Olaf", "Priya",
    "Liam", "Zara", "Kento", "Ingrid", "Rashid", "Mei",
];

pub fn random_name(seed: u32) -> String {
    let idx = (seed.wrapping_mul(2654435761) >> 16) as usize % NAMES.len();
    NAMES[idx].to_string()
}

/// Check if a pleb can stand at continuous position (x, y) using 4-corner bounding box.
pub fn is_walkable_pos(grid: &[u32], x: f32, y: f32) -> bool {
    is_walkable_pos_wd(grid, &[], x, y)
}

pub fn is_walkable_pos_wd(grid: &[u32], wall_data: &[u16], x: f32, y: f32) -> bool {
    let r = 0.25;
    for &(cx, cy) in &[
        (x - r, y - r),
        (x + r, y - r),
        (x - r, y + r),
        (x + r, y + r),
    ] {
        let bx = cx.floor() as i32;
        let by = cy.floor() as i32;
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
            return false;
        }
        let idx = (by as u32 * GRID_W + bx as u32) as usize;
        if idx < wall_data.len() && wd_blocks_movement(wall_data[idx], false) {
            return false;
        }
        let b = grid[idx];
        let bt = b & 0xFF;
        let bh = (b >> 8) & 0xFF;
        let is_door = (b >> 16) & 1 != 0;
        let is_dug_shallow = bt == BT_DUG_GROUND && bh <= 1;
        let is_pipe = ((15..=20).contains(&bt)
            || bt == BT_RESTRICTOR
            || bt == BT_LIQUID_PIPE
            || bt == BT_PIPE_BRIDGE
            || bt == BT_LIQUID_INTAKE
            || bt == BT_LIQUID_PUMP
            || bt == BT_LIQUID_OUTPUT)
            && bh <= 1;
        // Wire/power equipment: height byte is connection mask, not visual height
        let is_wire = is_wire_block(bt);
        // Diagonal wall: check which side of the diagonal this corner is on
        if bt == BT_DIAGONAL {
            let variant = (b >> 19) & 3;
            let lfx = cx - (cx.floor());
            let lfy = cy - (cy.floor());
            let on_wall = match variant {
                0 => lfy > (1.0 - lfx),
                1 => lfy > lfx,
                2 => lfy < (1.0 - lfx),
                _ => lfy < lfx,
            };
            if on_wall {
                return false;
            }
            continue; // open half is walkable
        }
        // Thin walls: walkable in open sub-cells
        let is_thin = is_wall_block(bt) && bh > 0 && thin_wall_is_walkable(b);
        if !is_door
            && !is_thin
            && !is_dug_shallow
            && !is_pipe
            && !is_wire
            && (bh > 0 || !is_type_walkable(bt))
        {
            return false;
        }
    }
    true
}

/// Water depth that fully blocks pathfinding (impassable — too deep to wade).
pub const DEEP_WATER_THRESHOLD: f32 = 1.0;

/// Check if a tile has deep water using the CPU water depth mirror.
pub fn is_deep_water_cpu(water_depth_cpu: &[f32], bx: i32, by: i32) -> bool {
    if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
        return false;
    }
    let idx = (by as u32 * GRID_W + bx as u32) as usize;
    if idx >= water_depth_cpu.len() {
        return false;
    }
    water_depth_cpu[idx] > DEEP_WATER_THRESHOLD
}

/// Check if a tile has deep water (blocks movement). Requires water table + elevation data.
pub fn is_deep_water(
    water_table: &[f32],
    elevation: &[f32],
    water_table_offset: f32,
    bx: i32,
    by: i32,
) -> bool {
    if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
        return false;
    }
    let idx = (by as u32 * GRID_W + bx as u32) as usize;
    if idx >= water_table.len() || idx >= elevation.len() {
        return false;
    }
    let wt = water_table[idx] + water_table_offset;
    let elev = elevation[idx];
    wt - elev > DEEP_WATER_THRESHOLD
}

/// Find the nearest walkable tile adjacent to (gx, gy). Used when pathfinding to non-walkable targets (e.g. crates, walls).
pub fn adjacent_walkable(grid: &[u32], gx: i32, gy: i32) -> Option<(i32, i32)> {
    for &(dx, dy) in &[
        (0i32, -1i32),
        (0, 1),
        (-1, 0),
        (1, 0),
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
    ] {
        let nx = gx + dx;
        let ny = gy + dy;
        if is_walkable_pos(grid, nx as f32 + 0.5, ny as f32 + 0.5) {
            return Some((nx, ny));
        }
    }
    None
}

/// A* pathfinding on the block grid. Returns path from start to goal (inclusive), or empty if unreachable.
/// Legacy A* without wall_data (for tests).
pub fn astar_path(grid: &[u32], start: (i32, i32), goal: (i32, i32)) -> Vec<(i32, i32)> {
    astar_path_terrain_wd(grid, &[], &[], start, goal)
}

/// Legacy A* with terrain but no wall_data (for callers without wall_data access).
pub fn astar_path_terrain(
    grid: &[u32],
    terrain: &[u32],
    start: (i32, i32),
    goal: (i32, i32),
) -> Vec<(i32, i32)> {
    astar_path_terrain_wd(grid, &[], terrain, start, goal)
}

/// Primary A* pathfinding — wall_data-aware, terrain cost, doors passable.
pub fn astar_path_terrain_wd(
    grid: &[u32],
    wall_data: &[u16],
    terrain: &[u32],
    start: (i32, i32),
    goal: (i32, i32),
) -> Vec<(i32, i32)> {
    astar_path_wd(grid, wall_data, &[], &[], 0.0, terrain, start, goal)
}

/// Convenience: terrain_wd + water awareness via CPU depth mirror.
/// `water_depth_cpu` is the pre-computed water depth per tile (updated each frame).
pub fn astar_path_terrain_water_wd(
    grid: &[u32],
    wall_data: &[u16],
    terrain: &[u32],
    water_depth_cpu: &[f32],
    start: (i32, i32),
    goal: (i32, i32),
) -> Vec<(i32, i32)> {
    astar_path_wd(
        grid,
        wall_data,
        water_depth_cpu,
        &[],
        0.0,
        terrain,
        start,
        goal,
    )
}

/// A* with water-aware pathfinding.
pub fn astar_path_water_wd(
    grid: &[u32],
    wall_data: &[u16],
    water_table: &[f32],
    elevation: &[f32],
    water_table_offset: f32,
    terrain: &[u32],
    start: (i32, i32),
    goal: (i32, i32),
) -> Vec<(i32, i32)> {
    astar_path_wd(
        grid,
        wall_data,
        water_table,
        elevation,
        water_table_offset,
        terrain,
        start,
        goal,
    )
}

/// A* pathfinding with wall_data layer support.
pub fn astar_path_wd(
    grid: &[u32],
    wall_data: &[u16],
    water_table: &[f32],
    elevation: &[f32],
    water_table_offset: f32,
    terrain: &[u32],
    start: (i32, i32),
    goal: (i32, i32),
) -> Vec<(i32, i32)> {
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashMap};

    if start == goal {
        return vec![goal];
    }

    let is_walk = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= GRID_W as i32 || y >= GRID_H as i32 {
            return false;
        }
        let idx = (y as u32 * GRID_W + x as u32) as usize;
        // Deep water blocks movement
        if !water_table.is_empty() && idx < water_table.len() {
            let depth = if elevation.is_empty() {
                water_table[idx] // precomputed depth
            } else if idx < elevation.len() {
                (water_table[idx] + water_table_offset) - elevation[idx]
            } else {
                0.0
            };
            if depth > DEEP_WATER_THRESHOLD {
                return false;
            }
        }
        // Doors passable for pathfinding (pleb will push them open)
        if idx < wall_data.len() && wd_blocks_movement(wall_data[idx], true) {
            return false;
        }
        let b = grid[idx];
        let bt = b & 0xFF;
        let bh = (b >> 8) & 0xFF;
        let is_door = (b >> 16) & 1 != 0;
        let is_any_pipe = (15..=20).contains(&bt)
            || bt == BT_RESTRICTOR
            || bt == BT_LIQUID_PIPE
            || bt == BT_PIPE_BRIDGE
            || bt == BT_LIQUID_INTAKE
            || bt == BT_LIQUID_PUMP
            || bt == BT_LIQUID_OUTPUT;
        let is_wire = is_wire_block(bt);
        let is_thin = is_wall_block(bt) && bh > 0 && thin_wall_is_walkable(b);
        is_door
            || is_thin
            || (bh == 0 && is_type_walkable(bt))
            || (bt == BT_DUG_GROUND && bh <= 1)
            || (is_any_pipe && bh <= 1)
            || is_wire
            || bt == BT_DIAGONAL
    };

    if !is_walk(goal.0, goal.1) {
        return vec![];
    }

    // Chebyshev distance heuristic (optimal for 8-directional with cost 10/14)
    let heuristic = |a: (i32, i32)| -> i32 {
        let dx = (a.0 - goal.0).abs();
        let dy = (a.1 - goal.1).abs();
        10 * (dx + dy) + (14 - 2 * 10) * dx.min(dy) // = 10*max + 14*min - 10*min = 10*max + 4*min
    };

    // 8 directions: 4 cardinal + 4 diagonal
    const DIRS: [(i32, i32, i32); 8] = [
        (0, -1, 10),
        (0, 1, 10),
        (-1, 0, 10),
        (1, 0, 10), // cardinal: cost 10
        (-1, -1, 14),
        (1, -1, 14),
        (-1, 1, 14),
        (1, 1, 14), // diagonal: cost 14
    ];

    let mut open = BinaryHeap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();

    g_score.insert(start, 0);
    open.push(Reverse((heuristic(start), start)));

    while let Some(Reverse((_, current))) = open.pop() {
        if current == goal {
            let mut path = vec![current];
            let mut node = current;
            while let Some(&prev) = came_from.get(&node) {
                path.push(prev);
                node = prev;
            }
            path.reverse();
            return path;
        }

        let g = *g_score.get(&current).unwrap_or(&i32::MAX);

        for &(ndx, ndy, cost) in &DIRS {
            let next = (current.0 + ndx, current.1 + ndy);
            if !is_walk(next.0, next.1) {
                continue;
            }

            // Edge blocking: walls block crossing, but doors are passable (pleb opens them)
            if ndx == 0 || ndy == 0 {
                if wd_edge_blocked_ignore_doors(wall_data, current.0, current.1, next.0, next.1) {
                    continue;
                }
                // Legacy block grid edge blocking
                if edge_blocked(grid, current.0, current.1, next.0, next.1) {
                    continue;
                }
            }

            // Diagonal: require both adjacent cardinal tiles to be walkable (no corner-cutting)
            if ndx != 0 && ndy != 0 {
                let cx = (current.0 + ndx, current.1);
                let cy = (current.0, current.1 + ndy);
                if !is_walk(cx.0, cx.1) || !is_walk(cy.0, cy.1) {
                    continue;
                }
                // Check edges along both cardinal steps of the diagonal (doors passable)
                if wd_edge_blocked_ignore_doors(wall_data, current.0, current.1, cx.0, cx.1)
                    || wd_edge_blocked_ignore_doors(wall_data, cx.0, cx.1, next.0, next.1)
                    || wd_edge_blocked_ignore_doors(wall_data, current.0, current.1, cy.0, cy.1)
                    || wd_edge_blocked_ignore_doors(wall_data, cy.0, cy.1, next.0, next.1)
                    || edge_blocked(grid, current.0, current.1, cx.0, cx.1)
                    || edge_blocked(grid, cx.0, cx.1, next.0, next.1)
                    || edge_blocked(grid, current.0, current.1, cy.0, cy.1)
                    || edge_blocked(grid, cy.0, cy.1, next.0, next.1)
                {
                    continue;
                }
            }

            // Elevation cost: uphill is harder, downhill slightly easier
            let elev_cost = if !elevation.is_empty() {
                let cur_idx = (current.1 as u32 * GRID_W + current.0 as u32) as usize;
                let nxt_idx = (next.1 as u32 * GRID_W + next.0 as u32) as usize;
                if cur_idx < elevation.len() && nxt_idx < elevation.len() {
                    let diff = elevation[nxt_idx] - elevation[cur_idx];
                    // Uphill: +3 cost per unit elevation. Downhill: -1 (slight benefit, clamped)
                    (diff * 3.0).max(-1.0) as i32
                } else {
                    0
                }
            } else {
                0
            };
            // Compaction bonus: well-trodden tiles are cheaper to traverse
            let compact_bonus = if !terrain.is_empty() {
                let nxt_idx = (next.1 as u32 * GRID_W + next.0 as u32) as usize;
                if nxt_idx < terrain.len() {
                    let compact = terrain_compaction(terrain[nxt_idx]);
                    // Up to -3 cost for fully compacted (31) tiles
                    -((compact as i32) * 3 / 31)
                } else {
                    0
                }
            } else {
                0
            };
            // Water cost: shallow water is expensive, deep water is impassable
            // water_table param doubles as precomputed water depth when elevation is empty
            let water_cost = if !water_table.is_empty() {
                let nxt_idx = (next.1 as u32 * GRID_W + next.0 as u32) as usize;
                if nxt_idx < water_table.len() {
                    let depth = if elevation.is_empty() {
                        // Precomputed depth (from water_depth_cpu)
                        water_table[nxt_idx]
                    } else if nxt_idx < elevation.len() {
                        (water_table[nxt_idx] + water_table_offset) - elevation[nxt_idx]
                    } else {
                        0.0
                    };
                    if depth > DEEP_WATER_THRESHOLD {
                        999 // impassable
                    } else if depth > 0.05 {
                        // Smooth quadratic cost: puddles cheap, deep water very expensive
                        // depth 0.1 → cost 5, depth 0.3 → cost 50, depth 0.6 → cost 250, depth 0.9 → cost 500
                        let t = ((depth - 0.05) / 0.95).min(1.0);
                        (t * t * 600.0) as i32 + 3
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            };
            let ng = g + cost + elev_cost + compact_bonus + water_cost;
            if ng < *g_score.get(&next).unwrap_or(&i32::MAX) {
                g_score.insert(next, ng);
                came_from.insert(next, current);
                open.push(Reverse((ng + heuristic(next), next)));
            }
        }
    }

    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::make_block;

    /// Create a small test grid. All dirt floor (walkable) with optional walls.
    fn test_grid(walls: &[(u32, u32)]) -> Vec<u32> {
        let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
        for &(x, y) in walls {
            grid[(y * GRID_W + x) as usize] = make_block(1, 3, 0); // stone wall
        }
        grid
    }

    #[test]
    fn test_astar_same_start_goal() {
        let grid = test_grid(&[]);
        let path = astar_path(&grid, (5, 5), (5, 5));
        assert_eq!(path, vec![(5, 5)]);
    }

    #[test]
    fn test_astar_straight_line() {
        let grid = test_grid(&[]);
        let path = astar_path(&grid, (5, 5), (5, 8));
        assert!(!path.is_empty());
        assert_eq!(path.first(), Some(&(5, 5)));
        assert_eq!(path.last(), Some(&(5, 8)));
        // Should be 4 steps (including start and goal)
        assert_eq!(path.len(), 4);
    }

    #[test]
    fn test_astar_around_wall() {
        // Wall blocking direct path from (5,5) to (5,8)
        let grid = test_grid(&[(5, 6), (5, 7)]);
        let path = astar_path(&grid, (5, 5), (5, 8));
        assert!(!path.is_empty());
        assert_eq!(path.last(), Some(&(5, 8)));
        // Path should go around (longer than 4)
        assert!(path.len() > 4);
        // Path should not contain wall tiles
        for &(px, py) in &path {
            assert!(!(px == 5 && (py == 6 || py == 7)), "path goes through wall");
        }
    }

    #[test]
    fn test_astar_unreachable() {
        // Completely walled-off goal
        let grid = test_grid(&[
            (9, 9),
            (10, 9),
            (11, 9),
            (9, 10),
            (11, 10),
            (9, 11),
            (10, 11),
            (11, 11),
        ]);
        let path = astar_path(&grid, (5, 5), (10, 10));
        assert!(path.is_empty(), "should be empty for unreachable goal");
    }

    #[test]
    fn test_astar_goal_is_wall() {
        let grid = test_grid(&[(10, 10)]);
        let path = astar_path(&grid, (5, 5), (10, 10));
        assert!(path.is_empty(), "should be empty when goal is a wall");
    }

    #[test]
    fn test_walkable_pos_open_ground() {
        let grid = test_grid(&[]);
        assert!(is_walkable_pos(&grid, 5.5, 5.5));
    }

    #[test]
    fn test_walkable_pos_wall() {
        let grid = test_grid(&[(5, 5)]);
        assert!(!is_walkable_pos(&grid, 5.5, 5.5));
    }

    #[test]
    fn test_walkable_pos_near_wall_edge() {
        let grid = test_grid(&[(5, 5)]);
        // Just outside the wall (pleb radius is 0.25)
        assert!(is_walkable_pos(&grid, 4.5, 5.5)); // left of wall
        // On the wall
        assert!(!is_walkable_pos(&grid, 5.2, 5.2));
    }

    #[test]
    fn test_walkable_pos_door() {
        let mut grid = test_grid(&[]);
        // Place a closed door (type 4, height 1, flag=door)
        grid[(5 * GRID_W + 5) as usize] = make_block(4, 1, 1);
        // Doors are walkable (plebs open them)
        assert!(is_walkable_pos(&grid, 5.5, 5.5));
    }

    #[test]
    fn test_appearance_deterministic() {
        let a1 = PlebAppearance::random(42);
        let a2 = PlebAppearance::random(42);
        assert_eq!(a1.skin_r, a2.skin_r);
        assert_eq!(a1.hair_r, a2.hair_r);
        assert_eq!(a1.shirt_r, a2.shirt_r);

        // Different seed = different appearance
        let a3 = PlebAppearance::random(99);
        assert_ne!(a1.skin_r, a3.skin_r);
    }
}

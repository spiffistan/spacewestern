//! Shared types, enums, and small structs used across the game.

use crate::grid::*;

// --- Game State ---

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameState {
    MainMenu,
    MapGen,
    Playing,
}

// --- Sandbox ---

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SandboxTool {
    None,
    Lightning,
    InjectWater,
    #[allow(dead_code)]
    TriggerDrought,
    Ignite,
    SoundPlace(usize), // index into SANDBOX_SOUNDS
}

/// Sandbox sound presets: (name, dB, frequency, pattern, duration)
pub const SANDBOX_SOUNDS: &[(&str, f32, f32, u32, f32)] = &[
    ("Whisper", 20.0, 0.0, 0, 0.3),      // soft impulse
    ("Conversation", 60.0, 0.0, 0, 0.2), // talking impulse
    ("Alarm Bell", 80.0, 8.0, 1, 5.0),   // continuous sine
    ("Lawnmower", 90.0, 4.0, 1, 4.0),    // low freq continuous
    ("Gunshot", 100.0, 0.0, 0, 0.05),    // sharp impulse
    ("Siren", 105.0, 12.0, 1, 6.0),      // high freq continuous
    ("Cannon", 110.0, 0.0, 0, 0.08),     // heavy impulse
    ("Thunder", 120.0, 0.0, 0, 0.2),     // rumbling impulse
    ("Grenade", 130.0, 0.0, 0, 0.15),    // explosion
    ("Explosion", 170.0, 0.0, 0, 0.25),  // massive blast
];

// --- Notifications & Conditions ---

/// Event notification (Rimworld-style right panel).
#[derive(Clone, Debug)]
pub struct GameNotification {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub category: NotifCategory,
    pub icon: &'static str,
    pub time_created: f32,
    pub dismissed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NotifCategory {
    Threat,   // red
    Warning,  // yellow
    Positive, // green
    #[allow(dead_code)]
    Info, // gray
}

impl NotifCategory {
    pub fn color(&self) -> egui::Color32 {
        match self {
            NotifCategory::Threat => egui::Color32::from_rgb(180, 40, 40),
            NotifCategory::Warning => egui::Color32::from_rgb(180, 160, 40),
            NotifCategory::Positive => egui::Color32::from_rgb(40, 160, 60),
            NotifCategory::Info => egui::Color32::from_rgb(80, 80, 90),
        }
    }
}

/// Active world condition with gameplay effects.
#[derive(Clone, Debug)]
pub struct ActiveCondition {
    pub id: u32,
    pub name: String,
    pub icon: &'static str,
    pub category: NotifCategory,
    pub remaining: f32, // game seconds remaining (0 = permanent until removed)
    pub duration: f32,  // total duration (for progress bar)
}

// --- Debug & Selection ---

/// GPU debug readback state (ctrl-hover info tool).
#[derive(Clone, Debug)]
pub struct DebugReadback {
    pub mode: bool,
    pub fluid_density: [f32; 4],
    pub block_temp: f32,
    pub block_temp_pending: bool,
    pub voltage: f32,
    pub voltage_pending: bool,
    pub fluid_pending: bool,
    pub water_level: f32,
    pub water_pending: bool,
}

impl Default for DebugReadback {
    fn default() -> Self {
        Self {
            mode: false,
            fluid_density: [0.0; 4],
            block_temp: 15.0,
            block_temp_pending: false,
            voltage: 0.0,
            voltage_pending: false,
            fluid_pending: false,
            water_level: 0.0,
            water_pending: false,
        }
    }
}

/// Which popup/slider is open for a selected block.
#[derive(Clone, Debug, Default)]
pub struct BlockSelection {
    pub pump: Option<u32>,
    pub pump_world: (f32, f32),
    pub fan: Option<u32>,
    pub fan_world: (f32, f32),
    pub dimmer: Option<u32>,
    pub dimmer_world: (f32, f32),
    pub cannon: Option<u32>,
    pub crate_idx: Option<u32>,
    pub crate_world: (f32, f32),
    pub workbench: Option<u32>, // grid_idx of open workbench popup
    pub workbench_world: (f32, f32),
}

/// A single selected item in the world.
#[derive(Clone, Debug)]
pub struct SelectedItem {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,                  // bounding box (grid coords)
    pub block_type: u32,         // 0 = pleb (not a block)
    pub pleb_idx: Option<usize>, // Some(idx) if this is a pleb
}

pub const SEL_PLEB: u32 = u32::MAX; // sentinel block_type for pleb selections

/// What's currently selected in the world (Rimworld-style).
#[derive(Clone, Debug, Default)]
pub struct WorldSelection {
    pub items: Vec<SelectedItem>,
}

impl WorldSelection {
    pub fn none() -> Self {
        WorldSelection { items: Vec::new() }
    }
    pub fn single(x: i32, y: i32, w: i32, h: i32, block_type: u32) -> Self {
        WorldSelection {
            items: vec![SelectedItem {
                x,
                y,
                w,
                h,
                block_type,
                pleb_idx: None,
            }],
        }
    }
    pub fn single_pleb(pleb_idx: usize, x: i32, y: i32) -> Self {
        WorldSelection {
            items: vec![SelectedItem {
                x,
                y,
                w: 1,
                h: 1,
                block_type: SEL_PLEB,
                pleb_idx: Some(pleb_idx),
            }],
        }
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// --- Event Bus ---

/// Typed game event — no string allocations, match-friendly for notifications.
#[derive(Clone, Debug)]
pub enum GameEventKind {
    // Weather
    WeatherChanged(&'static str),
    DroughtStarted,
    DroughtEnded(String),
    Lightning(i32, i32),
    FireConsumed(i32, i32),

    // Pleb needs / crisis
    CrisisStarted {
        pleb: String,
        reason: &'static str,
    },
    MentalBreak {
        pleb: String,
        kind: &'static str,
    },
    MentalBreakRecovered {
        pleb: String,
        kind: &'static str,
    },
    PlebDied(String),

    // Combat
    PlebHit {
        pleb: String,
        hp_pct: f32,
    },
    Explosion(f32, f32),

    // Hauling
    PickedUp {
        pleb: String,
        count: u16,
        item: String,
    },
    Delivered {
        pleb: String,
        material: &'static str,
        amount: u32,
    },
    Deposited(String),
    Dropped(String),
    Stored(String),
    AutoHauling(String),

    // Farming
    TaskAssigned {
        pleb: String,
        task: &'static str,
        x: i32,
        y: i32,
    },
    Planted(String),
    Harvested {
        pleb: String,
        what: &'static str,
    },
    DugClay {
        pleb: String,
        amount: u16,
    },

    // Building / crafting
    GoingToCraft(String),
    Crafting {
        pleb: String,
        recipe: String,
    },
    Crafted {
        pleb: String,
        recipe: String,
    },
    Built {
        pleb: String,
        block: String,
    },

    // Generic (for transitional use)
    Generic(EventCategory, String),
}

impl GameEventKind {
    /// Event category for log coloring.
    pub fn category(&self) -> EventCategory {
        match self {
            Self::WeatherChanged(_)
            | Self::DroughtStarted
            | Self::DroughtEnded(_)
            | Self::Lightning(_, _)
            | Self::FireConsumed(_, _) => EventCategory::Weather,
            Self::CrisisStarted { .. }
            | Self::MentalBreak { .. }
            | Self::MentalBreakRecovered { .. }
            | Self::PlebDied(_) => EventCategory::Need,
            Self::PlebHit { .. } | Self::Explosion(_, _) => EventCategory::Combat,
            Self::PickedUp { .. }
            | Self::Delivered { .. }
            | Self::Deposited(_)
            | Self::Dropped(_)
            | Self::Stored(_)
            | Self::AutoHauling(_) => EventCategory::Haul,
            Self::TaskAssigned { .. }
            | Self::Planted(_)
            | Self::Harvested { .. }
            | Self::DugClay { .. } => EventCategory::Farm,
            Self::GoingToCraft(_)
            | Self::Crafting { .. }
            | Self::Crafted { .. }
            | Self::Built { .. } => EventCategory::Build,
            Self::Generic(cat, _) => *cat,
        }
    }

    /// Human-readable message for the event log.
    pub fn message(&self) -> String {
        match self {
            Self::WeatherChanged(label) => label.to_string(),
            Self::DroughtStarted => "Drought has begun!".to_string(),
            Self::DroughtEnded(name) => format!("{} has ended", name),
            Self::Lightning(x, y) => format!("Lightning strike at ({}, {})", x, y),
            Self::FireConsumed(x, y) => format!("Fire consumed block at ({}, {})", x, y),
            Self::CrisisStarted { pleb, reason } => format!("{}: {}", pleb, reason),
            Self::MentalBreak { pleb, kind } => {
                format!("{} is having a mental break: {}!", pleb, kind)
            }
            Self::MentalBreakRecovered { pleb, kind } => {
                format!("{} recovered from {}", pleb, kind)
            }
            Self::PlebDied(pleb) => format!("{} has died!", pleb),
            Self::PlebHit { pleb, hp_pct } => format!("{} hit! ({:.0}% hp)", pleb, hp_pct),
            Self::Explosion(x, y) => format!("Explosion at ({:.0}, {:.0})", x, y),
            Self::PickedUp { pleb, count, item } => {
                format!("{} picked up {} {}", pleb, count, item)
            }
            Self::Delivered {
                pleb,
                material,
                amount,
            } => format!("{} delivered {} {}", pleb, amount, material),
            Self::Deposited(pleb) => format!("{} deposited items", pleb),
            Self::Dropped(pleb) => format!("{} dropped items (crate full)", pleb),
            Self::Stored(pleb) => format!("{} stored items", pleb),
            Self::AutoHauling(pleb) => format!("{} auto-hauling to crate", pleb),
            Self::TaskAssigned { pleb, task, x, y } => {
                format!("{} going to {} at ({},{})", pleb, task, x, y)
            }
            Self::Planted(pleb) => format!("{} planted a crop", pleb),
            Self::Harvested { pleb, what } => format!("{} harvested {}", pleb, what),
            Self::DugClay { pleb, amount } => format!("{} dug {} clay", pleb, amount),
            Self::GoingToCraft(pleb) => format!("{} going to craft", pleb),
            Self::Crafting { pleb, recipe } => format!("{} crafting {}", pleb, recipe),
            Self::Crafted { pleb, recipe } => format!("{} crafted {}", pleb, recipe),
            Self::Built { pleb, block } => format!("{} built {}", pleb, block),
            Self::Generic(_, msg) => msg.clone(),
        }
    }

    /// Should this event trigger a toast notification?
    pub fn notification(&self) -> Option<(NotifCategory, &'static str, &'static str)> {
        match self {
            Self::PlebDied(_) | Self::PlebHit { .. } | Self::Explosion(_, _) => {
                Some((NotifCategory::Threat, "\u{2694}", "Combat"))
            }
            Self::CrisisStarted { .. } | Self::MentalBreak { .. } => {
                Some((NotifCategory::Warning, "\u{26a0}", "Need"))
            }
            Self::Crafted { .. } | Self::Built { .. } => {
                Some((NotifCategory::Positive, "\u{2705}", "Complete"))
            }
            Self::DroughtStarted | Self::Lightning(_, _) => {
                Some((NotifCategory::Warning, "\u{26a1}", "Weather"))
            }
            _ => None,
        }
    }
}

/// In-game event log entry.
#[derive(Clone, Debug)]
pub struct GameEvent {
    pub time: f32,
    pub message: String,
    pub category: EventCategory,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventCategory {
    Farm,    // green
    Combat,  // red
    Need,    // yellow
    Build,   // blue
    Weather, // gray
    Haul,    // brown
    General, // white
}

impl EventCategory {
    pub fn icon(&self) -> &'static str {
        match self {
            EventCategory::Farm => "\u{1f33e}",
            EventCategory::Combat => "\u{2694}",
            EventCategory::Need => "\u{1f4a4}",
            EventCategory::Build => "\u{1f528}",
            EventCategory::Weather => "\u{26c8}",
            EventCategory::Haul => "\u{1f4e6}",
            EventCategory::General => "\u{2139}",
        }
    }
    pub fn color(&self) -> [u8; 3] {
        match self {
            EventCategory::Farm => [80, 200, 80],
            EventCategory::Combat => [255, 80, 80],
            EventCategory::Need => [220, 200, 60],
            EventCategory::Build => [80, 150, 255],
            EventCategory::Weather => [160, 160, 170],
            EventCategory::Haul => [180, 140, 80],
            EventCategory::General => [200, 200, 200],
        }
    }
}

pub const MAX_LOG_ENTRIES: usize = 100;

// --- Context Menu ---

/// A context menu action that can be performed.
#[derive(Clone)]
pub enum ContextAction {
    /// Send selected pleb to harvest a block at (grid_x, grid_y)
    Harvest(i32, i32),
    /// Haul a block/item at (grid_x, grid_y) to nearest crate
    Haul(i32, i32),
    /// Eat a ground item at index
    Eat(usize),
    /// Move selected pleb to world position
    MoveTo(f32, f32),
    /// Dig clay at (grid_x, grid_y) — pleb auto-fetches bucket
    DigClay(i32, i32),
    /// Hand-craft a recipe (recipe_id) — pleb crafts at current position
    HandCraft(u16),
    /// Gather branches from a tree without felling it (grid_x, grid_y)
    GatherBranches(i32, i32),
}

/// A context menu action entry: (label, action, enabled).
pub type MenuAction = (String, ContextAction, bool);

/// Unified context menu with a title, position, and list of labeled actions.
pub struct ContextMenu {
    pub screen_x: f32,
    pub screen_y: f32,
    pub title: String,
    pub actions: Vec<MenuAction>,
}

impl ContextMenu {
    pub fn new(sx: f32, sy: f32, title: impl Into<String>) -> Self {
        ContextMenu {
            screen_x: sx,
            screen_y: sy,
            title: title.into(),
            actions: Vec::new(),
        }
    }
    pub fn action(mut self, label: impl Into<String>, action: ContextAction) -> Self {
        self.actions.push((label.into(), action, true));
        self
    }
}

// --- Sound ---

/// An active sound source in the world.
#[derive(Clone, Debug)]
pub struct SoundSource {
    pub x: f32,
    pub y: f32,
    pub amplitude: f32,
    pub frequency: f32, // Hz (for sine pattern)
    pub phase: f32,     // accumulated phase
    pub pattern: u32,   // 0=impulse, 1=sine, 2=noise
    pub duration: f32,  // remaining seconds
}

/// Convert game decibels to wave equation amplitude.
/// Reference: 80 dB = amplitude 1.0 (alarm bell at source).
/// Uses compressed log scale: amp = 10^((dB - 80) / 40).
pub fn db_to_amplitude(db: f32) -> f32 {
    10.0f32.powf((db - 80.0) / 40.0)
}

/// Convert wave equation amplitude back to game decibels.
pub fn amplitude_to_db(amp: f32) -> f32 {
    if amp <= 0.0 {
        return 0.0;
    }
    80.0 + 40.0 * amp.log10()
}

// --- Craft Queue ---

/// A queued craft order on a workbench/kiln.
#[derive(Clone, Debug)]
pub struct CraftOrder {
    pub recipe_id: u16,
    pub count: u16,     // total to make
    pub completed: u16, // how many finished
}

/// Per-station craft queue. Stored in App::craft_queues keyed by grid_idx.
#[derive(Clone, Debug, Default)]
pub struct CraftQueue {
    pub orders: Vec<CraftOrder>,
}

impl CraftQueue {
    pub fn pending(&self) -> bool {
        self.orders.iter().any(|o| o.completed < o.count)
    }

    /// Get the next incomplete order.
    pub fn next_order(&self) -> Option<&CraftOrder> {
        self.orders.iter().find(|o| o.completed < o.count)
    }
}

// --- Blueprint ---

/// A pending construction — placed as a ghost, built by plebs over time.
#[derive(Clone, Debug)]
pub struct Blueprint {
    pub block_data: u32,      // target block (from make_block)
    pub progress: f32,        // 0.0-1.0 construction progress
    pub build_time: f32,      // total seconds to build
    pub wood_needed: u32,     // raw wood (logs) required
    pub wood_delivered: u32,  // raw wood deposited so far
    pub clay_needed: u32,     // clay required
    pub clay_delivered: u32,  // clay deposited so far
    pub plank_needed: u32,    // planks required (processed wood)
    pub plank_delivered: u32, // planks deposited so far
    pub rock_needed: u32,     // rock required
    pub rock_delivered: u32,  // rock deposited so far
    pub rope_needed: u32,     // rope required
    pub rope_delivered: u32,  // rope deposited so far
    // Wall blueprints: write to wall_data instead of grid_data
    pub wall_edges: u16,     // 0 = block blueprint, >0 = wall_data edges to place
    pub wall_thickness: u16, // wall_data thickness (1-4)
    pub wall_material: u16,  // wall_data material index
}

impl Blueprint {
    pub fn new(block_data: u32) -> Self {
        let bt = block_data & 0xFF;
        //                          (time, wood, clay, planks, rock, rope)
        let (build_time, wood, clay, planks, rock, rope) = match bt as u32 {
            //                     time  wood clay plnk rock rope
            // --- Floors ---
            BT_ROUGH_FLOOR => (0.8, 1, 0, 0, 0, 0),
            BT_WOOD_FLOOR => (1.5, 0, 0, 2, 0, 0),
            BT_STONE_FLOOR | BT_CONCRETE_FLOOR => (1.5, 0, 0, 0, 2, 0),
            // --- Walls ---
            BT_WOOD_WALL => (4.0, 2, 0, 0, 0, 0), // raw logs, no saw needed
            BT_MUD_WALL => (2.5, 0, 0, 0, 0, 0),  // auto-dug from ground, no material cost
            BT_STONE | BT_WALL | BT_SANDSTONE | BT_GRANITE | BT_LIMESTONE => (3.0, 0, 0, 0, 3, 0),
            BT_GLASS => (3.0, 0, 0, 0, 2, 0),
            BT_INSULATED => (4.0, 0, 2, 2, 0, 0),
            BT_STEEL_WALL => (4.0, 0, 0, 0, 4, 0),
            BT_DIAGONAL => (2.0, 0, 0, 0, 2, 0),
            // --- Furniture ---
            BT_BENCH => (2.0, 0, 0, 2, 0, 0),
            BT_BED => (3.0, 0, 0, 3, 0, 1), // rope for lacing
            BT_CRATE => (2.0, 0, 0, 2, 0, 0),
            // --- Crafting stations ---
            BT_WORKBENCH => (3.0, 0, 0, 4, 0, 0),
            BT_SAW_HORSE => (2.0, 2, 0, 0, 0, 0),
            BT_KILN => (8.0, 0, 10, 0, 0, 0),
            // --- Utilities ---
            BT_WELL => (8.0, 3, 0, 0, 2, 1), // rope to lower bucket
            BT_FIREPLACE => (1.0, 0, 0, 0, 0, 0), // campfire: 3 sticks (consumed on build)
            BT_CANNON => (5.0, 0, 0, 0, 6, 0),
            BT_COMPOST => (1.0, 1, 0, 0, 0, 0),
            // --- Lighting ---
            BT_FLOOR_LAMP => (1.0, 0, 0, 1, 0, 0),
            BT_TABLE_LAMP => (0.5, 0, 0, 0, 0, 0),
            BT_CEILING_LIGHT => (1.0, 0, 0, 1, 0, 0),
            BT_WALL_TORCH => (0.5, 1, 0, 0, 0, 0),
            BT_WALL_LAMP => (1.0, 0, 0, 1, 0, 0),
            BT_FLOODLIGHT => (2.0, 0, 0, 0, 2, 0),
            // --- Power ---
            BT_WIRE => (0.3, 0, 0, 0, 0, 0),
            BT_SOLAR => (3.0, 0, 0, 2, 1, 0),
            BT_BATTERY_S => (2.0, 0, 0, 1, 1, 0),
            BT_BATTERY_M => (3.0, 0, 0, 2, 2, 0),
            BT_BATTERY_L => (4.0, 0, 0, 3, 3, 0),
            BT_WIND_TURBINE => (5.0, 2, 0, 2, 0, 1), // rope for rigging
            BT_SWITCH | BT_DIMMER | BT_BREAKER => (0.5, 0, 0, 0, 0, 0),
            BT_WIRE_BRIDGE => (0.5, 0, 0, 0, 0, 0),
            // --- Gas piping ---
            BT_PIPE => (0.5, 0, 0, 0, 0, 0),
            BT_PUMP => (2.0, 0, 0, 1, 1, 0),
            BT_TANK => (3.0, 0, 0, 2, 1, 0),
            BT_VALVE | BT_RESTRICTOR => (1.0, 0, 0, 0, 0, 0),
            BT_OUTLET | BT_INLET => (0.5, 0, 0, 0, 0, 0),
            BT_FAN => (2.0, 0, 0, 1, 1, 0),
            BT_PIPE_BRIDGE => (0.5, 0, 0, 0, 0, 0),
            // --- Liquid piping ---
            BT_LIQUID_PIPE => (0.5, 0, 1, 0, 0, 0),
            BT_LIQUID_INTAKE | BT_LIQUID_OUTPUT => (1.0, 0, 1, 0, 0, 0),
            BT_LIQUID_PUMP => (2.0, 0, 0, 1, 1, 0),
            _ => (1.0, 0, 0, 0, 0, 0),
        };
        Blueprint {
            block_data,
            progress: 0.0,
            build_time,
            wood_needed: wood,
            wood_delivered: 0,
            clay_needed: clay,
            clay_delivered: 0,
            plank_needed: planks,
            plank_delivered: 0,
            rock_needed: rock,
            rock_delivered: 0,
            rope_needed: rope,
            rope_delivered: 0,
            wall_edges: 0,
            wall_thickness: 0,
            wall_material: 0,
        }
    }

    /// Create a wall blueprint (writes to wall_data on completion).
    pub fn new_wall(block_type: u32, edges: u16, thickness: u16, material: u16) -> Self {
        let mut bp = Self::new(make_block(block_type as u8, 0, 0));
        bp.wall_edges = edges;
        bp.wall_thickness = thickness;
        bp.wall_material = material;
        bp
    }

    /// Create a roof blueprint (requires 1 fiber, marks tile as roofed on completion).
    pub fn new_roof() -> Self {
        Blueprint {
            block_data: 0, // no block, just roof
            progress: 0.0,
            build_time: 0.5,
            wood_needed: 0,
            wood_delivered: 0,
            clay_needed: 0,
            clay_delivered: 0,
            plank_needed: 0,
            plank_delivered: 0,
            rock_needed: 0,
            rock_delivered: 0,
            rope_needed: 0,
            rope_delivered: 0,
            wall_edges: 0,
            wall_thickness: 0,
            wall_material: 0,
        }
    }

    pub fn is_roof(&self) -> bool {
        self.block_data == 0 && self.wall_edges == 0
    }

    pub fn is_campfire(&self) -> bool {
        (self.block_data & 0xFF) as u32 == BT_FIREPLACE && self.wall_edges == 0
    }

    pub fn is_wall(&self) -> bool {
        self.wall_edges != 0
    }

    pub fn resources_met(&self) -> bool {
        self.wood_delivered >= self.wood_needed
            && self.clay_delivered >= self.clay_needed
            && self.plank_delivered >= self.plank_needed
            && self.rock_delivered >= self.rock_needed
            && self.rope_delivered >= self.rope_needed
    }
}

// Door struct and constants are in grid.rs (to avoid circular dependency)

//! Physics bodies — moveable objects that interact with the fluid sim and plebs.
//! Projectile types are data-driven via ProjectileDef lookup table.

use crate::grid::*;
use std::sync::OnceLock;

// --- Data-driven projectile system ---

pub type ProjectileId = u16;

pub const PROJ_WOOD_BOX: ProjectileId = 0;
pub const PROJ_CANNONBALL: ProjectileId = 1;
pub const PROJ_GRENADE: ProjectileId = 2;
pub const PROJ_BULLET: ProjectileId = 3;
pub const PROJ_FRAGMENT: ProjectileId = 4;

/// How a projectile moves through the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TraversalMode {
    /// Normal arc: gravity, bounce, friction, wall collision.
    Ballistic,
    /// DDA ray march per frame (bullets, fast projectiles).
    Hitscan,
}

/// What happens when a projectile hits a wall or the ground.
#[derive(Clone, Debug)]
pub struct ImpactEffect {
    pub sound_db: f32,
    pub sound_duration: f32,
    pub destroy_multiplier: f32, // 0 = never destroys, 1 = normal KE check
    pub smoke_radius: f32,
    pub explosion: Option<ExplosionDef>,
    pub ricochet: bool,
    pub ricochet_loss: f32, // speed fraction lost per bounce (0.4 = lose 40%)
}

/// Continuous emission while on ground with fuse > 0.
#[derive(Clone, Debug)]
pub struct FuseEmission {
    pub duration: f32,
    pub gas: [f32; 4], // [smoke, O2, CO2, temp] injected per tick
    pub radius: i32,
    pub freeze_on_ground: bool,
}

/// One-time explosion event (blast wave, damage, force).
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ExplosionDef {
    pub radius: f32,   // effective radius (tiles)
    pub force: f32,    // impulse at epicenter (tiles/sec)
    pub damage: f32,   // health damage at epicenter (0-1)
    pub sound_db: f32, // detonation sound
    pub sound_duration: f32,
    pub block_ke: f32,    // KE applied to blocks for destruction
    pub fire_radius: f32, // ignite flammable blocks (0 = no fire)
}

/// Full definition of a projectile type.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ProjectileDef {
    pub name: &'static str,
    pub mass: f32,
    pub friction: f32,
    pub bounce: f32,
    pub size: f32,
    pub render_height: f32,
    pub max_speed: f32,
    pub traversal: TraversalMode,
    pub impact: ImpactEffect,
    pub fuse: Option<FuseEmission>,
    pub hit_damage: f32, // direct-hit damage (hitscan projectiles)
    pub remove_when_stopped: bool,
    pub remove_speed_threshold: f32,
}

/// Runtime explosion event emitted by tick_bodies.
#[derive(Debug)]
pub struct ExplosionEvent {
    pub x: f32,
    pub y: f32,
    pub def: ExplosionDef,
}

fn build_projectile_defs() -> Vec<ProjectileDef> {
    vec![
        // PROJ_WOOD_BOX (0)
        ProjectileDef {
            name: "Wood Box",
            mass: 20.0,
            friction: 0.85,
            bounce: 0.3,
            size: 0.45,
            render_height: 1.5,
            max_speed: 30.0,
            traversal: TraversalMode::Ballistic,
            impact: ImpactEffect {
                sound_db: 0.0,
                sound_duration: 0.0,
                destroy_multiplier: 0.0,
                smoke_radius: 0.0,
                explosion: None,
                ricochet: false,
                ricochet_loss: 0.0,
            },
            fuse: None,
            hit_damage: 0.0,
            remove_when_stopped: false,
            remove_speed_threshold: 0.0,
        },
        // PROJ_CANNONBALL (1)
        ProjectileDef {
            name: "Cannonball",
            mass: 5.0,
            friction: 0.6,
            bounce: 0.2,
            size: 0.12,
            render_height: 0.5,
            max_speed: 40.0,
            traversal: TraversalMode::Ballistic,
            impact: ImpactEffect {
                sound_db: 110.0,
                sound_duration: 0.08,
                destroy_multiplier: 1.0,
                smoke_radius: 2.0,
                explosion: None,
                ricochet: false,
                ricochet_loss: 0.0,
            },
            fuse: None,
            hit_damage: 0.0,
            remove_when_stopped: true,
            remove_speed_threshold: 0.5,
        },
        // PROJ_GRENADE (2) — frag grenade: heavy, bouncy, delayed fuse
        ProjectileDef {
            name: "Frag Grenade",
            mass: 1.5,     // heavy iron casing
            friction: 0.9, // high friction — stops rolling quickly
            bounce: 0.25,  // bounces a few times with thud
            size: 0.07,
            render_height: 0.25,
            max_speed: 18.0, // slower throw than before
            traversal: TraversalMode::Ballistic,
            impact: ImpactEffect {
                sound_db: 65.0, // heavy landing thud
                sound_duration: 0.06,
                destroy_multiplier: 0.0,
                smoke_radius: 0.0,
                explosion: None, // detonates on fuse expiry, not on landing
                ricochet: false,
                ricochet_loss: 0.0,
            },
            fuse: Some(FuseEmission {
                duration: 4.0,              // 4 second fuse
                gas: [0.03, 0.0, 0.0, 0.0], // faint smoke wisp only, no CO2
                radius: 0,
                freeze_on_ground: false, // rolls to a stop naturally via friction
            }),
            hit_damage: 0.0,
            remove_when_stopped: false,
            remove_speed_threshold: 0.0,
        },
        // PROJ_BULLET (3)
        ProjectileDef {
            name: "Bullet",
            mass: 0.01,
            friction: 0.0,
            bounce: 0.0,
            size: 0.02,
            render_height: 0.05,
            max_speed: 120.0,
            traversal: TraversalMode::Ballistic,
            impact: ImpactEffect {
                sound_db: 70.0,
                sound_duration: 0.05,
                destroy_multiplier: 0.0,
                smoke_radius: 0.0,
                explosion: None,
                ricochet: true,
                ricochet_loss: 0.4,
            },
            fuse: None,
            hit_damage: 0.2,
            remove_when_stopped: true,
            remove_speed_threshold: 1.0,
        },
        // PROJ_FRAGMENT (4) — grenade shrapnel
        ProjectileDef {
            name: "Fragment",
            mass: 0.05,
            friction: 0.2,
            bounce: 0.3, // ricochets off walls
            size: 0.03,
            render_height: 0.04,
            max_speed: 80.0,
            traversal: TraversalMode::Ballistic,
            impact: ImpactEffect {
                sound_db: 55.0, // metallic ping on wall hit
                sound_duration: 0.03,
                destroy_multiplier: 0.0,
                smoke_radius: 0.0,
                explosion: None,
                ricochet: true,
                ricochet_loss: 0.5, // loses 50% per bounce
            },
            fuse: None,
            hit_damage: 0.12, // slightly less than bullet (0.2)
            remove_when_stopped: true,
            remove_speed_threshold: 2.0, // dies faster than bullets
        },
    ]
}

static PROJECTILE_DEFS: OnceLock<Vec<ProjectileDef>> = OnceLock::new();

pub fn projectile_def(id: ProjectileId) -> &'static ProjectileDef {
    let defs = PROJECTILE_DEFS.get_or_init(build_projectile_defs);
    defs.get(id as usize).unwrap_or(&defs[0])
}

/// A physics body in the world (continuous position, not grid-aligned).
#[derive(Clone, Debug)]
pub struct PhysicsBody {
    pub x: f32,
    pub y: f32,
    pub z: f32, // height above ground (0 = on ground)
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,     // vertical velocity (positive = up)
    pub rot_x: f32,  // rotation around X axis (radians, tilts forward/back)
    pub rot_y: f32,  // rotation around Y axis (radians, tilts left/right)
    pub rot_z: f32,  // rotation around Z axis (radians, spins flat)
    pub spin_x: f32, // angular velocity around X
    pub spin_y: f32, // angular velocity around Y
    pub spin_z: f32, // angular velocity around Z
    pub mass: f32,
    pub friction: f32,
    pub bounce: f32,
    pub size: f32,
    pub render_height: f32,
    pub body_type: BodyType,
    pub kind: ProjectileId, // data-driven type (replaces body_type after migration)
    pub fuse_timer: f32,    // seconds remaining for fuse emission (0 = inactive)
    pub has_landed: bool,   // true after first ground contact (for one-time explosion)
    pub prev_x: f32,        // position at start of frame (for line-segment collision)
    pub prev_y: f32,
    pub shooter_pleb: Option<usize>, // pleb index who fired this (skip for self-hit)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyType {
    WoodBox,
    Cannonball,
    Grenade,
    Bullet,
    Fragment,
}

/// Result of a projectile impact.
#[derive(Debug)]
pub struct Impact {
    pub x: f32,
    pub y: f32,
    pub block_x: i32,
    pub block_y: i32,
    pub kinetic_energy: f32,
    pub destroy_block: bool,
    pub projectile_id: ProjectileId,
}

/// What kind of entity was hit by a bullet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HitTarget {
    Pleb(usize),
    Creature(usize),
}

/// Result of a bullet hitting an entity.
#[derive(Debug)]
pub struct BulletHit {
    pub target: HitTarget,
    pub x: f32,
    pub y: f32,
    pub kinetic_energy: f32,
    pub shooter: Option<usize>, // pleb index who fired (for kill credit)
}

impl PhysicsBody {
    pub fn new_wood_box(x: f32, y: f32) -> Self {
        PhysicsBody {
            x,
            y,
            z: 0.0,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
            spin_x: 0.0,
            spin_y: 0.0,
            spin_z: 0.0,
            mass: 20.0,
            friction: 0.85,
            bounce: 0.3,
            size: 0.45,
            render_height: 1.5,
            body_type: BodyType::WoodBox,
            kind: PROJ_WOOD_BOX,
            fuse_timer: 0.0,
            has_landed: false,
            prev_x: x,
            prev_y: y,
            shooter_pleb: None,
        }
    }

    /// Create a cannonball fired from position in a direction.
    pub fn new_cannonball(x: f32, y: f32, dir_x: f32, dir_y: f32) -> Self {
        let speed = 28.0; // tiles/sec horizontal
        let len = (dir_x * dir_x + dir_y * dir_y).sqrt().max(0.001);
        PhysicsBody {
            x,
            y,
            z: 1.5, // starts at cannon barrel height
            vx: dir_x / len * speed,
            vy: dir_y / len * speed,
            vz: 6.0, // upward arc
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
            spin_x: 0.0,
            spin_y: 0.0,
            spin_z: dir_y.atan2(dir_x) * 3.0, // spin around flight axis
            mass: 5.0,
            friction: 0.6,
            bounce: 0.2, // low bounce — cannonballs don't bounce much
            size: 0.12,
            render_height: 0.5,
            body_type: BodyType::Cannonball,
            kind: PROJ_CANNONBALL,
            fuse_timer: 0.0,
            has_landed: false,
            prev_x: x,
            prev_y: y,
            shooter_pleb: None,
        }
    }

    /// Create a toxic grenade thrown from position in a direction with given power (0-1).
    pub fn new_grenade(x: f32, y: f32, dir_x: f32, dir_y: f32, power: f32) -> Self {
        let speed = 6.0 + power * 14.0; // 6-20 tiles/sec
        let len = (dir_x * dir_x + dir_y * dir_y).sqrt().max(0.001);
        PhysicsBody {
            x,
            y,
            z: 1.2,
            vx: dir_x / len * speed,
            vy: dir_y / len * speed,
            vz: 6.0 + power * 8.0, // steep arc — lobs high
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
            spin_x: 3.0,
            spin_y: 2.0,
            spin_z: 5.0, // tumbles
            mass: 0.8,
            friction: 0.8,
            bounce: 0.3,
            size: 0.08,
            render_height: 0.3,
            body_type: BodyType::Grenade,
            kind: PROJ_GRENADE,
            fuse_timer: 4.0, // seconds until detonation after landing
            has_landed: false,
            prev_x: x,
            prev_y: y,
            shooter_pleb: None,
        }
    }

    /// Create a grenade aimed at a specific world position.
    /// `arc_height`: 0=flat, 1=medium, 2=lob. `strength`: 0-9 pleb stat.
    pub fn new_grenade_targeted(
        x: f32,
        y: f32,
        target_x: f32,
        target_y: f32,
        arc_height: u8,
        strength: f32,
    ) -> Self {
        let dx = target_x - x;
        let dy = target_y - y;
        let dist = (dx * dx + dy * dy).sqrt().max(0.1);
        let ndx = dx / dist;
        let ndy = dy / dist;

        // Elevation angle and max speed from arc setting + strength
        let (elev_angle, speed_mul) = match arc_height {
            0 => (0.3f32, 1.0f32), // flat: ~17°, full range
            2 => (1.0, 0.6),       // lob: ~57°, 60% range
            _ => (0.6, 0.85),      // medium: ~34°, 85% range
        };
        let max_speed = (15.0 + strength * 3.0) * speed_mul;
        let gravity = 25.0; // must match GRAVITY used in physics tick

        // Compute needed speed to reach target, compensating for air drag + launch height
        // Base: range = v² sin(2θ) / g, then scale up ~20% for air resistance loss
        let sin2a = (2.0 * elev_angle).sin().max(0.1);
        let needed_v = (dist * gravity / sin2a).sqrt(); // physics-accurate drag handles the rest
        let v = needed_v.min(max_speed);

        let vz = v * elev_angle.sin();
        let hvel = v * elev_angle.cos();

        PhysicsBody {
            x,
            y,
            z: 1.2,
            vx: ndx * hvel,
            vy: ndy * hvel,
            vz,
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
            spin_x: 3.0,
            spin_y: 2.0,
            spin_z: 5.0,
            mass: 0.8,
            friction: 0.8,
            bounce: 0.3,
            size: 0.08,
            render_height: 0.3,
            body_type: BodyType::Grenade,
            kind: PROJ_GRENADE,
            fuse_timer: 4.0,
            has_landed: false,
            prev_x: x,
            prev_y: y,
            shooter_pleb: None,
        }
    }

    /// Create a flat bullet (no aimed arc). Used for cannons and non-aimed fire.
    pub fn new_bullet(x: f32, y: f32, dir_x: f32, dir_y: f32) -> Self {
        Self::new_bullet_aimed(x, y, dir_x, dir_y, 1.0, 1.0, 20.0)
    }

    /// Create a bullet fired from position in a direction.
    /// `gun_z`: height of the gun muzzle. `target_dist`: distance to target.
    /// Computes an aimed arc so the bullet arrives at `target_z` at the target distance.
    pub fn new_bullet_aimed(
        x: f32,
        y: f32,
        dir_x: f32,
        dir_y: f32,
        gun_z: f32,
        target_z: f32,
        target_dist: f32,
    ) -> Self {
        let speed = 120.0;
        let len = (dir_x * dir_x + dir_y * dir_y).sqrt().max(0.001);
        let gravity = 25.0;
        // Time for bullet to reach target
        let t = (target_dist / speed).max(0.001);
        // vz needed: target_z = gun_z + vz*t - 0.5*g*t²
        // vz = (target_z - gun_z + 0.5*g*t²) / t
        let vz = (target_z - gun_z + 0.5 * gravity * t * t) / t;
        PhysicsBody {
            x,
            y,
            z: gun_z,
            vx: dir_x / len * speed,
            vy: dir_y / len * speed,
            vz,
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: dir_y.atan2(dir_x),
            spin_x: 0.0,
            spin_y: 0.0,
            spin_z: 0.0,
            mass: 0.01,
            friction: 0.0,
            bounce: 0.0,
            size: 0.02,
            render_height: 0.05,
            body_type: BodyType::Bullet,
            kind: PROJ_BULLET,
            fuse_timer: 0.0,
            has_landed: false,
            prev_x: x,
            prev_y: y,
            shooter_pleb: None,
        }
    }

    /// Create a fragment from an explosion at (x, y, z) with a random direction.
    /// `angle` = radial direction (0–TAU), `elev` = vertical angle (-0.3 to 0.5),
    /// `speed` = initial speed (varies per fragment).
    pub fn new_fragment(x: f32, y: f32, z: f32, angle: f32, elev: f32, speed: f32) -> Self {
        let vx = angle.cos() * speed * elev.cos();
        let vy = angle.sin() * speed * elev.cos();
        let vz = elev.sin() * speed;
        PhysicsBody {
            x,
            y,
            z: z.max(0.1),
            vx,
            vy,
            vz,
            rot_x: vx * 0.5,
            rot_y: vy * 0.5,
            rot_z: angle * 2.0,
            spin_x: speed * 0.3,
            spin_y: speed * 0.2,
            spin_z: speed * 0.5,
            mass: 0.05,
            friction: 0.2,
            bounce: 0.3,
            size: 0.03,
            render_height: 0.04,
            body_type: BodyType::Fragment,
            kind: PROJ_FRAGMENT,
            fuse_timer: 0.0,
            has_landed: false,
            prev_x: x,
            prev_y: y,
            shooter_pleb: None,
        }
    }

    /// Is this body on the ground?
    pub fn on_ground(&self) -> bool {
        self.z < 0.01
    }

    /// Throw the body in a direction with upward arc
    pub fn throw(&mut self, dx: f32, dy: f32, force: f32) {
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        self.vx = dx / len * force;
        self.vy = dy / len * force;
        self.vz = force * 0.5;
        self.z = 0.1;
        // Add tumbling spin proportional to force
        self.spin_x = (dy / len) * force * 0.5;
        self.spin_y = -(dx / len) * force * 0.5;
        self.spin_z = force * 0.3;
    }
}

/// Check if a physics body can occupy position (x, y) without overlapping walls.
pub fn body_can_move(grid: &[u32], x: f32, y: f32, size: f32) -> bool {
    body_can_move_z(grid, x, y, size, 2.0)
}

/// Like body_can_move but with a Z height check — low walls are passable if z > cover height.
pub fn body_can_move_z(grid: &[u32], x: f32, y: f32, size: f32, z: f32) -> bool {
    // Check 4 corners of bounding box
    for &(cx, cy) in &[
        (x - size, y - size),
        (x + size, y - size),
        (x - size, y + size),
        (x + size, y + size),
    ] {
        let bx = cx.floor() as i32;
        let by = cy.floor() as i32;
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
            return false;
        }
        let b = grid[(by as u32 * GRID_W + bx as u32) as usize];
        let bt = block_type_rs(b);
        let bh = block_height_rs(b) as u32;
        let is_door = (b >> 16) & 1 != 0;
        let is_open = (b >> 16) & 4 != 0;
        // Low walls: passable if body Z > cover height
        if bt == BT_LOW_WALL {
            if z > bh as f32 * 0.33 {
                continue;
            }
            return false; // bullet below cover = blocked
        }
        // Solid blocks that bodies can't pass through
        // Terrain (air=0, dirt=2, dug=32, rock=34) and furniture/plants are passable
        if bh > 0
            && !bt_is!(
                bt,
                BT_AIR,
                BT_GROUND,
                BT_DUG_GROUND,
                BT_ROCK,
                BT_FIREPLACE,
                BT_CEILING_LIGHT,
                BT_TREE,
                BT_FLOOR_LAMP,
                BT_TABLE_LAMP,
                BT_COMPOST,
                BT_BERRY_BUSH,
                BT_CROP
            )
            && !(is_door && is_open)
        {
            return false;
        }
    }
    true
}

/// Find the nearest ground-level body within range of position.
pub fn nearest_body(bodies: &[PhysicsBody], x: f32, y: f32, range: f32) -> Option<usize> {
    let mut best = None;
    let mut best_dist = range;
    for (i, body) in bodies.iter().enumerate() {
        if !body.on_ground() {
            continue;
        }
        let dist = ((x - body.x).powi(2) + (y - body.y).powi(2)).sqrt();
        if dist < best_dist {
            best_dist = dist;
            best = Some(i);
        }
    }
    best
}

/// DDA ray march through the grid from (x0,y0) to (x1,y1).
/// Returns the hit point if a solid wall is encountered, None if path is clear.
/// Steps through every grid cell the line segment crosses — no skips at any speed.
/// DDA bullet trace result
struct BulletTraceHit {
    x: f32,
    y: f32,           // hit position
    hit_x_face: bool, // true if hit a vertical face (reflect vx), false = horizontal face (reflect vy)
}

fn dda_bullet_trace(
    grid: &[u32],
    wall_data: &[u16],
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    z_start: f32,
    z_end: f32,
) -> Option<BulletTraceHit> {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 0.001 {
        return None;
    }

    let dir_x = dx / dist;
    let dir_y = dy / dist;

    let mut ix = x0.floor() as i32;
    let mut iy = y0.floor() as i32;

    let step_x: i32 = if dir_x >= 0.0 { 1 } else { -1 };
    let step_y: i32 = if dir_y >= 0.0 { 1 } else { -1 };

    let t_delta_x = if dir_x.abs() > 1e-6 {
        (1.0 / dir_x).abs()
    } else {
        f32::MAX
    };
    let t_delta_y = if dir_y.abs() > 1e-6 {
        (1.0 / dir_y).abs()
    } else {
        f32::MAX
    };

    let mut t_max_x = if dir_x > 1e-6 {
        ((ix as f32 + 1.0) - x0) / dir_x
    } else if dir_x < -1e-6 {
        (ix as f32 - x0) / dir_x
    } else {
        f32::MAX
    };

    let mut t_max_y = if dir_y > 1e-6 {
        ((iy as f32 + 1.0) - y0) / dir_y
    } else if dir_y < -1e-6 {
        (iy as f32 - y0) / dir_y
    } else {
        f32::MAX
    };

    // Check cells along the ray until we reach the endpoint or hit something
    for _ in 0..256 {
        // safety limit
        // Out of bounds = hit
        if ix < 0 || iy < 0 || ix >= GRID_W as i32 || iy >= GRID_H as i32 {
            let t = t_max_x.min(t_max_y).min(dist);
            return Some(BulletTraceHit {
                x: x0 + dir_x * t,
                y: y0 + dir_y * t,
                hit_x_face: t_max_x < t_max_y,
            });
        }

        let block = grid[(iy as u32 * GRID_W + ix as u32) as usize];
        let bt = block_type_rs(block);
        let bh = block_height_rs(block) as u32;
        let is_door = (block >> 16) & 1 != 0;
        let is_open = (block >> 16) & 4 != 0;

        // Bullet stops on: solid blocks with height
        // Passable: terrain, vegetation, furniture
        if bh > 0 {
            // Z-aware: bullet flies over if Z > block height
            let t_frac = (t_max_x.min(t_max_y) / dist).clamp(0.0, 1.0);
            let bullet_z_here = z_start + (z_end - z_start) * t_frac;
            // Map block height to physical Z: low walls (h=1) = 0.5, full walls (h=3) = 3.0
            let block_h = match bh {
                1 => 0.5,
                2 => 1.5,
                h => h as f32,
            };
            if bullet_z_here > block_h {
                // Bullet is above this block, skip
            } else {
                let passable = !is_wall_block(bt);
                let is_thin = is_wall_block(bt) && thin_wall_is_walkable(block);
                #[allow(clippy::nonminimal_bool)]
                if !passable && !is_thin && !(is_door && is_open) {
                    let t = t_max_x.min(t_max_y).max(0.0);
                    let hit_x = t_max_x <= t_max_y;
                    return Some(BulletTraceHit {
                        x: x0 + dir_x * t,
                        y: y0 + dir_y * t,
                        hit_x_face: hit_x,
                    });
                }
            } // end else (bullet below block)
        }

        // Step to next cell
        let prev_ix = ix;
        let prev_iy = iy;
        if t_max_x < t_max_y {
            if t_max_x > dist {
                break;
            }
            ix += step_x;
            t_max_x += t_delta_x;
        } else {
            if t_max_y > dist {
                break;
            }
            iy += step_y;
            t_max_y += t_delta_y;
        }

        // Edge blocking at tile transition — Z-aware for wall height
        if edge_blocked_wd(grid, wall_data, prev_ix, prev_iy, ix, iy) {
            // Check wall height vs bullet Z — bullets fly over short walls
            let t_frac = (t_max_x.min(t_max_y) / dist).clamp(0.0, 1.0);
            let bullet_z = z_start + (z_end - z_start) * t_frac;
            // Get the wall height from wall_data on both tiles
            let wd_a = if prev_ix >= 0
                && prev_iy >= 0
                && prev_ix < GRID_W as i32
                && prev_iy < GRID_H as i32
            {
                let ai = (prev_iy as u32 * GRID_W + prev_ix as u32) as usize;
                if ai < wall_data.len() {
                    wall_data[ai]
                } else {
                    0
                }
            } else {
                0
            };
            let wd_b = if ix >= 0 && iy >= 0 && ix < GRID_W as i32 && iy < GRID_H as i32 {
                let bi = (iy as u32 * GRID_W + ix as u32) as usize;
                if bi < wall_data.len() {
                    wall_data[bi]
                } else {
                    0
                }
            } else {
                0
            };
            // Wall height from wall_data (edges with explicit height)
            let wd_h = wd_physical_height(wd_a).max(wd_physical_height(wd_b));
            // Also check grid block heights (for blocks placed in grid_data)
            let phys_h = |raw: u8| -> f32 {
                match raw {
                    1 => 0.5,
                    2 => 1.5,
                    h => h as f32,
                }
            };
            let grid_h_a = if prev_ix >= 0
                && prev_iy >= 0
                && prev_ix < GRID_W as i32
                && prev_iy < GRID_H as i32
            {
                let b = grid[(prev_iy as u32 * GRID_W + prev_ix as u32) as usize];
                if is_wall_block(block_type_rs(b)) {
                    phys_h(block_height_rs(b))
                } else {
                    0.0
                }
            } else {
                0.0
            };
            let grid_h_b = if ix >= 0 && iy >= 0 && ix < GRID_W as i32 && iy < GRID_H as i32 {
                let b = grid[(iy as u32 * GRID_W + ix as u32) as usize];
                if is_wall_block(block_type_rs(b)) {
                    phys_h(block_height_rs(b))
                } else {
                    0.0
                }
            } else {
                0.0
            };
            let wall_h = wd_h.max(grid_h_a).max(grid_h_b);

            if bullet_z <= wall_h {
                let t = t_max_x.min(t_max_y).max(0.0);
                let hit_x = ix != prev_ix;
                return Some(BulletTraceHit {
                    x: x0 + dir_x * t,
                    y: y0 + dir_y * t,
                    hit_x_face: hit_x,
                });
            }
        }
    }

    None // clear path
}

/// Tick all physics bodies. Returns impacts, bullet hits, and explosion events.
pub fn tick_bodies(
    bodies: &mut Vec<PhysicsBody>,
    dt: f32,
    grid: &[u32],
    wall_data: &[u16],
    wind_x: f32,
    wind_y: f32,
    pleb: Option<(f32, f32, f32, f32, f32)>, // (pleb_x, pleb_y, pleb_vx, pleb_vy, pleb_angle)
    all_plebs: &[(f32, f32, usize, f32)],    // (x, y, pleb_index, z_height) for bullet collision
    all_creatures: &[(f32, f32, usize, f32)], // (x, y, creature_index, radius) for bullet collision
    selected_pleb: Option<usize>,
    ricochets_enabled: bool,
    sound_sources: &[(f32, f32, f32)], // (x, y, amplitude) for sound→body force
) -> (Vec<Impact>, Vec<BulletHit>, Vec<ExplosionEvent>) {
    let mut impacts = Vec::new();
    let mut explosions = Vec::new();
    let wind_threshold = 5.0; // minimum wind speed to push a box
    let gravity = 25.0; // tiles/sec² downward

    for body in bodies.iter_mut() {
        // Save position before physics update for accurate collision line segments
        body.prev_x = body.x;
        body.prev_y = body.y;

        let def = projectile_def(body.kind);

        // --- Wind force ---
        // Use global wind as approximation (actual fluid velocity sampling would need GPU readback)
        let wind_speed = (wind_x * wind_x + wind_y * wind_y).sqrt();
        if wind_speed > wind_threshold {
            let wind_force = (wind_speed - wind_threshold) * 0.02 / body.mass;
            body.vx += wind_x * wind_force * dt;
            body.vy += wind_y * wind_force * dt;
        }

        // --- Fan force ---
        // Check adjacent grid cells for fans (type 12) and apply their force
        let bx = body.x.floor() as i32;
        let by = body.y.floor() as i32;
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                let nx = bx + dx;
                let ny = by + dy;
                if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
                    continue;
                }
                let b = grid[(ny as u32 * GRID_W + nx as u32) as usize];
                let bt = block_type_rs(b);
                if bt == BT_FAN {
                    // fan
                    let dist = ((nx as f32 + 0.5 - body.x).powi(2)
                        + (ny as f32 + 0.5 - body.y).powi(2))
                    .sqrt();
                    if dist < 2.5 {
                        let dir_bits = ((b >> 16) >> 3) & 3;
                        let (fdx, fdy) = match dir_bits {
                            0 => (0.0f32, -1.0f32),
                            1 => (1.0, 0.0),
                            2 => (0.0, 1.0),
                            _ => (-1.0, 0.0),
                        };
                        let force = 15.0 / (1.0 + dist * 2.0) / body.mass;
                        body.vx += fdx * force * dt;
                        body.vy += fdy * force * dt;
                    }
                }
            }
        }

        // --- Sound pressure force ---
        for &(sx, sy, amplitude) in sound_sources {
            let sdx = body.x - sx;
            let sdy = body.y - sy;
            let dist = (sdx * sdx + sdy * sdy).sqrt().max(0.5);
            let pressure = amplitude / dist; // cylindrical falloff, same as pleb damage
            let force_threshold = 1.0; // ~80 dB minimum to push
            if pressure > force_threshold {
                let push = (pressure - force_threshold) * 0.08 / body.mass;
                let nx = sdx / dist;
                let ny = sdy / dist;
                body.vx += nx * push * dt;
                body.vy += ny * push * dt;
            }
        }

        // --- Pleb pushing (hard push in Jeff's facing direction) ---
        if let Some((px, py, pvx, pvy, pangle)) = pleb {
            let ddx = body.x - px;
            let ddy = body.y - py;
            let dist = (ddx * ddx + ddy * ddy).sqrt();
            let push_range = 0.75;
            if dist < push_range && dist > 0.01 && body.on_ground() {
                let pleb_speed = (pvx * pvx + pvy * pvy).sqrt();
                if pleb_speed > 0.1 {
                    let face_x = pangle.cos();
                    let face_y = pangle.sin();
                    let push_force = 8.0;
                    body.vx += face_x * push_force * dt;
                    body.vy += face_y * push_force * dt;
                    // Add slight spin from push
                    body.spin_z += push_force * 0.1 * dt;
                    body.spin_x += face_y * push_force * 0.05 * dt;
                    body.spin_y -= face_x * push_force * 0.05 * dt;
                }
                // Separation force (prevent overlap)
                let overlap = push_range - dist;
                if overlap > 0.0 {
                    body.vx += (ddx / dist) * overlap * 8.0 * dt;
                    body.vy += (ddy / dist) * overlap * 8.0 * dt;
                }
            }
        }

        // --- Body-body collision ---
        // TODO: O(n²) check for now, fine for small counts

        // --- Rotation update ---
        body.rot_x += body.spin_x * dt;
        body.rot_y += body.spin_y * dt;
        body.rot_z += body.spin_z * dt;

        // Angular friction (spin slows down)
        if body.on_ground() {
            body.spin_x *= 1.0 - body.friction * dt * 4.0;
            body.spin_y *= 1.0 - body.friction * dt * 4.0;
            body.spin_z *= 1.0 - body.friction * dt * 4.0;
            // Snap rotation to nearest 90° when nearly stopped and on ground
            let spin_total = body.spin_x.abs() + body.spin_y.abs() + body.spin_z.abs();
            if spin_total < 0.1 {
                body.spin_x = 0.0;
                body.spin_y = 0.0;
                body.spin_z = 0.0;
            }
        } else {
            // Air: very light spin damping
            body.spin_x *= 1.0 - 0.05 * dt;
            body.spin_y *= 1.0 - 0.05 * dt;
            body.spin_z *= 1.0 - 0.05 * dt;
        }

        // --- Gravity (Z axis) ---
        body.vz -= gravity * dt;
        body.z += body.vz * dt;

        // --- Bounce when hitting ground ---
        if body.z <= 0.0 {
            body.z = 0.0;
            if body.bounce < 0.01 {
                // No bounce (bullets): stop completely on ground impact
                body.vx = 0.0;
                body.vy = 0.0;
                body.vz = 0.0;
                body.has_landed = true;
            } else if body.vz < -1.0 {
                body.vz = -body.vz * body.bounce;
                body.vx *= 0.8;
                body.vy *= 0.8;
                body.spin_x += body.vy * 0.3;
                body.spin_y -= body.vx * 0.3;
                // Don't set has_landed here — explosion check needs it false until detonation
                // has_landed is set by the explosion section below
            } else {
                body.vz = 0.0;
            }
        }

        // --- Friction (only when on ground) ---
        if body.on_ground() {
            body.vx *= 1.0 - body.friction * dt * 3.0;
            body.vy *= 1.0 - body.friction * dt * 3.0;
        } else {
            // Air drag: F = ½ ρ v² Cd A → deceleration ∝ v × Cd×A/m
            // ρ = 1.225 kg/m³, Cd ≈ 0.47 (sphere), A = π r²
            // Tiles ≈ 1m, so size is in meters. Simplified: drag_coeff = ρ×Cd×π / (2×m) × r²
            let area = std::f32::consts::PI * body.size * body.size;
            let drag_coeff = 1.225 * 0.47 * area / (2.0 * body.mass.max(0.01));
            let speed = (body.vx * body.vx + body.vy * body.vy).sqrt();
            let drag = drag_coeff * speed * dt;
            let damp = (1.0 - drag).max(0.95); // never lose more than 5% per frame
            body.vx *= damp;
            body.vy *= damp;
        }

        // --- Velocity cap ---
        let max_speed = def.max_speed;
        let speed = (body.vx * body.vx + body.vy * body.vy).sqrt();
        if speed > max_speed {
            body.vx *= max_speed / speed;
            body.vy *= max_speed / speed;
        }

        // --- Move with collision ---
        let nx = body.x + body.vx * dt;
        let ny = body.y + body.vy * dt;

        if body.z < 2.0 {
            // below wall height — collide
            let mut hit_wall_x = false;
            let mut hit_wall_y = false;

            // Fast bodies (>20 tiles/sec) and bullets: use DDA for accurate wall detection
            let move_dist = ((nx - body.x) * (nx - body.x) + (ny - body.y) * (ny - body.y)).sqrt();
            let use_dda = move_dist > 0.5 || body.kind == PROJ_BULLET;
            if use_dda && move_dist > 0.001 {
                // DDA trace: catches thin walls and prevents tunneling
                let z_end = body.z + body.vz * dt;
                if let Some(hit) =
                    dda_bullet_trace(grid, wall_data, body.x, body.y, nx, ny, body.z, z_end)
                {
                    if ricochets_enabled && def.impact.ricochet {
                        let keep = 1.0 - def.impact.ricochet_loss;
                        body.x = hit.x;
                        body.y = hit.y;
                        if hit.hit_x_face {
                            body.vx = -body.vx * keep;
                            body.vy *= keep;
                            body.x += if body.vx > 0.0 { 0.05 } else { -0.05 };
                            hit_wall_x = true; // path=DDA_RICOCHET_X (KE=111)
                        } else {
                            body.vy = -body.vy * keep;
                            body.vx *= keep;
                            body.y += if body.vy > 0.0 { 0.05 } else { -0.05 };
                            hit_wall_y = true; // path=DDA_RICOCHET_Y (KE=112)
                        }
                        if body.vx.abs() + body.vy.abs() < def.remove_speed_threshold {
                            body.vx = 0.0;
                            body.vy = 0.0;
                        }
                    } else {
                        body.x = hit.x;
                        body.y = hit.y;
                        hit_wall_x = true; // for impact detection below
                        body.vx *= -body.bounce;
                        body.vy *= -body.bounce;
                    }
                } else {
                    body.x = nx;
                    body.y = ny;
                }
            } else {
                // Slow bodies: simple single-tile collision (Z-aware for low walls)
                if body_can_move_z(grid, nx, body.y, body.size, body.z) {
                    body.x = nx;
                } else {
                    hit_wall_x = true;
                    body.vx *= -body.bounce;
                }
                if body_can_move_z(grid, body.x, ny, body.size, body.z) {
                    body.y = ny;
                } else {
                    hit_wall_y = true;
                    body.vy *= -body.bounce;
                }
            }

            // Projectile impact on wall hit (block destruction + sound)
            if (def.impact.destroy_multiplier > 0.0 || def.impact.sound_db > 0.0)
                && (hit_wall_x || hit_wall_y)
            {
                let ke = 0.5 * body.mass * speed * speed * def.impact.destroy_multiplier;
                let hit_bx = if hit_wall_x {
                    nx.floor() as i32
                } else {
                    body.x.floor() as i32
                };
                let hit_by = if hit_wall_y {
                    ny.floor() as i32
                } else {
                    body.y.floor() as i32
                };

                let mut destroy = false;
                if hit_bx >= 0 && hit_by >= 0 && hit_bx < GRID_W as i32 && hit_by < GRID_H as i32 {
                    let b = grid[(hit_by as u32 * GRID_W + hit_bx as u32) as usize];
                    let bt = block_type_rs(b);
                    let strength = match bt {
                        BT_GLASS => 5.0,
                        BT_BENCH => 10.0,
                        BT_WOOD_WALL => 20.0,
                        BT_STONE | BT_WALL | BT_SANDSTONE | BT_LIMESTONE => 100.0,
                        BT_GRANITE => 200.0,
                        BT_STEEL_WALL => 300.0,
                        BT_INSULATED => 500.0,
                        _ => 50.0,
                    };
                    destroy = ke > strength;
                }

                impacts.push(Impact {
                    x: body.x,
                    y: body.y,
                    block_x: hit_bx,
                    block_y: hit_by,
                    kinetic_energy: ke,
                    destroy_block: destroy,
                    projectile_id: body.kind,
                });
            }
        } else {
            // Airborne above walls — no collision
            body.x = nx;
            body.y = ny;
        }

        // Data-driven stop: mark for removal when speed drops below threshold
        if def.remove_when_stopped && body.on_ground() && speed < def.remove_speed_threshold {
            body.vx = 0.0;
            body.vy = 0.0;
        }

        // Stop if very slow and on ground
        if body.on_ground() && speed < 0.01 {
            body.vx = 0.0;
            body.vy = 0.0;
        }
    }

    // Fuse emission + explosion detection (data-driven)
    for body in bodies.iter_mut() {
        let def = projectile_def(body.kind);

        // One-time explosion on first landing
        if !body.has_landed
            && body.on_ground()
            && let Some(expl) = &def.impact.explosion
        {
            body.has_landed = true;
            explosions.push(ExplosionEvent {
                x: body.x,
                y: body.y,
                def: expl.clone(),
            });
        }

        // Continuous fuse emission while grounded
        if let Some(fuse) = &def.fuse
            && body.on_ground()
            && body.fuse_timer > 0.0
        {
            let was_positive = body.fuse_timer > 0.0;
            body.fuse_timer -= dt;
            if fuse.freeze_on_ground {
                body.vx = 0.0;
                body.vy = 0.0;
            }
            // Fuse expired → detonate
            if was_positive && body.fuse_timer <= 0.0 {
                // Frag grenade explosion
                let grenade_explosion = ExplosionDef {
                    radius: 5.0,
                    force: 30.0,
                    damage: 0.25,
                    sound_db: 135.0,
                    sound_duration: 0.3,
                    block_ke: 20.0,
                    fire_radius: 0.0, // no fire — prevents plebs fleeing from heat
                };
                explosions.push(ExplosionEvent {
                    x: body.x,
                    y: body.y,
                    def: grenade_explosion,
                });
                body.fuse_timer = 0.0; // prevent re-trigger
            } else {
                impacts.push(Impact {
                    x: body.x,
                    y: body.y,
                    block_x: body.x.floor() as i32,
                    block_y: body.y.floor() as i32,
                    kinetic_energy: 0.0,
                    destroy_block: false,
                    projectile_id: body.kind,
                });
            }
        }
    }

    // --- Bullet-entity collision (plebs + creatures) ---
    let mut bullet_hits = Vec::with_capacity(all_plebs.len() + all_creatures.len());
    let pleb_hit_radius = 0.45f32;
    let mut bullets_hit = std::collections::HashSet::new();
    for (bi, body) in bodies.iter().enumerate() {
        if bullets_hit.contains(&bi) {
            continue;
        }
        let def = projectile_def(body.kind);
        if def.hit_damage <= 0.0 {
            continue;
        }
        let bx0 = body.prev_x;
        let by0 = body.prev_y;
        let bx1 = body.x;
        let by1 = body.y;
        let seg_dx = bx1 - bx0;
        let seg_dy = by1 - by0;
        let seg_len_sq = seg_dx * seg_dx + seg_dy * seg_dy;
        let speed_sq = body.vx * body.vx + body.vy * body.vy;
        let ke = 0.5 * body.mass * speed_sq;

        // Check plebs
        for &(px, py, pi, pz_height) in all_plebs {
            // Skip the pleb who fired this bullet (no self-hit)
            if body.shooter_pleb == Some(pi) || Some(pi) == selected_pleb {
                continue;
            }
            let t = if seg_len_sq > 0.0001 {
                ((px - bx0) * seg_dx + (py - by0) * seg_dy) / seg_len_sq
            } else {
                0.0
            }
            .clamp(0.0, 1.0);
            let cx = bx0 + t * seg_dx;
            let cy = by0 + t * seg_dy;
            let dist = ((cx - px) * (cx - px) + (cy - py) * (cy - py)).sqrt();
            // Z-height check: bullet's current Z must be at or below pleb's height
            if dist < pleb_hit_radius && body.z <= pz_height {
                bullet_hits.push(BulletHit {
                    target: HitTarget::Pleb(pi),
                    x: cx,
                    y: cy,
                    kinetic_energy: ke,
                    shooter: body.shooter_pleb,
                });
                bullets_hit.insert(bi);
                break;
            }
        }
        if bullets_hit.contains(&bi) {
            continue;
        }

        // Check creatures
        for &(cx_pos, cy_pos, ci, c_radius) in all_creatures {
            let t = if seg_len_sq > 0.0001 {
                ((cx_pos - bx0) * seg_dx + (cy_pos - by0) * seg_dy) / seg_len_sq
            } else {
                0.0
            }
            .clamp(0.0, 1.0);
            let cx = bx0 + t * seg_dx;
            let cy = by0 + t * seg_dy;
            let dist = ((cx - cx_pos) * (cx - cx_pos) + (cy - cy_pos) * (cy - cy_pos)).sqrt();
            if dist < c_radius.max(0.2) {
                bullet_hits.push(BulletHit {
                    target: HitTarget::Creature(ci),
                    x: cx,
                    y: cy,
                    kinetic_energy: ke,
                    shooter: body.shooter_pleb,
                });
                bullets_hit.insert(bi);
                break;
            }
        }
    }
    // Remove bullets that hit entities
    if !bullets_hit.is_empty() {
        let mut idx = 0;
        bodies.retain(|_| {
            let keep = !bullets_hit.contains(&idx);
            idx += 1;
            keep
        });
    }

    // Remove projectiles: data-driven removal logic
    bodies.retain(|b| {
        let def = projectile_def(b.kind);
        let in_bounds = b.x > 0.0 && b.y > 0.0 && b.x < GRID_W as f32 && b.y < GRID_H as f32;
        if !in_bounds {
            return false;
        }

        // Fuse-based: remove when fuse expired and on ground
        if def.fuse.is_some() {
            return b.fuse_timer > 0.0 || !b.on_ground();
        }

        // Hitscan: remove when stopped or below ground
        if def.traversal == TraversalMode::Hitscan {
            let moving = (b.vx.abs() + b.vy.abs()) > def.remove_speed_threshold;
            return moving && b.z > -0.1;
        }

        // Ballistic with removal: remove when stopped on ground
        if def.remove_when_stopped {
            let moving = (b.vx.abs() + b.vy.abs()) > def.remove_speed_threshold || b.z > 0.1;
            return moving;
        }

        true // keep (e.g. WoodBox)
    });

    (impacts, bullet_hits, explosions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::*;

    /// Wall_data with height=1 (low cover) at tile, all edges
    fn make_low_wall_wd(wx: i32, wy: i32) -> (Vec<u32>, Vec<u16>) {
        let size = (GRID_W * GRID_H) as usize;
        let grid = vec![make_block(BT_AIR as u8, 0, 0); size];
        let mut wd = vec![0u16; size];
        let idx = (wy as u32 * GRID_W + wx as u32) as usize;
        wd[idx] = pack_wall_data(WD_EDGE_MASK, 4, WMAT_MUD) | (1u16 << WD_HEIGHT_SHIFT);
        (grid, wd)
    }

    /// Wall_data with height=0 (full/3.0) at tile, all edges
    fn make_full_wall_wd(wx: i32, wy: i32) -> (Vec<u32>, Vec<u16>) {
        let size = (GRID_W * GRID_H) as usize;
        let grid = vec![make_block(BT_AIR as u8, 0, 0); size];
        let mut wd = vec![0u16; size];
        let idx = (wy as u32 * GRID_W + wx as u32) as usize;
        wd[idx] = pack_wall_data(WD_EDGE_MASK, 4, WMAT_STONE);
        (grid, wd)
    }

    #[test]
    fn bullet_over_low_wall_horizontal() {
        let (grid, wd) = make_low_wall_wd(5, 5);
        let result = dda_bullet_trace(&grid, &wd, 3.5, 5.5, 8.5, 5.5, 1.5, 1.5);
        assert!(
            result.is_none(),
            "z=1.5 bullet should pass over height-1 wall"
        );
    }

    #[test]
    fn bullet_hits_low_wall_when_low_z() {
        let (grid, wd) = make_low_wall_wd(5, 5);
        let result = dda_bullet_trace(&grid, &wd, 3.5, 5.5, 8.5, 5.5, 0.5, 0.5);
        assert!(result.is_some(), "z=0.5 bullet should hit height-1 wall");
    }

    #[test]
    fn bullet_hits_full_wall() {
        let (grid, wd) = make_full_wall_wd(5, 5);
        let result = dda_bullet_trace(&grid, &wd, 3.5, 5.5, 8.5, 5.5, 1.0, 1.0);
        assert!(result.is_some(), "z=1.0 bullet should hit full-height wall");
    }

    #[test]
    fn bullet_over_full_wall_when_very_high() {
        let (grid, wd) = make_full_wall_wd(5, 5);
        let result = dda_bullet_trace(&grid, &wd, 3.5, 5.5, 8.5, 5.5, 4.0, 4.0);
        assert!(
            result.is_none(),
            "z=4.0 bullet should pass over full-height wall"
        );
    }

    #[test]
    fn bullet_over_low_wall_diagonal() {
        let (grid, wd) = make_low_wall_wd(5, 5);
        let result = dda_bullet_trace(&grid, &wd, 3.5, 3.5, 7.5, 7.5, 1.5, 1.5);
        assert!(
            result.is_none(),
            "diagonal bullet should pass over low wall"
        );
    }

    #[test]
    fn bullet_over_low_wall_adjacent() {
        let (grid, wd) = make_low_wall_wd(5, 5);
        let result = dda_bullet_trace(&grid, &wd, 4.8, 5.5, 6.5, 5.5, 1.5, 1.5);
        assert!(
            result.is_none(),
            "adjacent bullet should pass over low wall"
        );
    }

    #[test]
    fn bullet_over_low_wall_neighbor_edges() {
        let size = (GRID_W * GRID_H) as usize;
        let grid = vec![make_block(BT_AIR as u8, 0, 0); size];
        let mut wd = vec![0u16; size];
        let low = pack_wall_data(WD_EDGE_MASK, 4, WMAT_MUD) | (1u16 << WD_HEIGHT_SHIFT);
        let center = (5u32 * GRID_W + 5) as usize;
        let left = (5u32 * GRID_W + 4) as usize;
        let right = (5u32 * GRID_W + 6) as usize;
        wd[center] = low;
        wd[left] = pack_wall_data(WD_EDGE_E, 4, WMAT_MUD) | (1u16 << WD_HEIGHT_SHIFT);
        wd[right] = pack_wall_data(WD_EDGE_W, 4, WMAT_MUD) | (1u16 << WD_HEIGHT_SHIFT);
        let result = dda_bullet_trace(&grid, &wd, 3.5, 5.5, 8.5, 5.5, 1.5, 1.5);
        assert!(
            result.is_none(),
            "bullet should pass over low wall edges on neighbors"
        );
    }

    #[test]
    fn bullet_over_extracted_low_wall() {
        let size = (GRID_W * GRID_H) as usize;
        let mut grid = vec![make_block(BT_AIR as u8, 0, 0); size];
        let idx = (5u32 * GRID_W + 5) as usize;
        grid[idx] = make_block(BT_LOW_WALL as u8, 1, 0);
        let wd = extract_wall_data_from_grid(&grid);
        // extract should set height=1 in wall_data
        assert!(
            wd_height(wd[idx]) == 1,
            "extracted wall_data should have height=1"
        );
        let result = dda_bullet_trace(&grid, &wd, 3.5, 5.5, 8.5, 5.5, 1.5, 1.5);
        assert!(
            result.is_none(),
            "bullet should pass over extracted low wall"
        );
    }
}

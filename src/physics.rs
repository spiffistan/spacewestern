//! Physics bodies — moveable objects that interact with the fluid sim and plebs.

use crate::grid::*;

/// A physics body in the world (continuous position, not grid-aligned).
#[derive(Clone, Debug)]
pub struct PhysicsBody {
    pub x: f32,
    pub y: f32,
    pub z: f32,            // height above ground (0 = on ground)
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,           // vertical velocity (positive = up)
    pub rot_x: f32,        // rotation around X axis (radians, tilts forward/back)
    pub rot_y: f32,        // rotation around Y axis (radians, tilts left/right)
    pub rot_z: f32,        // rotation around Z axis (radians, spins flat)
    pub spin_x: f32,       // angular velocity around X
    pub spin_y: f32,       // angular velocity around Y
    pub spin_z: f32,       // angular velocity around Z
    pub mass: f32,
    pub friction: f32,
    pub bounce: f32,
    pub size: f32,
    pub render_height: f32,
    pub body_type: BodyType,
    pub fuse_timer: f32, // seconds remaining for grenade emission (0 = inactive)
    pub prev_x: f32,     // position at start of frame (for accurate line-segment collision)
    pub prev_y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyType {
    WoodBox,
    Cannonball,
    Grenade,
    Bullet,
}

/// Result of a projectile impact.
#[derive(Debug)]
pub struct Impact {
    pub x: f32, pub y: f32,
    pub block_x: i32, pub block_y: i32,
    pub kinetic_energy: f32,
    pub destroy_block: bool,
    pub is_grenade: bool,
}

/// Result of a bullet hitting a pleb.
#[derive(Debug)]
pub struct BulletHit {
    pub pleb_idx: usize,
    pub x: f32, pub y: f32,
}

impl PhysicsBody {
    pub fn new_wood_box(x: f32, y: f32) -> Self {
        PhysicsBody {
            x, y, z: 0.0,
            vx: 0.0, vy: 0.0, vz: 0.0,
            rot_x: 0.0, rot_y: 0.0, rot_z: 0.0,
            spin_x: 0.0, spin_y: 0.0, spin_z: 0.0,
            mass: 20.0,
            friction: 0.85,
            bounce: 0.3,
            size: 0.45,
            render_height: 1.5,
            body_type: BodyType::WoodBox, fuse_timer: 0.0, prev_x: x, prev_y: y,
        }
    }

    /// Create a cannonball fired from position in a direction.
    pub fn new_cannonball(x: f32, y: f32, dir_x: f32, dir_y: f32) -> Self {
        let speed = 28.0; // tiles/sec horizontal
        let len = (dir_x * dir_x + dir_y * dir_y).sqrt().max(0.001);
        PhysicsBody {
            x, y, z: 1.5, // starts at cannon barrel height
            vx: dir_x / len * speed,
            vy: dir_y / len * speed,
            vz: 6.0, // upward arc
            rot_x: 0.0, rot_y: 0.0, rot_z: 0.0,
            spin_x: 0.0, spin_y: 0.0,
            spin_z: dir_y.atan2(dir_x) * 3.0, // spin around flight axis
            mass: 5.0,
            friction: 0.6,
            bounce: 0.2, // low bounce — cannonballs don't bounce much
            size: 0.12,
            render_height: 0.5,
            body_type: BodyType::Cannonball, fuse_timer: 0.0, prev_x: x, prev_y: y,
        }
    }

    /// Create a toxic grenade thrown from position in a direction with given power (0-1).
    pub fn new_grenade(x: f32, y: f32, dir_x: f32, dir_y: f32, power: f32) -> Self {
        let speed = 8.0 + power * 14.0; // 8-22 tiles/sec based on charge
        let len = (dir_x * dir_x + dir_y * dir_y).sqrt().max(0.001);
        PhysicsBody {
            x, y, z: 1.2,
            vx: dir_x / len * speed,
            vy: dir_y / len * speed,
            vz: 4.0 + power * 6.0, // higher arc with more power
            rot_x: 0.0, rot_y: 0.0, rot_z: 0.0,
            spin_x: 3.0, spin_y: 2.0, spin_z: 5.0, // tumbles
            mass: 0.8,
            friction: 0.8,
            bounce: 0.3,
            size: 0.08,
            render_height: 0.3,
            body_type: BodyType::Grenade, fuse_timer: 12.0, prev_x: x, prev_y: y,
        }
    }

    /// Create a bullet fired from position in a direction.
    pub fn new_bullet(x: f32, y: f32, dir_x: f32, dir_y: f32) -> Self {
        let speed = 120.0; // very fast — crosses screen in ~2s
        let len = (dir_x * dir_x + dir_y * dir_y).sqrt().max(0.001);
        PhysicsBody {
            x, y, z: 1.0,
            vx: dir_x / len * speed,
            vy: dir_y / len * speed,
            vz: 0.0,
            rot_x: 0.0, rot_y: 0.0, rot_z: dir_y.atan2(dir_x),
            spin_x: 0.0, spin_y: 0.0, spin_z: 0.0,
            mass: 0.01,
            friction: 0.0,
            bounce: 0.0,
            size: 0.02,
            render_height: 0.05,
            body_type: BodyType::Bullet, fuse_timer: 0.0, prev_x: x, prev_y: y,
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
    // Check 4 corners of bounding box
    for &(cx, cy) in &[(x - size, y - size), (x + size, y - size), (x - size, y + size), (x + size, y + size)] {
        let bx = cx.floor() as i32;
        let by = cy.floor() as i32;
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 { return false; }
        let b = grid[(by as u32 * GRID_W + bx as u32) as usize];
        let bt = block_type_rs(b) as u32;
        let bh = (b >> 8) & 0xFF;
        let is_door = (b >> 16) & 1 != 0;
        let is_open = (b >> 16) & 4 != 0;
        // Solid blocks that bodies can't pass through
        if bh > 0 && !matches!(bt, 6 | 7 | 8 | 10 | 11 | 13 | 15 | 16 | 17 | 18) && !(is_door && is_open) {
            return false;
        }
    }
    true
}

/// Check if a pleb at (px, py) would collide with any ground-level physics body.
/// Returns adjusted position (pushed away from bodies).
pub fn pleb_body_collision(bodies: &[PhysicsBody], px: f32, py: f32) -> (f32, f32) {
    let pleb_r = 0.25;
    let mut ax = px;
    let mut ay = py;
    for body in bodies {
        if !body.on_ground() { continue; } // only collide with grounded boxes
        let ddx = ax - body.x;
        let ddy = ay - body.y;
        let dist = (ddx * ddx + ddy * ddy).sqrt();
        let min_dist = pleb_r + body.size;
        if dist < min_dist && dist > 0.001 {
            // Push pleb out
            let overlap = min_dist - dist;
            ax += (ddx / dist) * overlap;
            ay += (ddy / dist) * overlap;
        }
    }
    (ax, ay)
}

/// Find the nearest ground-level body within range of position.
pub fn nearest_body(bodies: &[PhysicsBody], x: f32, y: f32, range: f32) -> Option<usize> {
    let mut best = None;
    let mut best_dist = range;
    for (i, body) in bodies.iter().enumerate() {
        if !body.on_ground() { continue; }
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
    x: f32, y: f32,           // hit position
    hit_x_face: bool,         // true if hit a vertical face (reflect vx), false = horizontal face (reflect vy)
}

fn dda_bullet_trace(grid: &[u32], x0: f32, y0: f32, x1: f32, y1: f32) -> Option<BulletTraceHit> {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 0.001 { return None; }

    let dir_x = dx / dist;
    let dir_y = dy / dist;

    let mut ix = x0.floor() as i32;
    let mut iy = y0.floor() as i32;

    let step_x: i32 = if dir_x >= 0.0 { 1 } else { -1 };
    let step_y: i32 = if dir_y >= 0.0 { 1 } else { -1 };

    let t_delta_x = if dir_x.abs() > 1e-6 { (1.0 / dir_x).abs() } else { f32::MAX };
    let t_delta_y = if dir_y.abs() > 1e-6 { (1.0 / dir_y).abs() } else { f32::MAX };

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
    for _ in 0..256 { // safety limit
        // Out of bounds = hit
        if ix < 0 || iy < 0 || ix >= GRID_W as i32 || iy >= GRID_H as i32 {
            let t = t_max_x.min(t_max_y).min(dist);
            return Some(BulletTraceHit { x: x0 + dir_x * t, y: y0 + dir_y * t, hit_x_face: t_max_x < t_max_y });
        }

        let block = grid[(iy as u32 * GRID_W + ix as u32) as usize];
        let bt = block_type_rs(block) as u32;
        let bh = (block >> 8) & 0xFF;
        let is_door = (block >> 16) & 1 != 0;
        let is_open = (block >> 16) & 4 != 0;

        // Bullet stops on: solid blocks with height, except trees/fire/lights/bushes/open doors
        if bh > 0 && bt != BT_TREE && bt != BT_FIREPLACE && bt != BT_CEILING_LIGHT && bt != BT_FLOOR_LAMP && bt != BT_BERRY_BUSH
            && !(is_door && is_open)
        {
            let t = t_max_x.min(t_max_y).max(0.0);
            // Determine which face was hit: the last axis we stepped along
            let hit_x = t_max_x <= t_max_y;
            return Some(BulletTraceHit { x: x0 + dir_x * t, y: y0 + dir_y * t, hit_x_face: hit_x });
        }

        // Step to next cell
        if t_max_x < t_max_y {
            if t_max_x > dist { break; }
            ix += step_x;
            t_max_x += t_delta_x;
        } else {
            if t_max_y > dist { break; }
            iy += step_y;
            t_max_y += t_delta_y;
        }
    }

    None // clear path
}

/// Tick all physics bodies. Returns list of cannonball impacts for block destruction.
pub fn tick_bodies(
    bodies: &mut Vec<PhysicsBody>,
    dt: f32,
    grid: &[u32],
    wind_x: f32,
    wind_y: f32,
    pleb: Option<(f32, f32, f32, f32, f32)>, // (pleb_x, pleb_y, pleb_vx, pleb_vy, pleb_angle)
    all_plebs: &[(f32, f32, usize)], // (x, y, pleb_index) for bullet collision
    selected_pleb: Option<usize>,
    ricochets_enabled: bool,
) -> (Vec<Impact>, Vec<BulletHit>) {
    let mut impacts = Vec::new();
    let wind_threshold = 5.0; // minimum wind speed to push a box

    for body in bodies.iter_mut() {
        // Save position before physics update for accurate collision line segments
        body.prev_x = body.x;
        body.prev_y = body.y;

        // --- Bullet fast path: DDA ray march through grid (no skipped cells) ---
        if body.body_type == BodyType::Bullet {
            let x0 = body.x;
            let y0 = body.y;
            let x1 = body.x + body.vx * dt;
            let y1 = body.y + body.vy * dt;

            if let Some(hit) = dda_bullet_trace(grid, x0, y0, x1, y1) {
                if ricochets_enabled {
                    // Ricochet: reflect velocity off the wall face, lose 40% speed
                    body.x = hit.x;
                    body.y = hit.y;
                    if hit.hit_x_face {
                        body.vx = -body.vx * 0.6;
                        body.vy *= 0.6;
                        // Nudge away from wall to prevent sticking
                        body.x += if body.vx > 0.0 { 0.05 } else { -0.05 };
                    } else {
                        body.vy = -body.vy * 0.6;
                        body.vx *= 0.6;
                        body.y += if body.vy > 0.0 { 0.05 } else { -0.05 };
                    }
                    // Kill bullet if too slow after ricochet
                    if body.vx.abs() + body.vy.abs() < 3.0 {
                        body.vx = 0.0; body.vy = 0.0;
                    }
                } else {
                    body.vx = 0.0; body.vy = 0.0; // mark for removal (no ricochet)
                }
            } else {
                body.x = x1;
                body.y = y1;
            }
            continue;
        }

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
                if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 { continue; }
                let b = grid[(ny as u32 * GRID_W + nx as u32) as usize];
                let bt = block_type_rs(b) as u32;
                if bt == BT_FAN { // fan
                    let dist = ((nx as f32 + 0.5 - body.x).powi(2) + (ny as f32 + 0.5 - body.y).powi(2)).sqrt();
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
                body.spin_x = 0.0; body.spin_y = 0.0; body.spin_z = 0.0;
            }
        } else {
            // Air: very light spin damping
            body.spin_x *= 1.0 - 0.05 * dt;
            body.spin_y *= 1.0 - 0.05 * dt;
            body.spin_z *= 1.0 - 0.05 * dt;
        }

        // --- Gravity (Z axis) ---
        let gravity = 25.0; // tiles/sec² downward
        body.vz -= gravity * dt;
        body.z += body.vz * dt;

        // --- Bounce when hitting ground ---
        if body.z <= 0.0 {
            body.z = 0.0;
            if body.vz < -1.0 {
                body.vz = -body.vz * body.bounce;
                body.vx *= 0.8;
                body.vy *= 0.8;
                // Impact adds tumble spin from horizontal velocity
                body.spin_x += body.vy * 0.3;
                body.spin_y -= body.vx * 0.3;
            } else {
                body.vz = 0.0;
            }
        }

        // --- Friction (only when on ground) ---
        if body.on_ground() {
            body.vx *= 1.0 - body.friction * dt * 3.0;
            body.vy *= 1.0 - body.friction * dt * 3.0;
        } else {
            // Air resistance (much less)
            body.vx *= 1.0 - 0.1 * dt;
            body.vy *= 1.0 - 0.1 * dt;
        }

        // --- Velocity cap ---
        let max_speed = if body.body_type == BodyType::Cannonball { 40.0 } else { 30.0 };
        let speed = (body.vx * body.vx + body.vy * body.vy).sqrt();
        if speed > max_speed {
            body.vx *= max_speed / speed;
            body.vy *= max_speed / speed;
        }

        // --- Move with collision ---
        let nx = body.x + body.vx * dt;
        let ny = body.y + body.vy * dt;

        if body.z < 2.0 { // below wall height — collide
            let mut hit_wall_x = false;
            let mut hit_wall_y = false;

            if body_can_move(grid, nx, body.y, body.size) {
                body.x = nx;
            } else {
                hit_wall_x = true;
                body.vx *= -body.bounce;
            }
            if body_can_move(grid, body.x, ny, body.size) {
                body.y = ny;
            } else {
                hit_wall_y = true;
                body.vy *= -body.bounce;
            }

            // Cannonball impact on wall hit
            if body.body_type == BodyType::Cannonball && (hit_wall_x || hit_wall_y) {
                let ke = 0.5 * body.mass * speed * speed;
                let hit_bx = if hit_wall_x { nx.floor() as i32 } else { body.x.floor() as i32 };
                let hit_by = if hit_wall_y { ny.floor() as i32 } else { body.y.floor() as i32 };

                // Block strength lookup (simplified — uses material table concept)
                let mut destroy = false;
                if hit_bx >= 0 && hit_by >= 0 && hit_bx < GRID_W as i32 && hit_by < GRID_H as i32 {
                    let b = grid[(hit_by as u32 * GRID_W + hit_bx as u32) as usize];
                    let bt = block_type_rs(b) as u32;
                    let strength = match bt {
                        5 => 5.0,    // glass: very fragile
                        9 => 10.0,   // bench: fragile
                        21 => 20.0,  // wood wall: breakable
                        1 | 4 | 23 | 25 => 100.0, // stone/sandstone/limestone
                        24 => 200.0, // granite: very tough
                        22 => 300.0, // steel: nearly indestructible
                        14 => 500.0, // insulated: reinforced
                        _ => 50.0,
                    };
                    destroy = ke > strength;
                }

                impacts.push(Impact {
                    x: body.x, y: body.y,
                    block_x: hit_bx, block_y: hit_by,
                    kinetic_energy: ke,
                    destroy_block: destroy,
                    is_grenade: false,
                });
            }
        } else {
            // Airborne above walls — no collision
            body.x = nx;
            body.y = ny;
        }

        // Cannonball: stop and mark for removal when speed is very low
        if body.body_type == BodyType::Cannonball && body.on_ground() && speed < 0.5 {
            body.vx = 0.0;
            body.vy = 0.0;
        }

        // Stop if very slow and on ground
        if body.on_ground() && speed < 0.01 {
            body.vx = 0.0;
            body.vy = 0.0;
        }
    }

    // Grenade emission: continuously emit toxic gas while on ground with fuse remaining
    for body in bodies.iter_mut() {
        if body.body_type == BodyType::Grenade && body.on_ground() && body.fuse_timer > 0.0 {
            body.fuse_timer -= dt;
            // Stop movement once on ground (grenade sits and hisses)
            body.vx = 0.0;
            body.vy = 0.0;
            impacts.push(Impact {
                x: body.x, y: body.y,
                block_x: body.x.floor() as i32, block_y: body.y.floor() as i32,
                kinetic_energy: 0.0,
                destroy_block: false,
                is_grenade: true,
            });
        }
    }

    // --- Bullet-pleb collision (before retain removes stopped bullets) ---
    let mut bullet_hits = Vec::new();
    let hit_radius = 0.45f32;
    let mut bullets_hit = std::collections::HashSet::new();
    for (bi, body) in bodies.iter().enumerate() {
        if body.body_type != BodyType::Bullet { continue; }
        let bx0 = body.prev_x;
        let by0 = body.prev_y;
        let bx1 = body.x;
        let by1 = body.y;
        let seg_dx = bx1 - bx0;
        let seg_dy = by1 - by0;
        let seg_len_sq = seg_dx * seg_dx + seg_dy * seg_dy;
        for &(px, py, pi) in all_plebs {
            if Some(pi) == selected_pleb { continue; }
            let t = if seg_len_sq > 0.0001 {
                ((px - bx0) * seg_dx + (py - by0) * seg_dy) / seg_len_sq
            } else { 0.0 }.clamp(0.0, 1.0);
            let cx = bx0 + t * seg_dx;
            let cy = by0 + t * seg_dy;
            let dist = ((cx - px) * (cx - px) + (cy - py) * (cy - py)).sqrt();
            if dist < hit_radius {
                bullet_hits.push(BulletHit { pleb_idx: pi, x: cx, y: cy });
                bullets_hit.insert(bi);
                break;
            }
        }
    }
    // Remove bullets that hit plebs
    if !bullets_hit.is_empty() {
        let mut idx = 0;
        bodies.retain(|_| {
            let keep = !bullets_hit.contains(&idx);
            idx += 1;
            keep
        });
    }

    // Remove projectiles that are out of bounds, stopped, or fuse expired
    bodies.retain(|b| {
        if b.body_type == BodyType::Grenade {
            let in_bounds = b.x > 0.0 && b.y > 0.0 && b.x < GRID_W as f32 && b.y < GRID_H as f32;
            in_bounds && (b.fuse_timer > 0.0 || !b.on_ground())
        } else if b.body_type == BodyType::Bullet {
            let in_bounds = b.x > 0.0 && b.y > 0.0 && b.x < GRID_W as f32 && b.y < GRID_H as f32;
            let moving = (b.vx.abs() + b.vy.abs()) > 1.0;
            in_bounds && moving && b.z > -0.1
        } else if b.body_type == BodyType::Cannonball {
            let in_bounds = b.x > 0.0 && b.y > 0.0 && b.x < GRID_W as f32 && b.y < GRID_H as f32;
            let moving = (b.vx.abs() + b.vy.abs()) > 0.3 || b.z > 0.1;
            in_bounds && moving
        } else {
            true
        }
    });

    (impacts, bullet_hits)
}

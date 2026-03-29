# DN-011: Combat Rework — Kinematic Bullets, Spatial Hash, Entity Collision

## Status: Proposed

## Problem

The current combat system has three issues:

1. **Bullets are hitscan** — they instantly resolve via DDA ray trace. No visible travel time, no bullet drop, no wind drift. Can't have "bullets in the air."
2. **Only plebs are hit targets** — creatures have health but no collision detection with bullets.
3. **O(N×M) collision** — every bullet tests against every pleb. Won't scale to hundreds of bullets + entities.

## Solution

### 1. Kinematic Bullets

Switch all bullets from `TraversalMode::Hitscan` to `TraversalMode::Ballistic` with high speed. The ballistic physics path already handles gravity, wind, friction, wall collision (DDA), and bounce. Bullets just need:

- **Reduced speed**: 120 → 60 tiles/sec (visible travel, crosses screen in ~4s)
- **Drag coefficient**: New field on `ProjectileDef`. `decel = drag * speed²`. Bullets slow over distance.
- **Gravity**: Already exists in ballistic path (25 tiles/s²). At 60 tiles/sec, a bullet drops ~0.3 tiles over 20 tiles of travel — subtle but real.
- **Wind drift**: Already exists. Wind pushes bullets laterally. A crosswind shifts aim over distance.
- **Remove hitscan path entirely**: All projectiles use the same ballistic code. DDA trace per-frame handles wall collision for fast-moving bullets (2 tiles/frame at 60fps — no tunneling).

```
Before:  Bullet created → instant DDA → hit or miss → done
After:   Bullet created → lives in physics_bodies → moves each frame
         → DDA checks walls per step → spatial hash checks entities → eventually hits or decays
```

### 2. Spatial Hash Grid

A 64×64 grid of cells (each cell covers 4×4 tiles). O(1) insert, O(1) query for nearby entities.

```rust
pub struct SpatialHash {
    cells: Vec<Vec<EntityRef>>,  // 64×64 = 4096 cells
    cell_size: f32,              // 4.0 tiles
    width: u32,                  // 64
}

#[derive(Clone, Copy)]
pub struct EntityRef {
    pub kind: EntityKind,  // Pleb, Creature, Body
    pub index: usize,      // index into the respective Vec
    pub x: f32,
    pub y: f32,
    pub radius: f32,       // collision radius
}

pub enum EntityKind {
    Pleb,
    Creature,
    PhysicsBody,
}
```

**Per-frame flow:**
1. `spatial.clear()` — reset all cells
2. Insert all plebs: `spatial.insert(EntityRef { kind: Pleb, index: i, x: pleb.x, y: pleb.y, radius: 0.45 })`
3. Insert all creatures: `spatial.insert(EntityRef { kind: Creature, ... })`
4. For each bullet, query `spatial.query_radius(bullet.x, bullet.y, check_radius)` → candidates
5. Line-segment collision test on candidates only (existing closest-point-on-segment math)

**Performance**: 1000 bullets × ~4 candidates per cell = 4000 precise tests (vs 100K without spatial hash).

### 3. Unified Entity Collision

The collision loop in `tick_bodies` currently only checks `all_plebs: &[(f32, f32, usize)]`. Replace with the spatial hash query that returns mixed entity types:

```rust
for &candidate in spatial.query_near(bullet.x, bullet.y, search_radius) {
    // Line-segment collision test (existing math)
    let dist = closest_point_distance(prev_x, prev_y, x, y, candidate.x, candidate.y);
    if dist < candidate.radius {
        match candidate.kind {
            EntityKind::Pleb => hits.push(PlebHit { ... }),
            EntityKind::Creature => hits.push(CreatureHit { ... }),
            EntityKind::PhysicsBody => { /* body-body collision, future */ }
        }
        break; // one hit per bullet per frame
    }
}
```

### 4. Projectile Properties (Data-Driven)

Extend `ProjectileDef` with ballistic properties:

```rust
pub struct ProjectileDef {
    // ... existing fields ...
    pub drag: f32,          // air resistance coefficient (0.0 = none, 0.01 = light, 0.05 = heavy)
    pub penetration: f32,   // 0.0 = stops on hit, 1.0 = passes through (future)
    pub spread: f32,        // accuracy cone in radians (0 = perfect)
    pub pellets: u8,        // 1 = single shot, 8 = shotgun blast
    pub tracer: bool,       // visible trail effect
    pub muzzle_velocity: f32, // initial speed (replaces hardcoded 120.0)
}
```

**Weapon types** (future, but architecture supports them):
| Weapon | Speed | Drag | Damage | Pellets | Spread |
|--------|-------|------|--------|---------|--------|
| Pistol | 60 | 0.005 | 0.15 | 1 | 0.03 |
| Rifle | 90 | 0.002 | 0.30 | 1 | 0.01 |
| Shotgun | 40 | 0.02 | 0.08 | 8 | 0.12 |
| Cannon | 28 | 0.001 | 0.50 | 1 | 0.02 |

### 5. Bullet Lifecycle

```
Created → Flying → [Hit entity | Hit wall | Decayed | Left map]
                       ↓            ↓          ↓
                   Damage         Ricochet    Remove
                   Remove         or Impact
```

**Decay**: Bullets below a speed threshold (e.g., 5 tiles/sec) are removed. Drag naturally causes this over long distances. No infinite-range bullets.

**Ricochet**: Already implemented for hitscan. Works identically for ballistic — reflect velocity component, lose energy.

## Implementation Phases

### Phase 1: Spatial Hash
- New `SpatialHash` struct in `physics.rs`
- Build from plebs + creatures each frame
- Replace the linear collision loop with spatial query
- **No bullet physics changes yet** — just faster collision
- **Files**: physics.rs, simulation.rs

### Phase 2: Creature Collision
- Include creatures in spatial hash
- Add `CreatureHit` to tick_bodies return
- Apply damage to creatures on bullet hit
- Apply explosion damage/knockback to creatures
- **Files**: physics.rs, simulation.rs, creatures.rs

### Phase 3: Kinematic Bullets
- Remove `TraversalMode::Hitscan` path
- Add `drag`, `muzzle_velocity` to ProjectileDef
- Apply drag each frame: `speed *= 1.0 / (1.0 + drag * speed * dt)`
- Reduce bullet speed for visible travel (120 → 60)
- Use DDA per-frame for wall collision (already in ballistic path)
- Bullets affected by wind from fluid sim
- **Files**: physics.rs

### Phase 4: Weapon Variety (Future)
- Multiple projectile defs with different properties
- Shotgun spread (multiple pellets per shot)
- Penetration through thin materials
- Tracer visual effect
- **Files**: physics.rs, simulation.rs, raytrace.wgsl

## Spatial Hash Detail

```
256×256 tile world
÷ 4 tiles per cell
= 64×64 = 4096 cells

Cell lookup: cell_x = (world_x / 4.0).floor() as u32
             cell_y = (world_y / 4.0).floor() as u32
             cell_idx = cell_y * 64 + cell_x

Query radius: check cell + 8 neighbors (3×3 block)
For bullets moving >4 tiles/frame: check cells along path
```

**Memory**: Each cell is a `SmallVec<[EntityRef; 4]>` (stack-allocated for ≤4 entities per cell, heap for more). EntityRef is 20 bytes. 4096 cells × 4 entities × 20 bytes = ~320KB worst case. Tiny.

**Alternative**: Flat array of `Vec<EntityRef>` with `clear()` reuse (no allocation after first frame). Even simpler.

## Files to Modify

| File | Phase | Changes |
|------|-------|---------|
| `src/physics.rs` | 1,2,3 | SpatialHash struct, unified collision, ballistic bullets, drag |
| `src/simulation.rs` | 1,2 | Build spatial hash, process creature hits, explosion→creature |
| `src/creatures.rs` | 2 | Creature knockback fields, damage application |
| `src/creatures.toml` | 2 | Collision radius per species |

## Verification

1. Shoot at a pleb → bullet travels visibly, hits, damage applied
2. Shoot at a duskweaver → visible hit, creature takes damage, dies at 0 health
3. Grenade near creature → knockback + damage
4. 100+ bullets in the air simultaneously → no frame drop
5. Bullet in crosswind → visible lateral drift
6. Bullet at long range → drops slightly, slows from drag
7. Shotgun blast (future) → 8 pellets spread, some hit, some miss

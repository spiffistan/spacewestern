# Rayworld — Physics System Design

## Philosophy

Physics in Rayworld is not a rigid body engine. It is a **coupling layer** between the block grid, the fluid simulation, entities (plebs, projectiles, particles), and the lighting system. Everything that moves displaces air. Everything that heats up heats the gas around it. Every light source that passes through gas illuminates (and heats) it. The physics system makes the world feel *connected* — not a collection of independent subsystems.

The guiding principle: **if it exists in the world, it participates in the fluid sim**.

---

## Layer 1: Entity-Fluid Coupling

### Plebs Displace Air

A pleb is a physical body occupying ~0.7×0.7 tiles. As they move, they push air:

```
velocity_injection = pleb_velocity × pleb_cross_section × coupling_factor
```

In `fluid.wgsl` advect_velocity, at each cell overlapping the pleb's bounding box:
```wgsl
// Pleb displacement: moving body pushes air
let pleb_dist = length(vec2(f32(pos.x) + 0.5 - pleb_x, f32(pos.y) + 0.5 - pleb_y));
if pleb_dist < 0.5 {
    let pleb_vel = vec2(pleb_vx, pleb_vy);  // from FluidParams or separate buffer
    new_v += pleb_vel * 0.3 * (1.0 - pleb_dist / 0.5);  // Gaussian-ish falloff
}
```

Effects:
- A running pleb creates a wake in the smoke behind them
- Walking through a smoky room pushes smoke aside momentarily
- A pleb standing still acts as a mild fluid obstacle (partial blockage, not solid)

### Plebs as Partial Obstacles

Unlike walls (solid obstacles), plebs are **partial obstacles** — air flows around them but is slowed. In the obstacle field or a separate "softness" texture:
- Wall: obstacle = 1.0 (completely solid)
- Pleb body: obstacle = 0.5 (partial — air slows but passes)
- Open air: obstacle = 0.0

This requires either:
- A dynamic obstacle texture updated each frame with pleb positions (GPU compute)
- Or inline checks in the velocity shader using pleb position from CameraUniform/FluidParams

For a single pleb, inline checks are fine. For many plebs, a dynamic obstacle texture is better.

### Moving Blocks Displace Air

When a block is placed, removed, or moved (future: pistons, sliding doors, conveyor belts), the air in that cell must respond:

**Block placement**: The cell transitions from open to solid. Air is compressed outward:
- Inject outward velocity from the placed block's position
- The pressure solver naturally handles the compression wave

**Block removal**: The cell transitions from solid to open. Air rushes in:
- The void creates a low-pressure zone
- Surrounding air flows in (handled by pressure solver)
- If the block was hot (e.g., removing a wall from a furnace room), hot air explodes outward

**Sliding blocks** (future): A moving solid pushes air in its direction of travel and creates a vacuum behind it. This is equivalent to a moving obstacle in the NS solver — the obstacle field changes each frame.

### Doors as Pressure Valves

Doors already work as binary obstacles. Future enhancement: **sliding doors** that partially open, creating a narrow slit that accelerates airflow (Venturi effect). The obstacle field supports fractional values for this.

---

## Layer 2: Thermal Physics

### Lasers Heat Gas

A laser beam (future weapon/tool) travels in a straight line and heats everything it passes through:

```
For each cell along the beam:
    temperature[cell] += laser_power * dt
    smoke[cell] += laser_power * 0.01  // ionization glow
    if temperature > ignition_threshold:
        trigger chemical reactions (CH4, H2, etc.)
```

The heated gas then expands (buoyancy), creating convection currents visible in the smoke overlay. A laser cutting through a methane cloud → ignition → explosion → pressure wave.

**Laser-gas interaction in raytrace shader**: The beam is visible through smoke (volumetric scattering). The laser illuminates dust/smoke particles along its path. This is purely visual — sample smoke density along the ray and add glow proportional to density × laser intensity.

### Fire Heats Surrounding Air (already implemented)

Fire blocks inject heat (~300°C) into the temperature field (dye.a channel, future). The buoyancy system converts temperature differentials into velocity, creating convection. This is the foundation for all thermal physics.

### Explosion Pressure Waves

An explosion is a sudden injection of:
1. **High temperature** at the epicenter → radiates outward via fluid advection
2. **High pressure** (outward velocity) → creates a shockwave in the NS solver
3. **Smoke/debris** particles → advected by the blast velocity

```wgsl
// Explosion at (ex, ey) with radius r and power p
let dist = length(vec2(f32(pos.x) - ex, f32(pos.y) - ey));
if dist < r {
    let falloff = 1.0 - dist / r;
    let outward = normalize(vec2(f32(pos.x) - ex, f32(pos.y) - ey));
    new_v += outward * p * falloff;
    temperature[pos] += p * falloff * 500.0;  // heat
}
```

The NS solver then propagates the shockwave, which:
- Blows smoke away from the blast center
- Pushes air through doorways (blowing doors open?)
- Creates a partial vacuum at the center (implosion phase)
- Shatters glass blocks if pressure exceeds structural threshold

### Heat Conduction Through Solids

Blocks have thermal properties (defined in SPEC.md). Heat conducts through solid blocks slowly:

```
block_temp_new = block_temp + conductivity * (neighbor_avg_temp - block_temp) * dt
```

A hot wall radiates heat to adjacent air cells:
```
air_temp[adjacent] += (block_temp - air_temp) * radiation_rate * dt
```

This creates realistic thermal behavior: a furnace room's walls slowly heat up, then radiate warmth even after the fire dies. A cold stone wall absorbs heat from warm air, cooling the room.

---

## Layer 3: Particle Physics

### Particle Types

Small physical objects that interact with the fluid but don't occupy full grid cells:

| Particle | Size | Mass | Fluid Coupling | Source |
|----------|------|------|---------------|--------|
| Ember | tiny | 0.01 | strong (carried by wind) | Fire, explosions |
| Debris | small | 0.5 | medium (affected by wind) | Explosions, structural collapse |
| Raindrop | tiny | 0.02 | medium (falls, affected by wind) | Weather system |
| Snowflake | tiny | 0.005 | very strong (floats on wind) | Cold weather |
| Projectile | medium | 2.0 | weak (ballistic, slight drag) | Weapons |
| Thrown object | varies | varies | medium drag | Plebs (berserk break) |
| Leaf | tiny | 0.001 | very strong (floats on wind) | Trees (autumn) |
| Spark | tiny | 0.001 | strong | Metal grinding, electrical |

### Particle Simulation

Each particle has:
```rust
struct Particle {
    pos: Vec2,          // world position (continuous)
    vel: Vec2,          // velocity
    mass: f32,
    drag: f32,          // fluid coupling strength (0=no coupling, 1=fully carried by wind)
    lifetime: f32,      // seconds remaining
    particle_type: ParticleType,
    temperature: f32,   // for embers (hot particles can ignite gas)
}
```

Per-frame update:
```rust
// Sample fluid velocity at particle position
let fluid_vel = sample_velocity_field(particle.pos);
let fluid_drag = (fluid_vel - particle.vel) * particle.drag;

// Gravity (in top-down: slight downward bias, or radial from explosions)
let gravity = Vec2::new(0.0, 0.1 * particle.mass);

// Update
particle.vel += (fluid_drag + gravity) * dt;
particle.pos += particle.vel * dt;
particle.lifetime -= dt;

// Two-way coupling: particle injects velocity back into fluid
inject_velocity_at(particle.pos, particle.vel * particle.mass * 0.01);

// Hot particles heat the gas
if particle.temperature > 100.0 {
    inject_temperature_at(particle.pos, particle.temperature * 0.01 * dt);
}
```

### Particle-Fluid Two-Way Coupling

**Fluid → Particle**: Particles are carried by the fluid velocity field (wind, convection, explosion blasts). Drag coefficient determines how strongly.

**Particle → Fluid**: Particles inject small velocity impulses back into the fluid. A dense cloud of debris creates its own air current. Hot embers heat the gas they pass through.

**Particle → Gas Chemistry**: A hot ember (>580°C) passing through a methane cloud triggers ignition. A spark particle in a hydrogen-rich area causes explosion.

### Particle Collision

Particles collide with the block grid:
- **Wall hit**: bounce with energy loss (`vel *= -restitution`), or embed in wall
- **Water hit**: splash particles, slow down, sink
- **Glass hit**: if velocity > threshold, break the glass block
- **Pleb hit**: damage (projectiles), or just bounce (debris)

### GPU vs CPU Particles

For < 100 particles: CPU simulation, render via shader using a particle buffer.
For 100-10000 particles: GPU compute shader (one thread per particle), two-way fluid coupling via atomic operations or scatter-gather.

The particle buffer is a storage buffer read by the raytrace shader:
```wgsl
struct PackedParticle {
    pos_x: f32, pos_y: f32,
    color_packed: u32,    // RGBA8
    size_lifetime: u32,   // size:f16, lifetime:f16
};
```

Raytrace shader checks if the current pixel is near any particle (spatial hash or just iterate for small counts).

---

## Layer 4: Structural Physics

### Structural Integrity

Blocks have a **support** value based on their connections:
- A wall on the ground: fully supported
- A wall on top of another wall: supported
- A roof: supported if walls underneath
- A floating block (no support below): collapses

When a supporting block is removed:
1. Recompute support for affected blocks
2. Unsupported blocks become **falling debris** particles
3. Falling debris damages blocks below, potentially cascading
4. Dust particles generated (smoke injection)
5. Air displacement from collapse (velocity injection)

### Fire Structural Damage

Wood blocks have a **burn timer**. When adjacent to fire and temperature > 250°C:
- Burn timer decreases
- Block produces smoke, CO2
- When timer reaches 0: block is destroyed → structural recalculation → potential collapse

This creates emergent fire behavior: fire weakens a support wall → upper structure collapses → debris falls → more fire → more collapse.

### Explosion Structural Damage

High-velocity fluid (from explosion) exerts force on blocks:
```
force = velocity_at_block × fluid_density
if force > block.structural_strength:
    block is destroyed or damaged
```

Block strengths:
- Glass: 5 (shatters easily)
- Wood: 20
- Stone: 100 (very hard to break)
- Metal: 200

---

## Layer 5: Light-Physics Interactions

### Volumetric Light Through Gas

Already partially implemented (smoke overlay in raytrace shader). Enhanced version:

**Light beams through smoke**: When a directional light (sun, headlamp, laser) passes through smoky air, the smoke scatters light. This is visible as "god rays" — bright shafts where light intersects dense smoke.

```wgsl
// In shadow ray trace: accumulate smoke density along ray
var fog_density = 0.0;
for each step along ray:
    fog_density += sample_smoke(step_pos) * step_length;
// Light is attenuated by fog, but also scatters
let scattered = sun_color * fog_density * scatter_coefficient;
color += scattered;  // additive light scattering
```

### Hot Gas Glows

At very high temperatures (>500°C), gas itself emits visible light:
```wgsl
let temp = sample_temperature(world_pos);
if temp > 500.0 {
    let glow = (temp - 500.0) / 500.0;  // 0 at 500°C, 1 at 1000°C
    let glow_color = mix(vec3(0.8, 0.2, 0.0), vec3(1.0, 1.0, 0.8), clamp(glow, 0.0, 1.0));
    color += glow_color * glow * 0.3;
}
```

This makes explosion fireballs self-illuminating — they glow orange-white independent of other light sources.

### Shadow Casting from Dynamic Objects

Plebs and particles cast shadows. The shadow ray trace already checks the block grid; extend it to also check pleb positions:

```wgsl
// During shadow ray: check if ray passes near pleb
let pleb_dist_to_ray = point_line_distance(pleb_pos, ray_origin, ray_dir);
if pleb_dist_to_ray < 0.4 {
    shadow *= 0.3;  // pleb partially blocks sunlight
}
```

---

## Layer 6: Weapon Physics (Future)

### Projectile Types

| Weapon | Speed | Damage | Gas Interaction |
|--------|-------|--------|----------------|
| Arrow | 30 tiles/sec | 20 | Slight drag, deflected by strong wind |
| Bullet | 200 tiles/sec | 50 | Minimal drag, traces through gas |
| Laser | instant | 30/sec | Heats gas along beam, ignites flammables |
| Flamethrower | 10 tiles/sec | 15/sec | Injects fire particles, ignites gas |
| Grenade | 15 tiles/sec | 80 (blast) | Creates explosion (pressure + temp + debris) |
| Smoke bomb | 15 tiles/sec | 0 | Injects dense smoke, blocks visibility |

### Laser Beam Physics

A laser is a ray from source to first solid block:
1. Ray traces through grid (like shadow ray)
2. At each cell: heat gas, trigger chemistry, scatter light
3. At hit block: apply damage, heat block, spawn sparks
4. Through glass: partial absorption + tinting (like sunlight through windows)
5. Reflects off metal surfaces (future: mirrors, redirected beams)

### Flamethrower

Injects a spray of hot particles + velocity + temperature into the fluid:
```
For a cone in front of the weapon:
    inject_velocity(pos, forward_dir * power)
    inject_temperature(pos, 800°C * falloff)
    inject_smoke(pos, density * falloff)
    spawn_ember_particles()
```

The fluid sim then carries the flame realistically — it follows airflow, curls around corners, billows against ceilings.

---

## Implementation Phases

### Phase P1: Entity-Fluid Coupling
- [ ] Pleb displaces air (velocity injection at pleb position)
- [ ] Pleb as partial obstacle (soft obstacle at pleb position)
- [ ] Block placement/removal creates pressure wave
- [ ] Pleb body blocks/scatters light (shadow from pleb)

### Phase P2: Thermal Physics
- [ ] Temperature field (dye.a channel) — requires Phase 2d
- [ ] Buoyancy from temperature differentials
- [ ] Block thermal mass and heat conduction
- [ ] Hot gas self-illumination (>500°C glow)
- [ ] Explosion pressure waves

### Phase P3: Particles
- [ ] Particle system (CPU for <100, GPU compute for more)
- [ ] Ember particles from fire (carried by fluid velocity)
- [ ] Debris from structural collapse
- [ ] Particle-fluid two-way coupling
- [ ] Particle collision with grid
- [ ] Particle rendering in raytrace shader

### Phase P4: Structural Physics
- [ ] Block support computation
- [ ] Structural collapse when support removed
- [ ] Fire weakens wood blocks → collapse
- [ ] Explosion damages blocks → collapse
- [ ] Debris particles from collapse

### Phase P5: Weapons
- [ ] Projectile system (arrows, bullets)
- [ ] Laser beams (heat gas, trigger chemistry, volumetric scatter)
- [ ] Flamethrower (particle spray + fluid injection)
- [ ] Grenades/explosions
- [ ] Smoke bombs

### Phase P6: Advanced
- [ ] Volumetric god rays through smoke
- [ ] Sliding doors / pistons (moving obstacles)
- [ ] Mirrors / laser reflection
- [ ] Conveyor belts (moving blocks that push fluid)
- [ ] Water flow physics (MPM hybrid — see SPEC.md)

---

## Integration Points Summary

```
                    ┌──────────────┐
                    │  Block Grid  │
                    └──────┬───────┘
                           │ obstacles, placement events
                    ┌──────┴───────┐
         ┌─────────┤  Fluid Sim   ├──────────┐
         │         └──────┬───────┘          │
         │ velocity,      │ temp,            │ velocity,
         │ drag           │ O2/CO2           │ pressure
    ┌────┴────┐    ┌──────┴───────┐    ┌─────┴─────┐
    │Particles│    │    Plebs     │    │  Weapons  │
    └────┬────┘    └──────┬───────┘    └─────┬─────┘
         │ hot embers     │ needs,           │ damage,
         │ ignition       │ health           │ heat
    ┌────┴────┐    ┌──────┴───────┐    ┌─────┴─────┐
    │Chemistry│    │   AI/Mood    │    │Structural │
    └─────────┘    └──────────────┘    └───────────┘
         │                                    │
         └────── both feed back into ─────────┘
                    Fluid Sim
```

Every system reads from and writes to the fluid sim. The fluid sim is the central nervous system of the world.

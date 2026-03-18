# Spacewestern

A colony survival simulation rendered entirely via GPU compute shader raytracing. Inspired by Rimworld's top-down management gameplay, with a real-time Navier-Stokes fluid simulation at its core.

**Play it**: https://spiffistan.github.io/spacewestern/

## Building

```bash
# Native
cargo run --release

# Web (requires trunk: cargo install trunk)
trunk serve --release
```

## Documentation

| Document | Description |
|----------|-------------|
| [CONTEXT.md](docs/CONTEXT.md) | Project state summary — architecture, data structures, GPU pipeline, current features |
| [SPEC.md](docs/SPEC.md) | Full game specification — vision, systems, fluid mechanics, rendering, AI |
| [PLAN.md](docs/PLAN.md) | Phased development roadmap with deliverables and acceptance criteria |
| [AI_SYSTEM.md](docs/AI_SYSTEM.md) | Pleb AI design — needs, mood, personality, utility AI, mental breaks |
| [PHYSICS_SYSTEM.md](docs/PHYSICS_SYSTEM.md) | Physics integration — entity-fluid coupling, thermal, particles, structural |
| [MASTER_SPEC.md](docs/MASTER_SPEC.md) | Original high-level vision document |

### Design Notes

| Note | Description |
|------|-------------|
| [DN-001](docs/dn/DN-001-blender-sprite-pipeline.md) | Blender-to-sprite asset pipeline for tree/object models |

### References

| Document | Description |
|----------|-------------|
| [Fluid Mechanics Inspiration](docs/fluid_mechanics/INSPIRATON.md) | Reference demos and academic papers for the fluid sim |

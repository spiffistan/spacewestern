# Rayworld

## Rust Coding Standards
Follow the Rust best practices in `.claude/skills/rust-skills/CLAUDE.md` (179 rules).
Key priorities for this project:
- **Ownership**: prefer `&T` over `.clone()`, accept `&[T]` not `&Vec<T>`
- **Error handling**: use `Result` over `panic`, `?` for propagation
- **Performance**: use iterators over indexing, `with_capacity()` for known sizes
- **No unwrap in production**: use `unwrap_or`, `if let`, or `?` instead
- **Naming**: `snake_case` functions, `SCREAMING_SNAKE` constants, `CamelCase` types

## Version & Push Policy
The current version number is in the `VERSION` file (integer). The version label in `src/main.rs` reads it via `include_str!("../VERSION")`.
- **Do NOT push** — only commit. The user will say "push it" explicitly when ready.
- **Bump VERSION only when pushing**, not on every commit.
- When the user says "push it": bump VERSION, amend/commit, then push.
- **Always `cargo build` and `cargo test` before committing.** Shader errors only surface at runtime, so a clean compile is the minimum bar.

## Architecture
Single-crate Rust project with wgpu. Shaders are WGSL files in `src/shaders/`.
Builds native and WASM (via Trunk).

### Module structure
| File | Purpose |
|------|---------|
| `main.rs` | App struct, GfxState, render loop, resize, event loop |
| `types.rs` | Shared types: notifications, events, selection, context menu, sound, blueprint |
| `placement.rs` | Build placement, tile helpers, drag shapes, block interactions |
| `input.rs` | Click handling, keyboard input, notify/log helpers |
| `simulation.rs` | Time, weather, plebs, farming, construction, hauling, combat |
| `ui.rs` | All egui draw functions: menus, overlays, labels, panels |
| `gpu_init.rs` | GPU resource creation, pipelines, bind groups |
| `grid.rs` | BT_* block type constants, block packing, world gen |
| `pleb.rs` | Pleb struct, activities, schedule, A* pathfinding |
| `needs.rs` | Needs system, breathing, environment sampling |
| `physics.rs` | Physics bodies, DDA bullet trace |
| `pipes.rs` | CPU pipe network simulation |
| `zones.rs` | Zones, work tasks, crop growth |

All modules use `impl App` blocks in separate files (same pattern as ui.rs, simulation.rs).

## Block type constants
- **Single source of truth**: `BT_*` constants in `grid.rs` (u32)
- **Rust**: use `BT_*` constants directly; `bt_is!()` macro for multi-value checks
- **WGSL**: `wgsl_block_constants()` generates `const BT_*: u32 = N;` lines, prepended to shader source at load time via `shader_with_constants()` in gpu_init.rs
- **Adding a new block type**: add `BT_*` const in grid.rs + entry in `wgsl_block_constants()` array + block def in blocks.toml
- `block_type_rs()` returns `u32` (not u8) — no casting needed when comparing to BT_*
- `BuildTool::Place` holds `u32` — matches BT_* constants directly
- `BlockRegistry::get/name` accept `u32`
- Narrow to `u8` only at `make_block()` call sites

## Key conventions
- Workgroup size 8x8 for all compute shaders
- CameraUniform struct must match identically across all WGSL files and the Rust struct
- FluidParams struct must match between Rust and all fluid WGSL files
- Ping-pong textures: odd iteration counts so final result is in texture B
- Use named constants for gameplay tuning values — no magic numbers
- Pipe component functions (`is_gas_pipe_component` etc.) take `u32`
- `is_conductor_rs()` takes `u32` for block type

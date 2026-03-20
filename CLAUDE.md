# Rayworld

## Version & Push Policy
The current version number is in the `VERSION` file (integer). The version label in `src/main.rs` reads it via `include_str!("../VERSION")`.
- **Do NOT push** — only commit. The user will say "push it" explicitly when ready.
- **Bump VERSION only when pushing**, not on every commit.
- When the user says "push it": bump VERSION, amend/commit, then push.

## Architecture
Single-crate Rust project with wgpu. All game logic in `src/main.rs`. Shaders are WGSL files in `src/shaders/`.
Builds native and WASM (via Trunk).

## Key conventions
- Workgroup size 8x8 for all compute shaders
- CameraUniform struct must match identically across all WGSL files and the Rust struct
- FluidParams struct must match between Rust and all fluid WGSL files
- Ping-pong textures: odd iteration counts so final result is in texture B

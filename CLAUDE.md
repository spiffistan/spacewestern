# Rayworld

## Version
The current version number is in the `VERSION` file (integer). Always bump it when committing/pushing.
Update both `VERSION` and the version label in `src/main.rs` (search for the `format!("v{} |"` pattern).

## Architecture
Single-crate Rust project with wgpu. All game logic in `src/main.rs`. Shaders are WGSL files in `src/`.
Builds native and WASM (via Trunk).

## Key conventions
- Workgroup size 8x8 for all compute shaders
- CameraUniform struct must match identically across all WGSL files and the Rust struct
- FluidParams struct must match between Rust and all fluid WGSL files
- Ping-pong textures: odd iteration counts so final result is in texture B

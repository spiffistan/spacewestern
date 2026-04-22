# Refactoring TODO

Temporary tracking doc. Delete when done.

## Done

- [x] Merge simulation.rs impl blocks
- [x] Extract CharGenState (12), ManifestState (4), FogState (8), CombatState (13) — App 232→192 fields
- [x] Split ui.rs 13K→5.7K: ui_world.rs, ui_build.rs, ui_screens.rs, world_setup.rs
- [x] Fix unsafe unwraps (total_cmp, unwrap_or, if let)
- [x] Pathfinding helper send_pleb_to_target() (~110 lines removed)
- [x] Day/night thresholds → named constants
- [x] Fix charcoal icon typo

## Remaining

- [ ] Replace `use crate::*` in ui.rs, simulation.rs, placement.rs, input.rs
- [ ] More magic numbers — wind params, ambient colors, lightning timing
- [ ] Build menu gaps — 8 blocks with placement defs not in menu
- [ ] Test coverage for simulation.rs, placement.rs, input.rs

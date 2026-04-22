# Refactoring TODO

Temporary tracking doc. Delete when done.

## Critical (structural)

- [x] **Merge simulation.rs impl blocks** — done
- [x] **Extract App sub-structs** — CharGenState (12 fields), ManifestState (4 fields)
- [x] **Split ui.rs** — 13K→5.7K: ui_world.rs (4K), ui_build.rs (1.9K), ui_screens.rs (1.3K), world_setup.rs (209)

## High (crash risks + boilerplate)

- [x] **Fix unsafe unwraps** — partial_cmp→total_cmp, min/max→unwrap_or, harvest_target→if let
- [x] **Fix items.toml** — Charcoal icon typo
- [x] **Pathfinding helper** — send_pleb_to_target() deduplicates ~110 lines

## Medium (quality)

- [x] **Magic numbers** — day/night thresholds extracted to named constants

## Remaining

- [ ] Extract FogState (8 fields), CombatState (13 fields) from App
- [ ] Replace `use crate::*` in ui.rs, simulation.rs, placement.rs, input.rs
- [ ] More magic numbers — wind params, ambient colors, lightning timing
- [ ] Build menu gaps — 8 blocks with placement defs not in menu
- [ ] Test coverage for simulation.rs, placement.rs, input.rs

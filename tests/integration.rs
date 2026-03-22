//! Cross-module integration tests for Rayworld.

use rayworld::grid::*;
use rayworld::pipes::*;
use rayworld::resources::*;
use rayworld::materials::NUM_MATERIALS;

// --- Block system ---

#[test]
fn all_block_ids_fit_in_materials() {
    let highest = BT_LIQUID_OUTPUT; // 54
    assert!(NUM_MATERIALS > highest as usize,
        "NUM_MATERIALS ({}) must be > highest block ID ({})", NUM_MATERIALS, highest);
}

#[test]
fn ground_block_classification() {
    assert!(is_ground_block(BT_AIR));
    assert!(is_ground_block(BT_DIRT));
    assert!(is_ground_block(BT_WATER));
    assert!(is_ground_block(BT_WOOD_FLOOR));
    assert!(is_ground_block(BT_DUG_GROUND));
    assert!(!is_ground_block(BT_STONE));
    assert!(!is_ground_block(BT_PIPE));
    assert!(!is_ground_block(BT_WIRE));
}

#[test]
fn wall_block_classification() {
    assert!(is_wall_block(BT_STONE));
    assert!(is_wall_block(BT_WOOD_WALL));
    assert!(is_wall_block(BT_STEEL_WALL));
    assert!(is_wall_block(BT_GLASS));
    assert!(is_wall_block(BT_DIAGONAL));
    assert!(!is_wall_block(BT_DIRT));
    assert!(!is_wall_block(BT_PIPE));
    assert!(!is_wall_block(BT_WIRE));
}

#[test]
fn wire_block_classification() {
    assert!(is_wire_block(BT_WIRE));
    assert!(is_wire_block(BT_DIMMER));
    assert!(is_wire_block(BT_SWITCH));
    assert!(is_wire_block(BT_BREAKER));
    assert!(is_wire_block(BT_WIRE_BRIDGE));
    assert!(!is_wire_block(BT_PIPE));
    assert!(!is_wire_block(BT_DIRT));
}

#[test]
fn conductor_includes_all_power_blocks() {
    let power_ids: &[u8] = &[36, 37, 38, 39, 40, 41, 42, 43, 45, 48, 51, 7, 10, 11, 12, 16];
    for &id in power_ids {
        assert!(is_conductor_rs(id, 0), "Block type {} should be a conductor", id);
    }
    assert!(is_conductor_rs(1, 0x80), "Wall with wire overlay should be conductor");
    assert!(!is_conductor_rs(2, 0), "Dirt should not be conductor");
    assert!(!is_conductor_rs(15, 0), "Pipe should not be conductor");
}

// --- Pipe networks ---

fn test_grid(blocks: &[((u32, u32), u8, u8)]) -> Vec<u32> {
    let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
    for &((x, y), bt, h) in blocks {
        grid[(y * GRID_W + x) as usize] = make_block(bt, h, 0);
    }
    grid
}

#[test]
fn liquid_network_isolation_from_gas() {
    let grid = test_grid(&[
        ((10, 10), 15, 1),  // gas pipe
        ((11, 10), 49, 1),  // liquid pipe
    ]);
    let mut gas_net = PipeNetwork::new();
    gas_net.rebuild(&grid);
    let mut liq_net = PipeNetwork::new();
    liq_net.rebuild_with(&grid, is_liquid_pipe_component);

    assert_eq!(gas_net.cells.len(), 1);
    assert_eq!(liq_net.cells.len(), 1);
    assert!(gas_net.cells.contains_key(&(10 * GRID_W + 10)));
    assert!(!gas_net.cells.contains_key(&(10 * GRID_W + 11)));
    assert!(liq_net.cells.contains_key(&(10 * GRID_W + 11)));
}

#[test]
fn liquid_pump_builds_pressure() {
    let grid = test_grid(&[
        ((10, 10), 49, 1), ((11, 10), 53, 1), ((12, 10), 49, 1),
    ]);
    let mut net = PipeNetwork::new();
    net.rebuild_with(&grid, is_liquid_pipe_component);
    for _ in 0..60 { net.tick(1.0 / 60.0, &grid, 5.0); }
    let pump_p = net.cells[&(10 * GRID_W + 11)].pressure;
    assert!(pump_p > 1.0, "Pump should have built pressure, got {}", pump_p);
}

#[test]
fn liquid_output_creates_injections() {
    let grid = test_grid(&[
        ((10, 10), 53, 1), ((11, 10), 49, 1), ((12, 10), 54, 1),
    ]);
    let mut net = PipeNetwork::new();
    net.rebuild_with(&grid, is_liquid_pipe_component);
    let mut total = 0;
    for _ in 0..120 { total += net.tick(1.0 / 60.0, &grid, 5.0).len(); }
    assert!(total > 0, "Output should have produced injections");
}

#[test]
fn intake_without_pump_no_pressure() {
    let mut grid = test_grid(&[
        ((10, 10), 52, 1), ((11, 10), 52, 1), ((12, 10), 49, 1),
    ]);
    grid[(10 * GRID_W + 11) as usize] = make_block(52, 1, 1 << 3);
    let mut net = PipeNetwork::new();
    net.rebuild_with(&grid, is_liquid_pipe_component);
    for _ in 0..60 { net.tick(1.0 / 60.0, &grid, 5.0); }
    let p = net.cells[&(10 * GRID_W + 12)].pressure;
    assert!(p < 0.01, "Pipe should have no pressure without pump, got {:.4}", p);
}

// --- Resources ---

#[test]
fn item_kind_labels() {
    assert_eq!(ItemKind::Berries(5).label(), "5 berries");
    assert_eq!(ItemKind::Wood(10).label(), "10 wood");
    assert_eq!(ItemKind::Rocks(3).label(), "3 rocks");
}

#[test]
fn item_kind_counts() {
    assert_eq!(ItemKind::Berries(5).count(), 5);
    assert_eq!(ItemKind::Wood(10).count(), 10);
    assert_eq!(ItemKind::Rocks(3).count(), 3);
}

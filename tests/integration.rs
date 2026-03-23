//! Cross-module integration tests for Rayworld.

use rayworld::grid::*;
use rayworld::pipes::*;
use rayworld::resources::*;
use rayworld::item_defs::*;
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
    let power_ids: &[u32] = &[36, 37, 38, 39, 40, 41, 42, 43, 45, 48, 51, 7, 10, 11, 12, 16];
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
fn item_stack_labels() {
    let stack = ItemStack::new(ITEM_BERRIES, 5);
    assert!(stack.label().contains("5"));
    assert!(stack.label().contains("Berries"));
    let wood = ItemStack::new(ITEM_WOOD, 10);
    assert!(wood.label().contains("10"));
}

#[test]
fn item_stack_counts() {
    let stack = ItemStack::new(ITEM_BERRIES, 5);
    assert_eq!(stack.count, 5);
    let inv = CrateInventory::default();
    assert_eq!(inv.total(), 0);
}

// --- Fluid obstacle field ---

#[test]
fn obstacle_field_walls_are_solid() {
    use rayworld::fluid::build_obstacle_field;
    let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize]; // all dirt
    // Place a 4x4 room at (10,10)-(13,13) with stone walls (height 3)
    for x in 10..14 { grid[(10 * GRID_W + x) as usize] = make_block(1, 3, 0); } // top
    for x in 10..14 { grid[(13 * GRID_W + x) as usize] = make_block(1, 3, 0); } // bottom
    for y in 10..14 { grid[(y * GRID_W + 10) as usize] = make_block(1, 3, 0); } // left
    for y in 10..14 { grid[(y * GRID_W + 13) as usize] = make_block(1, 3, 0); } // right

    let obs = build_obstacle_field(&grid);
    // Walls should be solid (255)
    assert_eq!(obs[(10 * GRID_W + 10) as usize], 255, "top-left wall should be solid");
    assert_eq!(obs[(10 * GRID_W + 12) as usize], 255, "top wall should be solid");
    assert_eq!(obs[(12 * GRID_W + 10) as usize], 255, "left wall should be solid");
    // Interior should be open (0)
    assert_eq!(obs[(11 * GRID_W + 11) as usize], 0, "interior should be open");
    assert_eq!(obs[(12 * GRID_W + 12) as usize], 0, "interior should be open");
    // Exterior should be open (0)
    assert_eq!(obs[(5 * GRID_W + 5) as usize], 0, "exterior should be open");
}

#[test]
fn obstacle_field_doors_open_are_passable() {
    use rayworld::fluid::build_obstacle_field;
    let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
    // Stone wall with door flag (bit 0) + open flag (bit 2) = flags 5
    grid[(10 * GRID_W + 10) as usize] = make_block(1, 3, 5); // open door
    grid[(10 * GRID_W + 11) as usize] = make_block(1, 3, 1); // closed door
    let obs = build_obstacle_field(&grid);
    assert_eq!(obs[(10 * GRID_W + 10) as usize], 0, "open door should be passable");
    assert_eq!(obs[(10 * GRID_W + 11) as usize], 255, "closed door should be solid");
}

#[test]
fn obstacle_field_plants_are_passable() {
    use rayworld::fluid::build_obstacle_field;
    let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
    grid[(10 * GRID_W + 10) as usize] = make_block(BT_TREE as u8, 3, 0);
    grid[(10 * GRID_W + 11) as usize] = make_block(BT_BERRY_BUSH as u8, 1, 0);
    grid[(10 * GRID_W + 12) as usize] = make_block(BT_CROP as u8, 2, 0);
    let obs = build_obstacle_field(&grid);
    assert_eq!(obs[(10 * GRID_W + 10) as usize], 0, "tree should be passable");
    assert_eq!(obs[(10 * GRID_W + 11) as usize], 0, "berry bush should be passable");
    assert_eq!(obs[(10 * GRID_W + 12) as usize], 0, "crop should be passable");
}

#[test]
fn obstacle_field_pipes_wires_passable() {
    use rayworld::fluid::build_obstacle_field;
    let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
    grid[(10 * GRID_W + 10) as usize] = make_block(BT_PIPE as u8, 1, 0);
    grid[(10 * GRID_W + 11) as usize] = make_block(BT_WIRE as u8, 0xA0, 0); // wire with conn mask
    grid[(10 * GRID_W + 12) as usize] = make_block(BT_LIQUID_PIPE as u8, 1, 0);
    let obs = build_obstacle_field(&grid);
    assert_eq!(obs[(10 * GRID_W + 10) as usize], 0, "gas pipe should be passable");
    assert_eq!(obs[(10 * GRID_W + 11) as usize], 0, "wire should be passable");
    assert_eq!(obs[(10 * GRID_W + 12) as usize], 0, "liquid pipe should be passable");
}

#[test]
fn obstacle_field_complete_room_sealed() {
    // Verify a fully-sealed room has NO passable gaps in the wall ring
    use rayworld::fluid::build_obstacle_field;
    let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
    // 6x6 room at (20,20)-(25,25)
    for x in 20..26 {
        grid[(20 * GRID_W + x) as usize] = make_block(1, 3, 0);
        grid[(25 * GRID_W + x) as usize] = make_block(1, 3, 0);
    }
    for y in 20..26 {
        grid[(y * GRID_W + 20) as usize] = make_block(1, 3, 0);
        grid[(y * GRID_W + 25) as usize] = make_block(1, 3, 0);
    }
    let obs = build_obstacle_field(&grid);
    // Check every wall tile is solid
    for x in 20..26 {
        assert_eq!(obs[(20 * GRID_W + x) as usize], 255, "top wall at x={} should be solid", x);
        assert_eq!(obs[(25 * GRID_W + x) as usize], 255, "bottom wall at x={}", x);
    }
    for y in 20..26 {
        assert_eq!(obs[(y * GRID_W + 20) as usize], 255, "left wall at y={}", y);
        assert_eq!(obs[(y * GRID_W + 25) as usize], 255, "right wall at y={}", y);
    }
    // Interior all open
    for y in 21..25 {
        for x in 21..25 {
            assert_eq!(obs[(y * GRID_W + x) as usize], 0, "interior ({},{}) should be open", x, y);
        }
    }
}

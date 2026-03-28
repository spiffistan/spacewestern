//! Cross-module integration tests for Rayworld.

use rayworld::grid::*;
use rayworld::item_defs::*;
use rayworld::materials::NUM_MATERIALS;
use rayworld::pipes::*;
use rayworld::resources::*;

// --- Block system ---

#[test]
fn all_block_ids_fit_in_materials() {
    let highest = BT_LIQUID_OUTPUT; // 54
    assert!(
        NUM_MATERIALS > highest as usize,
        "NUM_MATERIALS ({}) must be > highest block ID ({})",
        NUM_MATERIALS,
        highest
    );
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
    let power_ids: &[u32] = &[
        36, 37, 38, 39, 40, 41, 42, 43, 45, 48, 51, 7, 10, 11, 12, 16,
    ];
    for &id in power_ids {
        assert!(
            is_conductor_rs(id, 0),
            "Block type {} should be a conductor",
            id
        );
    }
    assert!(
        is_conductor_rs(1, 0x80),
        "Wall with wire overlay should be conductor"
    );
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
        ((10, 10), 15, 1), // gas pipe
        ((11, 10), 49, 1), // liquid pipe
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
    let grid = test_grid(&[((10, 10), 49, 1), ((11, 10), 53, 1), ((12, 10), 49, 1)]);
    let mut net = PipeNetwork::new();
    net.rebuild_with(&grid, is_liquid_pipe_component);
    for _ in 0..60 {
        net.tick(1.0 / 60.0, &grid, 5.0);
    }
    let pump_p = net.cells[&(10 * GRID_W + 11)].pressure;
    assert!(
        pump_p > 1.0,
        "Pump should have built pressure, got {}",
        pump_p
    );
}

#[test]
fn liquid_output_creates_injections() {
    let grid = test_grid(&[((10, 10), 53, 1), ((11, 10), 49, 1), ((12, 10), 54, 1)]);
    let mut net = PipeNetwork::new();
    net.rebuild_with(&grid, is_liquid_pipe_component);
    let mut total = 0;
    for _ in 0..120 {
        total += net.tick(1.0 / 60.0, &grid, 5.0).len();
    }
    assert!(total > 0, "Output should have produced injections");
}

#[test]
fn intake_without_pump_no_pressure() {
    let mut grid = test_grid(&[((10, 10), 52, 1), ((11, 10), 52, 1), ((12, 10), 49, 1)]);
    grid[(10 * GRID_W + 11) as usize] = make_block(52, 1, 1 << 3);
    let mut net = PipeNetwork::new();
    net.rebuild_with(&grid, is_liquid_pipe_component);
    for _ in 0..60 {
        net.tick(1.0 / 60.0, &grid, 5.0);
    }
    let p = net.cells[&(10 * GRID_W + 12)].pressure;
    assert!(
        p < 0.01,
        "Pipe should have no pressure without pump, got {:.4}",
        p
    );
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

// --- Crafting system ---

#[test]
fn fiber_and_scrap_wood_are_valid_items() {
    let reg = rayworld::item_defs::ItemRegistry::cached();
    let fiber = reg.get(ITEM_FIBER);
    assert!(
        fiber.is_some(),
        "ITEM_FIBER (id={}) should exist in registry",
        ITEM_FIBER
    );
    assert_eq!(fiber.unwrap().name, "Fiber");

    let scrap = reg.get(ITEM_SCRAP_WOOD);
    assert!(
        scrap.is_some(),
        "ITEM_SCRAP_WOOD (id={}) should exist in registry",
        ITEM_SCRAP_WOOD
    );
    assert_eq!(scrap.unwrap().name, "Sticks");
}

#[test]
fn ground_items_with_new_types() {
    let fiber_item = GroundItem::new(5.0, 5.0, ITEM_FIBER, 3);
    assert_eq!(fiber_item.stack.item_id, ITEM_FIBER);
    assert_eq!(fiber_item.stack.count, 3);
    assert!(fiber_item.stack.label().contains("Fiber"));

    let scrap_item = GroundItem::new(5.0, 5.0, ITEM_SCRAP_WOOD, 4);
    assert_eq!(scrap_item.stack.item_id, ITEM_SCRAP_WOOD);
    assert!(scrap_item.stack.label().contains("Sticks"));
}

#[test]
fn crate_stores_any_item_type() {
    let mut crate_inv = CrateInventory::default();
    crate_inv.add(ITEM_FIBER, 5);
    crate_inv.add(ITEM_SCRAP_WOOD, 3);
    assert_eq!(crate_inv.count_of(ITEM_FIBER), 5);
    assert_eq!(crate_inv.count_of(ITEM_SCRAP_WOOD), 3);
    assert_eq!(crate_inv.total(), 8);
}

#[test]
fn recipe_can_craft_with_fiber() {
    let reg = rayworld::recipe_defs::RecipeRegistry::load();
    let rope = reg
        .for_station("workbench")
        .into_iter()
        .find(|r| r.name == "Rope")
        .expect("Rope recipe should exist");
    // Rope needs 4 fiber
    assert_eq!(rope.inputs[0].item, ITEM_FIBER);
    assert_eq!(rope.inputs[0].count, 4);

    // Not enough
    let inv = vec![ItemStack::new(ITEM_FIBER, 3)];
    assert!(!rayworld::recipe_defs::RecipeRegistry::can_craft(
        rope, &inv
    ));

    // Enough
    let inv = vec![ItemStack::new(ITEM_FIBER, 4)];
    assert!(rayworld::recipe_defs::RecipeRegistry::can_craft(rope, &inv));
}

#[test]
fn recipe_can_craft_with_scrap_wood() {
    let reg = rayworld::recipe_defs::RecipeRegistry::load();
    let bucket = reg
        .for_station("workbench")
        .into_iter()
        .find(|r| r.name == "Wooden Bucket")
        .expect("Wooden Bucket recipe should exist");
    // Bucket needs 3 scrap wood
    assert_eq!(bucket.inputs[0].item, ITEM_SCRAP_WOOD);
    assert_eq!(bucket.inputs[0].count, 3);

    let inv = vec![ItemStack::new(ITEM_SCRAP_WOOD, 3)];
    assert!(rayworld::recipe_defs::RecipeRegistry::can_craft(
        bucket, &inv
    ));
}

#[test]
fn pleb_inventory_handles_all_item_types() {
    let mut inv = rayworld::resources::PlebInventory::default();
    inv.add(ITEM_FIBER, 5);
    inv.add(ITEM_SCRAP_WOOD, 3);
    inv.add(ITEM_BERRIES, 2);
    assert_eq!(inv.count_of(ITEM_FIBER), 5);
    assert_eq!(inv.count_of(ITEM_SCRAP_WOOD), 3);
    assert_eq!(inv.count_of(ITEM_BERRIES), 2);
    assert!(inv.is_carrying());

    // Remove some
    inv.remove(ITEM_FIBER, 3);
    assert_eq!(inv.count_of(ITEM_FIBER), 2);
}

#[test]
fn resource_counting_all_sources() {
    // Simulate counting resources across ground items, crates, and pleb inventories
    let ground_items = vec![
        GroundItem::new(1.0, 1.0, ITEM_FIBER, 3),
        GroundItem::new(2.0, 2.0, ITEM_SCRAP_WOOD, 4),
        GroundItem::new(3.0, 3.0, ITEM_WOOD, 10),
    ];

    let mut crate_inv = CrateInventory::default();
    crate_inv.add(ITEM_FIBER, 2);

    // Total fiber should be 3 (ground) + 2 (crate) = 5
    let total_fiber: u32 = ground_items
        .iter()
        .filter(|gi| gi.stack.item_id == ITEM_FIBER)
        .map(|gi| gi.stack.count as u32)
        .sum::<u32>()
        + crate_inv.count_of(ITEM_FIBER);
    assert_eq!(total_fiber, 5);

    // Total scrap wood: 4 (ground only)
    let total_scrap: u32 = ground_items
        .iter()
        .filter(|gi| gi.stack.item_id == ITEM_SCRAP_WOOD)
        .map(|gi| gi.stack.count as u32)
        .sum();
    assert_eq!(total_scrap, 4);
}

// --- Fluid obstacle field ---

#[test]
fn obstacle_field_walls_are_solid() {
    use rayworld::fluid::build_obstacle_field;
    let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize]; // all dirt
    // Place a 4x4 room at (10,10)-(13,13) with stone walls (height 3)
    for x in 10..14 {
        grid[(10 * GRID_W + x) as usize] = make_block(1, 3, 0);
    } // top
    for x in 10..14 {
        grid[(13 * GRID_W + x) as usize] = make_block(1, 3, 0);
    } // bottom
    for y in 10..14 {
        grid[(y * GRID_W + 10) as usize] = make_block(1, 3, 0);
    } // left
    for y in 10..14 {
        grid[(y * GRID_W + 13) as usize] = make_block(1, 3, 0);
    } // right

    let obs = build_obstacle_field(&grid, &[]);
    // Walls should be solid (255)
    assert_eq!(
        obs[(10 * GRID_W + 10) as usize],
        255,
        "top-left wall should be solid"
    );
    assert_eq!(
        obs[(10 * GRID_W + 12) as usize],
        255,
        "top wall should be solid"
    );
    assert_eq!(
        obs[(12 * GRID_W + 10) as usize],
        255,
        "left wall should be solid"
    );
    // Interior should be open (0)
    assert_eq!(
        obs[(11 * GRID_W + 11) as usize],
        0,
        "interior should be open"
    );
    assert_eq!(
        obs[(12 * GRID_W + 12) as usize],
        0,
        "interior should be open"
    );
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
    let obs = build_obstacle_field(&grid, &[]);
    assert_eq!(
        obs[(10 * GRID_W + 10) as usize],
        0,
        "open door should be passable"
    );
    assert_eq!(
        obs[(10 * GRID_W + 11) as usize],
        255,
        "closed door should be solid"
    );
}

#[test]
fn obstacle_field_plants_are_passable() {
    use rayworld::fluid::build_obstacle_field;
    let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
    grid[(10 * GRID_W + 10) as usize] = make_block(BT_TREE as u8, 3, 0);
    grid[(10 * GRID_W + 11) as usize] = make_block(BT_BERRY_BUSH as u8, 1, 0);
    grid[(10 * GRID_W + 12) as usize] = make_block(BT_CROP as u8, 2, 0);
    let obs = build_obstacle_field(&grid, &[]);
    assert_eq!(
        obs[(10 * GRID_W + 10) as usize],
        0,
        "tree should be passable"
    );
    assert_eq!(
        obs[(10 * GRID_W + 11) as usize],
        0,
        "berry bush should be passable"
    );
    assert_eq!(
        obs[(10 * GRID_W + 12) as usize],
        0,
        "crop should be passable"
    );
}

#[test]
fn obstacle_field_pipes_wires_passable() {
    use rayworld::fluid::build_obstacle_field;
    let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
    grid[(10 * GRID_W + 10) as usize] = make_block(BT_PIPE as u8, 1, 0);
    grid[(10 * GRID_W + 11) as usize] = make_block(BT_WIRE as u8, 0xA0, 0); // wire with conn mask
    grid[(10 * GRID_W + 12) as usize] = make_block(BT_LIQUID_PIPE as u8, 1, 0);
    let obs = build_obstacle_field(&grid, &[]);
    assert_eq!(
        obs[(10 * GRID_W + 10) as usize],
        0,
        "gas pipe should be passable"
    );
    assert_eq!(
        obs[(10 * GRID_W + 11) as usize],
        0,
        "wire should be passable"
    );
    assert_eq!(
        obs[(10 * GRID_W + 12) as usize],
        0,
        "liquid pipe should be passable"
    );
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
    let obs = build_obstacle_field(&grid, &[]);
    // Check every wall tile is solid
    for x in 20..26 {
        assert_eq!(
            obs[(20 * GRID_W + x) as usize],
            255,
            "top wall at x={} should be solid",
            x
        );
        assert_eq!(
            obs[(25 * GRID_W + x) as usize],
            255,
            "bottom wall at x={}",
            x
        );
    }
    for y in 20..26 {
        assert_eq!(obs[(y * GRID_W + 20) as usize], 255, "left wall at y={}", y);
        assert_eq!(
            obs[(y * GRID_W + 25) as usize],
            255,
            "right wall at y={}",
            y
        );
    }
    // Interior all open
    for y in 21..25 {
        for x in 21..25 {
            assert_eq!(
                obs[(y * GRID_W + x) as usize],
                0,
                "interior ({},{}) should be open",
                x,
                y
            );
        }
    }
}

// --- Thin wall edge blocking ---

#[test]
fn thin_wall_north_edge_blocks_northward() {
    let mut grid = vec![make_block(BT_DIRT as u8, 0, 0); (GRID_W * GRID_H) as usize];
    // Place a thin wall on tile (5, 5) with north edge, thickness 1
    let (flags, edge_mask) = make_thin_wall_flags(0, 0, 1); // edge=N, thickness=1
    grid[(5 * GRID_W + 5) as usize] =
        make_block(BT_STONE as u8, make_wall_height(3, edge_mask), flags);

    // Moving north from (5,5) to (5,4): blocked (wall on north edge)
    assert!(edge_blocked(&grid, 5, 5, 5, 4));
    // Moving south from (5,5) to (5,6): not blocked (no wall on south edge)
    assert!(!edge_blocked(&grid, 5, 5, 5, 6));
    // Moving east from (5,5): not blocked
    assert!(!edge_blocked(&grid, 5, 5, 6, 5));
}

#[test]
fn thin_wall_corner_blocks_two_edges() {
    let mut grid = vec![make_block(BT_DIRT as u8, 0, 0); (GRID_W * GRID_H) as usize];
    // Place NE corner (N+E edges, thickness=2)
    let (flags, edge_mask) = make_thin_wall_corner_flags(0, 0, 2); // edge=N, thickness=2, corner
    grid[(5 * GRID_W + 5) as usize] =
        make_block(BT_STONE as u8, make_wall_height(3, edge_mask), flags);

    // North: blocked
    assert!(edge_blocked(&grid, 5, 5, 5, 4));
    // East: blocked (corner covers next clockwise = E)
    assert!(edge_blocked(&grid, 5, 5, 6, 5));
    // South: not blocked
    assert!(!edge_blocked(&grid, 5, 5, 5, 6));
    // West: not blocked
    assert!(!edge_blocked(&grid, 5, 5, 4, 5));
}

#[test]
fn full_wall_blocks_all_edges() {
    let mut grid = vec![make_block(BT_DIRT as u8, 0, 0); (GRID_W * GRID_H) as usize];
    // Full wall (thickness 4, flags bits 5-6 = 0 = full)
    grid[(5 * GRID_W + 5) as usize] = make_block(BT_STONE as u8, 3, 0);

    assert!(edge_blocked(&grid, 5, 5, 5, 4));
    assert!(edge_blocked(&grid, 5, 5, 6, 5));
    assert!(edge_blocked(&grid, 5, 5, 5, 6));
    assert!(edge_blocked(&grid, 5, 5, 4, 5));
}

#[test]
fn thin_wall_is_walkable_check() {
    // Full wall: not walkable
    let full = make_block(BT_STONE as u8, 3, 0);
    assert!(!thin_wall_is_walkable(full));

    // Thin wall: walkable (has open sub-cells)
    let (tw_flags, tw_mask) = make_thin_wall_flags(0, 0, 1);
    let thin = make_block(BT_STONE as u8, make_wall_height(3, tw_mask), tw_flags);
    assert!(thin_wall_is_walkable(thin));
}

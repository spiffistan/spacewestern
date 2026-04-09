# DN-028: Hidden Conduits — Infrastructure Inside Walls and Underground

**Status:** Draft
**Depends on:** DN-004 (thin walls), DN-008 (three-layer world model)
**Related:** pipe network (pipes.rs), power grid (gpu_init.rs power shader), fluid sim

## Problem

Pipes and wires currently sit on the floor as visible block types. This creates ugly industrial spaghetti in finished rooms. Real buildings hide infrastructure inside walls — electrical runs through studs, plumbing through cavities, sometimes HVAC too.

Players should be able to embed wires, gas pipes, and liquid pipes inside walls. Thicker walls accommodate more. The room stays clean. The overlay modes reveal what's hidden.

## Core Rules

### One of each per tile, maximum

A single tile can contain at most:
- One wire run
- One gas pipe
- One liquid pipe

No doubling. You can't run two wires through the same wall tile. If you need parallel runs, use adjacent wall tiles or route one through the floor.

This keeps the system simple and the conduit data compact.

### Wall material AND thickness gate capacity

Capacity depends on both wall type (does it have internal cavities?) and thickness (how much room in those cavities?). Not all wall types support conduits.

**By wall material:**

| Wall type | Internal structure | Conduit support |
|-----------|-------------------|-----------------|
| Wattle & Daub | Solid mud over sticks | None — solid fill, too fragile to carve |
| Low Wattle Fence | Open stick frame | None — not enclosed |
| Wood Wall | Plank frame with cavity | Best conduit wall — natural hollow between studs |
| Stone Wall | Solid masonry | Limited — can chisel channels for wire and liquid, but too rigid to seal gas pipe joints |
| Insulated Wall | Double-layer with fill | Excellent — designed with internal cavity |
| Glass Wall | Solid pane | None — can't penetrate glass |

**By thickness (for walls that support conduits):**

| Thickness | Wire | Gas pipe | Liquid pipe |
|-----------|------|----------|-------------|
| 1 (thin) | Wood/Insulated only | No | No |
| 2 (standard) | Wood/Insulated | No | Wood/Insulated |
| 3 (thick) | Wood/Insulated/Stone | Wood/Insulated (not stone) | All supporting types |
| 4 (full block) | All supporting types | Wood/Insulated | All supporting types |

**Combined capacity matrix:**

| | Wire | Liquid pipe | Gas pipe |
|-|------|------------|----------|
| **Wattle & Daub** (any thickness) | No | No | No |
| **Wood 1** | Yes | No | No |
| **Wood 2** | Yes | Yes | No |
| **Wood 3-4** | Yes | Yes | Yes |
| **Stone 1-2** | No | No | No |
| **Stone 3-4** | Yes | Yes | No |
| **Insulated 1** | Yes | No | No |
| **Insulated 2** | Yes | Yes | No |
| **Insulated 3-4** | Yes | Yes | Yes |
| **Glass** (any) | No | No | No |
| **Low Fence** (any) | No | No | No |

Key trade-offs:
- **Wattle** is the cheapest wall but carries nothing. Early game = visible floor runs.
- **Wood** is the infrastructure wall. Natural cavities fit everything at thickness 3+.
- **Stone** is the structural wall. Hard to channel — no gas, wire/liquid only at thickness 3+.
- **Insulated** is the premium wall. Best of both — insulation AND full conduit capacity.
- **Glass** is purely visual — no infrastructure possible.

This creates a real upgrade path: wattle (nothing hidden) → wood (infrastructure) → insulated (infrastructure + thermal). Stone is a parallel choice: strong but infrastructure-limited.

### Conduits follow wall edges

A conduit placed in a wall tile runs along the wall's edge(s). If the wall has a north edge, the wire runs east-west along the north side of the tile. If the wall has both north and east edges (corner), the conduit turns the corner.

This means the conduit's direction is determined by the wall topology, not by a separate placement step. You place "wire in this wall tile" and the system figures out which direction it runs based on the wall edges.

For T-junctions and crosses: the conduit branches to match the wall. A T-junction wall tile creates a T-junction in the conduit.

### No conduits without walls

You can't place a conduit in a tile that has no wall. Conduits are IN walls, not floating. If you remove the wall, the conduits drop as items (or are destroyed with a warning).

## Outlets and Connections

### The outlet problem

A wire running inside a wall is useless if nothing can connect to it. You need **wall outlets** — points where the in-wall wire connects to the room's interior.

Similarly: a gas pipe in a wall needs an **outlet/inlet** to deliver gas into the room. A liquid pipe needs a **tap/spigot**.

### Wall outlet block

A new block type (or wall feature, like doors/windows): **Wall Outlet**.

Placed on a wall tile that contains a wire conduit. The outlet appears on the room-facing side of the wall — a small box that floor-standing devices can connect to.

Devices within 2 tiles of a wall outlet receive power without needing a visible wire run from device to wall. The outlet acts as a wireless-range power tap.

This is important: without outlets, hiding wires in walls is pointless because you'd still need visible wire from the wall to each lamp/device.

### Gas/liquid wall taps

Same concept: a **wall vent** (gas) or **wall tap** (liquid) placed on a wall tile with the appropriate conduit. Gas vents into the room (for ventilation). Liquid taps provide water access (for a sink, future).

These are essentially the existing BT_OUTLET/BT_INLET/BT_LIQUID_OUTPUT but as wall features instead of floor blocks.

### Outlet placement

Outlets are placed like doors/windows — you click a wall tile and the outlet appears on the room-facing side. The wall must contain the appropriate conduit type.

| Feature | Requires | Function |
|---------|----------|----------|
| Wall Outlet | Wire conduit in wall | Powers devices within 2 tiles |
| Wall Vent | Gas conduit in wall | Injects/extracts air (like BT_OUTLET) |
| Wall Tap | Liquid conduit in wall | Provides water access |

## Data Structure

### Conduit data layer

A new `Vec<u16>` parallel to `wall_data`, one entry per grid tile:

```rust
conduit_data: Vec<u16>  // (GRID_W * GRID_H) entries

// Bit layout:
// bits 0-1: wire state
//   00 = no wire
//   01 = wire present
//   10 = wire + outlet (room-facing)
//   11 = reserved
// bits 2-3: gas pipe state
//   00 = no gas pipe
//   01 = gas pipe present
//   10 = gas pipe + vent
//   11 = reserved
// bits 4-5: liquid pipe state
//   00 = no liquid pipe
//   01 = liquid pipe present
//   10 = liquid pipe + tap
//   11 = reserved
// bits 6-7: reserved for future (e.g., data cable, heating duct)
// bits 8-15: unused
```

6 bits for the core system. Extremely compact. The outlet/vent/tap state is encoded alongside the conduit presence, so no additional data needed.

### Reading conduit data

```rust
fn has_wire(cd: u16) -> bool { (cd & 0x3) != 0 }
fn has_gas_pipe(cd: u16) -> bool { (cd & 0xC) != 0 }
fn has_liquid_pipe(cd: u16) -> bool { (cd & 0x30) != 0 }
fn has_wire_outlet(cd: u16) -> bool { (cd & 0x3) == 2 }
fn has_gas_vent(cd: u16) -> bool { (cd & 0xC) == 8 }
fn has_liquid_tap(cd: u16) -> bool { (cd & 0x30) == 0x20 }
```

## Simulation Integration

### Power grid (wire conduits)

The power shader currently reads BT_WIRE blocks from the grid. With conduits:
1. Upload conduit_data as a storage buffer (binding N)
2. In the power shader, check BOTH grid[idx] for BT_WIRE AND conduit_data[idx] for wire bit
3. A tile conducts power if it has BT_WIRE OR has a wire conduit
4. Wall outlets: tiles adjacent to a wall with `has_wire_outlet` also receive power (2-tile wireless range could be handled in the shader or CPU)

### Pipe network (gas/liquid conduits)

The CPU pipe network (pipes.rs) builds a cell graph from BT_PIPE blocks. With conduits:
1. When building the pipe cell graph, also scan conduit_data for gas pipe bits
2. A wall tile with gas conduit bit set becomes a pipe cell, connected to adjacent pipe cells
3. Connection directions follow the wall edge topology (same as how wall_data edges work)
4. Gas vents act like BT_OUTLET: they inject/extract gas to/from the adjacent room tile

### Liquid network

Same pattern as gas. Liquid conduit bits create liquid pipe cells in the liquid network graph.

## Placement UX

The UI must clearly communicate what each wall can and can't carry. The player should NEVER have to memorize the capacity matrix — the interface should make it obvious.

### Wall hover info

When hovering over an existing wall tile, the info card shows conduit capacity:

```
Wattle & Daub Wall (thickness 2)
├── Conduits: none (wall type cannot carry conduits)
```

```
Wood Wall (thickness 3)
├── Wire: empty (can install)
├── Gas pipe: empty (can install)
├── Liquid pipe: empty (can install)
```

```
Stone Wall (thickness 2)
├── Wire: not possible (need thickness 3+)
├── Gas pipe: not possible (stone cannot carry gas)
├── Liquid pipe: not possible (need thickness 3+)
```

```
Wood Wall (thickness 2)
├── Wire: installed ⚡
├── Gas pipe: not possible (need thickness 3+)
├── Liquid pipe: installed 💧 + tap
```

The key: each conduit slot shows one of four states:
- **"empty (can install)"** — green/neutral, clickable
- **"installed"** — blue icon, with optional outlet indicator
- **"not possible (reason)"** — grey, with specific explanation WHY
- Not shown at all for wall types that can't carry anything

### Placing conduits

Two modes for placing conduits in walls:

**Mode A: From the Pipes category (bulk placement)**

When a pipe/wire tool is active and the player clicks/drags over wall tiles:
1. Floor tiles: place pipe/wire block as normal (visible on floor)
2. Wall tiles: automatically embed the conduit inside the wall (if compatible)
3. Incompatible wall tiles: show red tint + tooltip "Wall cannot carry gas pipe" — skip silently during drag

This means a single drag can lay pipe that runs along the floor, enters a wall, runs through wall tiles, exits to floor on the other side. The system automatically chooses floor-placement vs wall-embedding based on what's under the cursor.

The blueprint preview shows:
- **Blue** on compatible wall tiles (conduit will be embedded)
- **Red/grey** on incompatible wall tiles (can't place here)
- **Normal blue** on floor tiles (standard placement)

**Mode B: From the context menu (per-tile)**

Right-click a wall tile → context menu shows available conduit options:
- "Install wire" (if wall supports + no wire yet)
- "Install gas pipe" (if wall supports + no gas yet)
- "Install liquid pipe" (if wall supports + no liquid yet)
- "Add outlet / vent / tap" (if conduit already installed)

Greyed-out entries with reason for anything not possible:
- "Install gas pipe (stone cannot carry gas)" — grey, not clickable
- "Install wire (need thickness 3+)" — grey, not clickable

### Wall type selection feedback

When BUILDING a new wall (placing wall blueprint), the build tooltip should preview conduit capacity:

```
Wood Wall (thickness 2)
  Cost: 2 planks
  Conduits: wire, liquid pipe
```

```
Stone Wall (thickness 3)
  Cost: 3 stone
  Conduits: wire, liquid pipe (no gas)
```

```
Wattle & Daub (thickness 2)
  Cost: 2 sticks
  Conduits: none
```

This helps the player choose wall types WITH infrastructure in mind before building. "I want pipes here, so I need wood or insulated, thickness 3+."

### Placing outlets

After a conduit is installed, the player can add an outlet:
1. Right-click wall tile with conduit → "Add outlet" / "Add vent" / "Add tap"
2. The outlet appears on the room-facing side of the wall
3. Requires a small material cost (1 wire for outlet, 1 pipe fitting for vent/tap)
4. Only one outlet per conduit type per tile

The outlet is visible in normal view — a small widget on the wall face. The conduit behind it remains hidden.

### Visual feedback during placement

**Drag-placing pipes through walls:**

```
Floor    Wall(ok)  Wall(ok)  Wall(NO)  Floor
  ===     [===]     [===]     ❌        ===
 blue     blue      blue      red      blue
```

The `[===]` indicates the pipe is embedded in the wall (brackets = hidden). The `❌` shows an incompatible wall. The drag skips it or terminates, and the player sees exactly why.

**Thickness indicator on wall blueprint:**

When placing walls, the thickness selector could show small icons indicating what each thickness level enables:

```
Thickness: [1] [2⚡] [3⚡💧] [4⚡💧💨]
```

Where ⚡=wire, 💧=liquid, 💨=gas. Instant visual shorthand for "if I build thickness 3, I can run wire and liquid through it."

### Removing conduits

**Remove conduit only (preserve wall):**
Right-click wall → "Remove wire" / "Remove gas pipe" / "Remove liquid pipe"
- A pleb extracts the conduit (takes time, like construction in reverse)
- Material recovered (1 wire / 1 pipe item returned)
- Network breaks at this point

**Destroy wall (conduits lost):**
- Warning tooltip: "This wall contains: wire, liquid pipe. Conduits will be destroyed."
- If confirmed: wall and conduits are both destroyed
- No material recovery for embedded conduits (they're cemented in)
- This makes wall destruction a real cost when infrastructure is embedded

## Visual Representation

### Normal view

Conduits inside walls are INVISIBLE in normal view. That's the point — clean interiors. The wall looks like a wall.

Outlets/vents/taps are visible: small rectangles on the wall face (rendered in the raytrace shader as part of the wall face rendering).

### Overlay modes

When the player activates an overlay (Power, Gas, Liquid):
- Conduits inside walls glow through the wall in the overlay color
- Wire conduits: blue glow through wall material
- Gas conduits: grey/white glow
- Liquid conduits: cyan/blue glow
- The overlay already shows floor-level pipes/wires; wall conduits just add to it

This is satisfying — switching to the power overlay reveals your hidden infrastructure glowing through the walls like an X-ray.

### Shader implementation

The raytrace shader already renders wall faces. For overlay modes:
```wgsl
// In pipe/power overlay section:
let cd = conduit_data[grid_idx];
if is_wall && (cd & 0x3) != 0u && showing_power_overlay {
    // Glow wire through wall
    color = mix(color, vec3(0.3, 0.5, 0.9), 0.4);
}
```

## Interaction with Existing Wall Features

### Doors and windows

Conduits cannot run through door tiles (the door swings — it would sever the pipe). Conduits CAN run through window tiles (the conduit runs through the wall material above/below the window opening).

### Wall material

Conduit capacity depends on thickness, not material. A thin wattle wall and a thin stone wall both fit only wire. But the stone wall is more durable (conduit survives damage better — future system).

### Wall removal

Removing a wall segment that contains conduits breaks the conduit network. The pipe/power sim immediately reflects the break. If this disconnects a section of your network, you get a notification.

## Buried Conduits — Underground Infrastructure

### The trench method

Walls aren't the only way to hide infrastructure. The oldest method: dig a trench, lay the pipe, backfill. This works anywhere — between buildings, across open ground, under paths.

### How it works

1. **Dig a trench.** Player designates a dig line (like the existing dig zone). Pleb digs a shallow ditch — the tile becomes BT_DUG_GROUND at depth 1 (shallowest).
2. **Lay conduit.** Player places pipe/wire in the trench. The conduit sits in the dug tile — visible while the trench is open.
3. **Backfill.** Player designates backfill (or pleb auto-backfills). The tile returns to BT_GROUND but retains its conduit data. The conduit is now underground and invisible.

After backfilling, the tile looks like normal ground. Footpaths can form over it. Plants can grow. But the conduit data persists — the pipe/wire network still includes this tile.

### Why bury instead of wall-embed?

| Method | Speed | Cost | Where | Visibility |
|--------|-------|------|-------|-----------|
| Floor placement | Instant | Pipe item only | Anywhere | Always visible (ugly) |
| Wall conduit | Instant | Pipe + wall | Only in walls | Hidden in wall |
| Buried | Slow (dig + lay + fill) | Pipe + labor | Anywhere outdoors | Hidden underground |

Burying is the SLOWEST method but works ANYWHERE. It's the long-distance solution — running water from a spring 30 tiles away, or electric from a solar field to the base. You wouldn't bury pipe for a 3-tile kitchen run (use the wall). You'd bury pipe for a 40-tile supply line.

### What can be buried

All three conduit types: wire, gas pipe, liquid pipe. Same one-of-each-maximum rule applies. A single trench tile can carry one wire + one gas pipe + one liquid pipe.

No thickness restrictions underground — there's unlimited space in the ground. Any depth of trench works.

### Trench properties

- **Open trench:** Visible conduit. Slows movement (walking across a ditch). Fills with water in rain. Creatures can fall in (minor obstacle).
- **Backfilled trench:** Invisible conduit. Normal ground — no movement penalty, no visual. But: the ground is looser (lower compaction) so paths don't form as quickly over backfill.
- **Digging through existing conduit:** If a pleb digs a tile that has buried conduit, the conduit is exposed (reverts to open trench). Warning: "Buried wire exposed!" If they dig DEEPER (depth 2+), the conduit is destroyed. Warning: "Buried pipe destroyed!"

### Time cost

Trenching is labor-intensive:
- Dig trench: same as normal digging (~3-5 game-seconds per tile with pick, longer by hand)
- Lay conduit: ~1 game-second per tile (just placing it in the ditch)
- Backfill: ~2 game-seconds per tile (shoveling dirt back)

Total: ~6-8 seconds per tile for the full cycle. For a 30-tile supply run, that's 3-4 game-minutes of dedicated labor. A real investment.

### Frost and depth (future)

In cold climates, shallow burial risks freezing. A frozen liquid pipe bursts (breaks the conduit, spills water). Deep burial (depth 2+) is frost-safe. This creates a climate-specific decision: bury deeper in cold regions (more digging labor) or risk burst pipes in cold snaps.

### Visual in overlay mode

Buried conduits show as dashed/dotted lines in overlay mode (vs solid lines for floor and wall conduits). Color-coded as usual (blue=wire, grey=gas, cyan=liquid). The dashed style communicates "underground" at a glance.

### Data structure

Uses the same `conduit_data: Vec<u16>` as wall conduits. Any tile can have conduit bits set — wall tiles, ground tiles, dug tiles. The simulation reads conduit_data regardless of block type. The difference is purely visual/interaction:
- Wall tile + conduit = hidden in wall
- Ground tile + conduit = buried (was backfilled)
- Dug tile + conduit = visible in open trench
- Any other tile + conduit = floor-level (legacy visible placement)

### Placement UX

When placing pipe/wire on a DUG_GROUND tile, the system embeds it as a buried conduit (conduit_data bit) instead of placing a BT_PIPE block. The player doesn't choose — if there's a trench, the conduit goes in it.

After laying conduit in a trench, the context menu for that tile shows "Backfill" to close the trench. Or: a "Backfill zone" tool that designates multiple trench tiles for filling, similar to dig zone.

### Transitions between surface, wall, and underground

No special transition block needed. Connections are **implicit** — the simulation checks neighbors for conduit_data in addition to block types. A floor-level BT_WIRE adjacent to a wall tile with wire conduit data = connected. A floor-level BT_PIPE adjacent to a backfilled tile with pipe conduit data = connected.

**Three transition types and how they render:**

| Transition | When | Visual |
|------------|------|--------|
| Floor → wall | Floor pipe/wire meets wall with conduit | Small penetration mark on wall face (circle where pipe enters) |
| Floor → underground | Floor pipe/wire meets trench or backfilled conduit | Pipe angles downward at tile edge |
| Wall → underground | Wall conduit meets buried conduit below | Pipe exits wall base into ground (through foundation) |

**The trench as natural transition:**

The simplest visual transition: leave the first/last tile of a trench OPEN (not backfilled). The open trench tile shows the pipe sitting in the ditch — literally the point where it enters the earth. Backfill everything in between. The result:

```
Floor pipe ═══ [open trench with pipe visible] ··· backfilled ··· [open trench] ═══ floor pipe
              (entry point)                    (hidden)           (exit point)
```

The open ends are the access points. If you need to repair the buried section, you dig it up from these ends. If the pipe breaks mid-run, you have to dig down to find the break.

**For wall transitions:** the shader renders a small mark on the wall face where the conduit enters — like a junction box or pipe collar. This mark is always visible (even in normal view, not just overlay) because it's a physical thing on the wall surface. Subtle: a small dark circle or rectangle on the wall face at the base.

**Implementation:** transitions are purely visual. The simulation doesn't care — a conduit bit is a conduit bit, whether in a wall, underground, or on the floor. The raytrace shader detects boundary conditions (this tile has surface pipe, neighbor has conduit_data but no surface pipe) and renders the appropriate transition visual.

## Implementation Order

1. **conduit_data Vec<u16>** — add to App, initialize to 0, upload as GPU buffer
2. **Placement validation** — check wall type + thickness when placing conduit
3. **Power grid integration** — power shader reads conduit_data for wire connectivity
4. **Pipe network integration** — pipe graph builder includes conduit gas/liquid cells
5. **Overlay rendering** — conduits glow through walls (solid) / underground (dashed) in overlay modes
6. **Trench conduit placement** — pipe/wire placed on DUG_GROUND embeds as conduit
7. **Backfill mechanic** — backfill zone tool, tile returns to BT_GROUND with conduit data preserved
8. **Wall outlet feature** — new wall feature type, powers adjacent devices wirelessly
9. **Gas vent / liquid tap** — wall features for gas/liquid access
10. **Material costs** — consume wire/pipe items when placing conduits
11. **Removal/warning** — handle wall destruction and accidental dig-through of buried conduits

## Design Principles

1. **Hidden by default, revealed on demand.** Conduits are infrastructure, not decoration. They should be invisible during normal gameplay and visible in overlay/debug modes.
2. **Thickness is meaningful.** The choice between thin and thick walls isn't just structural or aesthetic — it's an infrastructure decision.
3. **One of each, maximum.** Simple rules. No stacking. No complexity explosion.
4. **Outlets bridge the gap.** Without outlets, hiding wires is pointless. Outlets are the interface between hidden infrastructure and visible room equipment.
5. **Network continuity matters.** Breaking a wall breaks its conduits. Plan your infrastructure before your walls — or pay to retrofit.

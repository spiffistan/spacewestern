# DN-032: Tool Components, Durability, and Repair

**Status:** Draft
**Depends on:** DN-025 (primitive tool chain), DN-031 (crash salvage)
**Related:** items.toml, item_defs.rs, simulation.rs

## Problem

Tools are currently monolithic — a "Stone Axe" is one item with one durability counter. This misses the reality that primitive tools are ASSEMBLED from parts. A stone axe is a blade + a handle + binding. Each part can fail independently. Crafting is assembly. Repair is replacement. The same component system serves both.

## Core Concept: Tools Are Assemblies

Every tool above Tier 0 (hammerstone, loose rocks) is made from 2-3 components. Each component:
- Has a material type (stone, stick, fiber, flint, metal)
- Has a failure weight (how likely this part breaks)
- Determines part of the tool's stats (blade → damage/sharpness, handle → speed/reach, binding → holds it together)

When a tool breaks, the system rolls which component failed. Repair requires replacing THAT component.

## Component Definitions

### Blade/Head Components

The business end of the tool. Determines damage, sharpness, and tool type.

| Component | Material | Durability contribution | Where it comes from |
|-----------|----------|------------------------|-------------------|
| Knapped stone edge | 1 rock | Low (25 uses) | Knap with hammerstone |
| Shaped stone head | 1 rock | Medium (40 uses) | Knap with hammerstone, slower |
| Flint edge | 1 flint | High (60 uses) | Knap from flint nodule |
| Flint head | 1 flint | High (70 uses) | Knap from flint nodule |
| Metal head | 1 iron ingot | Very high (200 uses) | Forge (future) |

### Handle Components

What you hold. Determines reach, swing speed, and ergonomics.

| Component | Material | Durability contribution | Where it comes from |
|-----------|----------|------------------------|-------------------|
| Bare grip (no handle) | — | N/A | Hammerstone, stone blade |
| Stick handle | 1 stick | Medium (50 snaps) | Gathered |
| Shaped handle | 1 stick + knife | High (80 snaps) | Carved with knife |
| Hardwood handle | 1 log + saw | Very high (150 snaps) | Saw horse (future) |

### Binding Components

What holds blade to handle. The weak link in most tools.

| Component | Material | Durability contribution | Where it comes from |
|-----------|----------|------------------------|-------------------|
| None | — | N/A | Hammerstone (just a rock in hand) |
| Fiber wrap | 1 fiber | Low (30 uses) | Gathered from dustwhisker |
| Rope lashing | 1 rope | Medium (60 uses) | Crafted from fiber |
| Sinew binding | 1 sinew (future) | High (100 uses) | From animal butchering |
| Metal rivets | 1 metal bit (future) | Very high (200 uses) | Forged |

## Tool Assembly Table

| Tool | Head | Handle | Binding | Total craft cost |
|------|------|--------|---------|-----------------|
| Hammerstone | — (IS the rock) | — (bare grip) | — | 1 rock (instant, no craft) |
| Stone Blade | Knapped stone | — (bare grip) | — | 1 rock + hammerstone (10s) |
| Digging Stick | — (sharpened tip) | Stick handle | — | 1 stick + campfire to harden (10s) |
| Stone Axe | Shaped stone head | Stick handle | Fiber wrap | 1 rock + 1 stick + 1 fiber (20s) |
| Stone Pick | Shaped stone head | Stick handle | Fiber wrap | 1 rock + 1 stick + 1 fiber (15s) |
| Hunting Knife | Knapped stone edge | Stick handle | Fiber wrap | 1 rock + 1 stick + 1 fiber (15s) |
| Flint Blade | Flint edge | — (bare grip) | — | 1 flint + hammerstone (15s) |
| Flint Axe | Flint head | Stick handle | Fiber wrap | 1 flint + 1 stick + 1 fiber (20s) |
| Flint Pick | Flint head | Stick handle | Fiber wrap | 1 flint + 1 stick + 1 fiber (15s) |

## Failure and Repair

### How Breaking Works

Each component has a failure weight. When durability hits 0, the system rolls to determine which component failed:

```
Stone Axe: head 30%, handle 50%, binding 20%
```

The handle breaks most often because wood snaps under stress. The binding frays. The stone head chips least often.

The tool becomes "Broken Stone Axe" with a tag indicating WHICH component failed: "Broken Stone Axe (handle snapped)." This determines the repair cost.

### Repair Costs

Repair = replace the failed component:

| Failed component | Repair cost | Time | Where |
|-----------------|-------------|------|-------|
| Fiber wrap (frayed) | 1 fiber | 5s | In the field (no workbench) |
| Stick handle (snapped) | 1 stick | 8s | In the field |
| Stone head (chipped) | 1 rock | 12s | Needs hammerstone |
| Flint head (chipped) | 1 flint | 15s | Needs hammerstone |
| Rope lashing (worn) | 1 rope | 5s | In the field |

Simple repairs (binding, handle) can be done in the field — the pleb pauses, replaces the part, continues. Head replacement needs a hammerstone for re-knapping.

Repaired tools return to 60% durability (the NEW component is fresh but the other components are still worn).

### Auto-Repair Behavior

The pleb handles all of this autonomously:

1. **Tool reaches worn (30%)**: pleb auto-sharpens after current task (2-3s pause, needs hammerstone/rock). Restores to ~60%. This maintains the HEAD component.

2. **Tool breaks**: pleb checks inventory for the failed component's material. If available: repairs immediately (5-15s). If not: drops the broken tool, switches to alternative, broken tool enters work queue.

3. **Work queue repair**: an idle pleb at a workbench picks up broken tools from nearby ground/storage, fetches materials, repairs. This handles the case where the field pleb didn't have materials.

The player never clicks "repair." They ensure materials are available (strategic) and the system handles the rest (automatic).

## Why Components Matter

### For Crafting
Making a tool isn't "click craft stone axe." It's: knap a head (needs hammerstone + rock), cut a handle (needs stick), wrap the binding (needs fiber). Each step is a mini-activity. The pleb does them in sequence automatically when you queue "craft stone axe."

### For Quality (future)
A tool's components could have quality based on the crafter's skill. A skilled knapper makes a better head (higher durability, sharper edge). A sloppy binding frays faster. This creates variation between two "stone axes" — one might be better because the head was well-knapped.

### For Upgrade
Want to upgrade your stone axe to flint? You don't craft a whole new tool — you replace the head. The handle and binding transfer. Cost: 1 flint + hammerstone time. Much cheaper than building from scratch. This makes partial upgrades viable: "I can't afford a full flint toolkit, but I can re-head my best axe."

### For Scavenging
A broken tool found in a crash crate isn't useless — the handle might still be good. Salvage the intact components and combine them with fresh parts. "The handle from this broken axe fits our spare flint head."

## Data Structure

```rust
// In items.toml (per tool definition)
[[item]]
id = 500
name = "Stone Axe"
components = [
    { part = "head", material = "rock", failure_weight = 30 },
    { part = "handle", material = "stick", failure_weight = 50 },
    { part = "binding", material = "fiber", failure_weight = 20 },
]

// In ItemStack (runtime)
pub struct ItemStack {
    pub item_id: u16,
    pub count: u16,
    pub durability: u16,
    pub broken_part: Option<String>,  // which component failed, if broken
}
```

The `broken_part` field is only set when durability hits 0. It tells the repair system which material to consume.

## Interaction with Other Systems

- **DN-025 (primitive tools)**: the tool progression IS the component system. Better materials → better components → longer-lasting tools.
- **DN-031 (crash salvage)**: damaged crash tools have specific broken parts. "Broken Stone Axe (binding frayed)" — needs 1 fiber to fix.
- **DN-019 (knowledge)**: crafting a tool requires knowledge of how to assemble it. Knapping is a skill. A pleb who's never knapped might chip the stone wrong (failure chance).
- **DN-022 (skill scale)**: higher crafting skill → higher quality components → more durable tools. Eventually, a master crafter's tools barely wear.
- **DN-026 (discovery)**: first time crafting a new tool type is a discovery event: "Ada figured out how to lash a flint head to a handle."

## Auto-Management: Zero Micromanagement

### Design Goal

The player manages the SUPPLY CHAIN (keep materials stocked, set priorities). Individual tool maintenance is invisible — plebs handle it like eating or sleeping. The component system is internal bookkeeping that creates natural cost variation. The player never tracks head/handle/binding states.

### What the Player Sees

**Colony tool status** — a summary in the resource bar or info panel:
```
Tools: 4 working  1 worn  1 broken
```

**Warning notifications** (only when something needs strategic attention):
- "Running low on fiber — tool repairs at risk"
- "No workbench — broken tools piling up"
- "All axes broken — tree chopping halted"

These are colony-level signals, not per-item alerts.

### Minimum Stock System

The ONLY tool-related UI interaction. At the workbench (or crafting menu), the player sets target inventory levels:

```
Stone Axe:    keep [2] in stock
Stone Pick:   keep [1] in stock
Knife:        keep [2] in stock
Flint Axe:    keep [0] in stock  (don't auto-craft yet)
```

The system counts working tools (equipped on plebs + in storage). When count drops below target, it auto-queues a craft job. If materials aren't available, a warning icon appears on the workbench. The player adjusts numbers as the colony grows.

Early game: keep 1 of each. Mid game: keep 2-3. Late game: 4+. This is the player's tool "policy" — set once, adjust occasionally.

### Priority Chain (all automatic)

**Priority 1 — Self-sharpen (instant, in the field):**
Pleb's tool hits 30% durability → brief pause (2-3s) between tasks → sharpens with hammerstone/rock from own inventory → back to 60%. No work queue. No assignment. Like eating — they just do it. If no sharpening material in inventory, they skip and keep working.

**Priority 2 — Self-repair (seconds, in the field):**
Tool breaks → pleb checks own inventory for the failed component's material. Stick in pocket + handle snapped = repair on the spot (5-15s). Never enters work queue. If material not available → drops broken tool, switches to next tool of same type (or bare hands).

**Priority 3 — Workshop repair (work task, automatic):**
Broken tool on ground or in storage + workbench exists + materials available → idle pleb with crafting priority picks up tool, fetches materials, walks to workbench, repairs. Same system as hauling/construction — fully automatic, prioritized by work settings.

**Priority 4 — Replacement crafting (work task, automatic):**
Colony tool count below minimum stock target → auto-queue craft job at workbench. Pleb with crafting priority gathers materials, assembles tool. Placed in storage when complete. Same priority system as all other work.

### What the Player NEVER Does

- Click "repair" on a specific tool
- Choose which tool a pleb uses
- Track individual component durability
- Manually queue repair jobs
- Assign repair tasks to specific plebs
- Dismiss "tool worn" notifications (there are none)

### What the Player DOES Do (Strategic)

1. **Material supply**: ensure rocks, sticks, fiber, flint are being gathered. If the material pipeline breaks, tools can't be maintained. This is the same concern as "do we have enough wood for fire?" — supply chain management, not item management.

2. **Stock targets**: adjust minimum stock levels at the workbench. More plebs = more tools needed. Upgrading to flint? Set flint axe target to 2, reduce stone axe target to 0.

3. **Workbench placement**: build a workbench and assign crafting priority. Without a workbench, broken tools can't be repaired and replacements can't be crafted. The workbench IS the tool maintenance infrastructure.

4. **Tier decisions**: when to invest in finding flint (exploration) to upgrade from stone tools (which break constantly) to flint tools (which last 3x longer). This is the meaningful progression decision, not "should I repair this axe."

### Why Components Are Invisible to the Player

The player sees:
- "Stone Axe" (not "Stone Head + Stick Handle + Fiber Wrap")
- "Ada's axe broke" (not "Ada's axe handle snapped, component #2 failed")
- Craft recipe: "1 rock + 1 stick + 1 fiber" (not "knap head → attach handle → wrap binding")
- Repair cost: "1 stick" (because the handle broke — but the player just sees the material cost)

The event log might say "handle snapped" for flavor, but the player's ACTION is the same regardless: ensure sticks are available. Components add natural cost variation and narrative flavor without adding UI complexity.

## Implementation Order

1. Add `components` field to ItemDef (parsed from items.toml)
2. Add `broken_part` field to ItemStack
3. Implement break-roll: when durability hits 0, roll which component failed
4. Implement field repair: pleb checks inventory for material, repairs if available
5. Implement workbench repair: broken tools enter auto-repair work queue
6. Implement auto-sharpen: pleb pauses and sharpens at 30% durability
7. Minimum stock system: target counts per tool type, auto-queue crafting
8. Component-based crafting: "craft stone axe" = sequential assembly steps
9. Head replacement upgrade: swap component without full re-craft
10. Colony tool status display in resource bar

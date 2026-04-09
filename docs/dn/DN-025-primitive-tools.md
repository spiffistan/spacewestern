# DN-025: Primitive Tool Progression

**Status:** Draft
**Depends on:** DN-023 (sub-tile mining), DN-019 (knowledge/crafting)
**Related:** DN-017 (fauna/food), items.toml, item_defs.rs

## Problem

The game currently starts with manufactured tools (stone axe, stone pick, pistol). This skips the most interesting part of a frontier survival arc: making your first tools from nothing. The Primitive Technology channel's appeal is exactly this — hands → found materials → crude tools → better tools → infrastructure.

Tools should be a progression chain where each tier unlocks capabilities that were previously impossible or painfully slow.

## Design Principles

1. **Hands are always an option.** Everything CAN be done by hand — it's just very slow. Tools multiply speed, they don't gate access (with a few exceptions like hard rock mining).
2. **Each tool tier is a meaningful upgrade.** Not +10% speed, but 2-4x speed. The player FEELS the difference.
3. **Tools break.** Durability creates ongoing demand. You don't craft one axe and never think about tools again.
4. **Tools require maintenance materials.** Sharpening, re-hafting, re-binding. Creates a steady resource drain.
5. **Better materials = better tools.** Same tool design, different material. Flint knife > stone knife. Iron knife > flint knife.

## The Tiers

### Tier 0: Bare Hands

Available from moment one. No crafting needed.

| Action | By hand | Notes |
|--------|---------|-------|
| Gather dustwhisker | Slow (5s) | Pull grass by hand |
| Gather sticks | Slow (8s) | Break dead branches |
| Pick up loose stones | Instant | Small rocks only |
| Harvest berries | Slow (3s per berry) | One at a time |
| Dig soft earth | Very slow (15s) | Scoop with hands |
| Mine rock | Extremely slow (8s per sub-cell) | Bash with found stone, hurts hands |
| Butcher | Cannot | Need a cutting edge |
| Chop tree | Cannot | Need an axe |

Hands establish the baseline. Everything you do with hands, a tool does 2-10x faster.

### Tier 1: Found/Knapped Stone

Made from loose stones and sticks. No workbench needed — crafted in-hand.

**Hammerstone** — a round stone held in the fist. The first "tool."
- Recipe: 1 stone (pick up any loose rock)
- No crafting time — just picking up a rock
- Uses: knapping (making other stone tools), crushing, pounding stakes
- Durability: high (it's a rock)

**Knapped stone blade** — a stone flaked to a cutting edge by striking with hammerstone.
- Recipe: 1 stone + hammerstone (in hand)
- Craft time: 10-15 game-seconds (knapping animation)
- Skill: crafting. Low skill = high failure rate (stone shatters, wasted)
- Uses: cutting (butchering, harvesting fiber), scraping hides
- Durability: low (20-30 uses). Cutting edges dull/chip fast.
- Upgrade: flint blade is sharper, lasts 3x longer

**Stone axe** — blade lashed to a stick handle.
- Recipe: 1 knapped blade + 2 sticks + 1 fiber (binding)
- Craft time: 20 game-seconds
- Uses: chop trees, clear brush, combat (slow). 3x faster than hands for wood gathering
- Durability: medium (50 uses). Handle breaks before blade.

**Stone pick** — pointed stone lashed to a handle.
- Recipe: 1 stone + 1 stick + 1 fiber
- Craft time: 15 game-seconds
- Uses: mining soft rock (chalk, sandstone). 3x faster than hands. Cannot mine hard rock (granite, basalt).
- Durability: medium (40 uses).

**Digging stick** — a sharpened stick, fire-hardened.
- Recipe: 1 stick + campfire (harden the tip)
- Craft time: 10 game-seconds
- Uses: digging soft earth (planting, small holes). 2x faster than hands.
- Durability: low (30 uses). The tip dulls.

### Tier 2: Flint Tools

Flint is found as nodules in chalk and limestone (see DN-023). It's harder and holds a sharper edge than generic stone. Flint tools are the "good" primitive tier.

**Flint blade** — knapped from a flint nodule.
- Recipe: 1 flint + hammerstone
- Craft time: 15 game-seconds (requires more precise knapping)
- Skill: crafting 3.0+. Below that, high failure rate.
- Uses: same as stone blade but sharper. Butchering is faster, cleaner cuts. Better knife.
- Durability: medium (60 uses). Flint holds an edge longer.

**Flint axe** — flint blade hafted to a handle.
- Recipe: 1 flint blade + 2 sticks + 1 fiber
- Uses: 5x faster than hands for wood. Can chop hardwood.
- Durability: medium-high (80 uses).

**Flint pick** — flint point on a handle.
- Recipe: 1 flint + 1 stick + 1 fiber
- Uses: mines all rock types including granite (slowly). 4x faster than hands on soft rock, 2x on hard.
- Durability: medium (60 uses).

**Flint drill** — pointed flint for boring holes.
- Recipe: 1 flint + 1 stick
- Uses: drilling holes in wood/bone (needed for construction joints, handles). Enables better tool construction.
- Durability: low (25 uses). The point wears down.

### Tier 3: Metal Tools (future, requires smelting)

Iron/copper tools from smelted ore. 2-3x better than flint in speed and durability. Required for serious quarrying and construction.

**Iron knife, iron axe, iron pick, iron hammer.** Same designs, vastly better material. An iron pick mines granite at practical speed. An iron axe fells trees in seconds. Iron tools last hundreds of uses.

## Durability System

Each tool has a durability counter (uses remaining). Displayed as a small bar on the item icon.

| Quality | Uses | Notes |
|---------|------|-------|
| Crude stone | 20-30 | Breaks fast, easily replaced |
| Shaped stone | 40-50 | Better, still disposable |
| Flint | 60-80 | Worth maintaining |
| Iron | 200-300 | Valuable, repair instead of replace |

When durability reaches 0:
- Tool breaks with a sound effect
- Pleb gets thought bubble: "My axe broke..."
- Work stops — pleb needs a new tool or reverts to hands
- Broken tool drops as "broken [tool]" — salvage materials

### Repair / Maintenance

Tools can be repaired before they break:
- Stone/flint: re-sharpen with hammerstone (restores ~50% durability, uses crafting skill)
- Iron: re-sharpen at grindstone, re-haft with new handle (future)
- Repair is faster than crafting a new tool — incentivizes maintenance over disposal

## Crafting Without a Workbench

Tier 0-1 tools are crafted in-hand (no workbench). The pleb sits down, takes 10-20 game-seconds, makes the tool. This is important — you shouldn't need infrastructure to make your first tools.

Tier 2 (flint) can also be in-hand but benefits from a workbench (+speed, -failure rate).

Tier 3 (metal) requires forge + anvil. Full infrastructure.

## How This Connects to Mining (DN-023)

| Rock type | Hands | Stone pick | Flint pick | Iron pick |
|-----------|-------|-----------|------------|-----------|
| Chalk | 8s/cell | 3s/cell | 2s/cell | 0.5s/cell |
| Sandstone | 10s/cell | 3s/cell | 2s/cell | 0.8s/cell |
| Limestone | 12s/cell | 4s/cell | 2.5s/cell | 1.0s/cell |
| Granite | 20s/cell | Cannot* | 4s/cell | 1.5s/cell |
| Basalt | 25s/cell | Cannot* | 5s/cell | 2.0s/cell |

*Stone picks bounce off hard rock — not enough hardness to fracture it. You need flint or metal.

This creates a clear progression gate: you CANNOT efficiently mine granite until you find flint. Flint is found in chalk/limestone. So you mine soft rock first (to find flint), then use flint tools to mine hard rock (for building material and ore). The geology drives the tool progression.

## Item IDs

New items to add (in the 500-599 tools range):

| ID | Name | Notes |
|----|------|-------|
| 504 | Hammerstone | No crafting, just pick up a rock |
| 505 | Stone Blade | Knapped cutting edge |
| 506 | Flint Blade | Better cutting edge |
| 507 | Flint Pick | Mines hard rock |
| 508 | Flint Axe | Better wood chopping |
| 509 | Digging Stick | Basic earth digging |
| 510 | Flint Drill | Boring holes, advanced construction |

Existing items to adjust:
- ITEM_STONE_AXE (500): now requires knapped blade + sticks + fiber, not "spawned in inventory"
- ITEM_STONE_PICK (501): same adjustment
- ITEM_KNIFE (503): rename to "Stone Knife" or split into stone/flint/iron variants

## Starting Equipment

At game start, plebs arrive with nothing — or minimal supplies depending on the manifest. The first task is always: pick up rocks, gather sticks, knap blades, make tools. This IS the gameplay for the first 10 minutes.

Optionally: one pleb arrives with a worn stone axe (low durability) as a head start.

## Implementation Order

1. Hammerstone + stone blade items and in-hand crafting
2. Durability system on all tools (uses counter, breaks at 0)
3. Tool speed multipliers for gathering/mining (replace hardcoded values)
4. Flint item + knapping recipe (flint blade, flint pick, flint axe)
5. Digging stick for earth digging
6. Starting equipment rework (minimal or nothing)
7. Repair/sharpening mechanic

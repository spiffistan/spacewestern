# DN-018: Equipment and Inventory System

**Status:** Proposed
**Depends on:** DN-009 (pleb sprites), DN-011 (combat rework), DN-012 (wound system)
**Affects:** pleb.rs, resources.rs, items.toml, simulation.rs, ui.rs, raytrace.wgsl

## Summary

A layered body-equipment system with Diablo-style grid inventory. Equipment is worn in physical layers (pockets → belt → vest → pack), each layer is a 2D grid with differently-sized items, and pouches are sub-containers that nest a tiny-item grid inside a single belt/vest cell. All deployment is automatic — the player makes loadout decisions, colonists use items situationally.

## Problem

The current system has `equipped_weapon: Option<u16>` — a single weapon slot. `PlebInventory` is a flat `Vec<ItemStack>` for hauling. There's no concept of worn gear, no tool requirements for tasks, no way to carry small items (bullets, seeds, coins), and no visual equipment on sprites. A colonist with a pistol looks the same as one with an axe.

## Design Principles

**The body is the inventory.** No abstract grid floating in UI space. Equipment goes where it physically goes on a person: pockets, belt, chest, back. Each layer is a crafted wearable with its own grid dimensions.

**Auto-deploy eliminates micromanagement.** The player chooses WHO carries WHAT (loadout). The colonist chooses WHEN to use it (context). Chop tree → axe auto-drawn. Enemy spotted → pistol auto-drawn. Wounded ally → bandage auto-applied. The player never manually switches tools.

**Equipment gates activities.** Instead of skill-menu unlocks, physical items enable tasks. No knife = can't butcher. No pick = can't mine. No ranged weapon = can't hunt. The crafting system produces capabilities, not the tech tree.

**Rich variety, zero tedium.** The system supports hundreds of distinct item types (RPG-grade variety) while the player interacts with it at the level of "give Kai the hunter loadout" not "drag bullet #47 into slot #3." Pouches are the key mechanism: one belt slot expands into a sub-container holding dozens of tiny items that auto-refill from stockpiles.

**Visible on sprite.** Every equipment layer renders on the colonist. Belt with holstered pistol, knife on hip, pouch dangling. Vest with bulging pockets. Pack on back. You read a colonist's role from their silhouette.

---

## Equipment Layers

Five physical layers, each a separate grid. Layers are unlocked by crafting the wearable.

### Layer 1: Pockets (Default — everyone has them)

- **Grid:** None. Two implicit tiny-item slots.
- **Holds:** Up to 2 tiny items (matches, a single herb, a couple of coins).
- **Craft cost:** Free. Every colonist starts with pockets.
- **Progression:** Day 1. The bare minimum. Can light a fire with pocketed matches, nothing more.
- **Gameplay role:** Teaches the player that tiny items exist and that carrying capacity matters. The moment you find 3 things worth pocketing and only have 2 slots, you want a belt.

### Layer 2: Belt (Early game — first craft priority)

- **Grid:** 1D strip, 4 cells (upgradable to 6 with reinforced belt).
- **Holds:** Small and medium items. Tools, weapons, canteen, pouches.
- **Craft cost:** Leather (from ridgeback hide, future) + rope (existing item 10). Early-game: fiber belt (3 fiber) as a weaker 3-slot version.
- **Progression:** Day 5–10. The colonist can now carry tools for work and a weapon for defense simultaneously.
- **Upgrade path:** Basic fiber belt (3 slots) → leather belt (4 slots) → reinforced belt (6 slots, requires metal buckle).

### Layer 3: Vest / Bandolier (Mid game — specialization begins)

Two mutually exclusive options for the torso slot:

**Vest (general purpose):**
- **Grid:** 3×2 (6 cells).
- **Holds:** Small items only. Bandages, compass, flask, spyglass, pouches. Also provides light armor (damage reduction 10%).
- **Craft cost:** Leather + fiber. Mid-game investment.
- **Progression:** Day 20–30. The colonist becomes self-sufficient — carries medical supplies, navigation tools, utility items.
- **Gameplay role:** The "utility" layer. What makes a colonist feel like a prepared frontier survivor rather than a laborer with a tool.

**Bandolier (combat specialist):**
- **Grid:** 1×8 strip (8 narrow cells — ammo loops).
- **Holds:** Only ammo and throwables. Each cell holds one ammo stack or one grenade/explosive.
- **Craft cost:** Leather + metal fittings.
- **Progression:** Day 30+. Combat-focused colonists.
- **Gameplay role:** Fast reload speed bonus (+30%). The shooter's alternative to the vest. Choosing bandolier over vest means giving up medical self-sufficiency for combat effectiveness. A meaningful build decision.

### Layer 4: Pack (Mid-late game — expedition and hauling)

- **Grid:** 4×4 (16 cells). The largest personal grid.
- **Holds:** All sizes including large items (bedroll 2×2, rifle 1×3, rope coil 2×1).
- **Craft cost:** Leather + wood frame + rope. Significant investment.
- **Movement penalty:** -15% speed (light load) to -30% (full pack). Colonists are slower and louder in the sound sim.
- **Progression:** Day 30+. Not worn routinely — equipped for expeditions, trade runs, hauling jobs.
- **Gameplay role:** Extends range. A colonist without a pack works close to base. A colonist with a pack can go on multi-day expeditions (bedroll, food, water, trade goods). The pack is the difference between "colonist" and "explorer."
- **Auto-equip behavior:** Colonists assigned to hauling work automatically equip a pack from storage before starting. They stow it when switching to non-hauling work. The player doesn't manually equip/unequip.

### Layer 5: Special Slot (Late game — rare items)

One additional slot for rare or unique items that don't fit the other layers:

- **Holster rig:** Adds a second weapon draw slot (cross-draw). The gunslinger's kit.
- **Tool harness:** Adds 2 extra belt slots for heavy tools. The miner/builder's kit.
- **Satchel:** A large pouch (5×5 tiny grid) worn across the body. The trader/doc's kit.
- **Armor plate:** Worn over vest. Significant damage reduction. Heavy. The fighter's kit.

Only one special item at a time. These are rare drops, expensive crafts, or trade acquisitions. They define the colonist's late-game identity.

---

## Item Sizes

Every item has a grid footprint measured in cells. The cell scale is "roughly fist-sized" for belt/vest/pack grids.

### Standard items (belt / vest / pack grids)

| Size | Grid Cells | Examples | Where it fits |
|------|-----------|----------|---------------|
| 1×1 | 1 cell | Knife, canteen, bandage, pouch, flask, compass, hand axe | Belt, vest, pack |
| 2×1 | 2 horizontal | Pistol + holster, rope coil (small), spyglass, hatchet | Belt, vest, pack |
| 1×2 | 2 vertical | Shovel, torch, machete, rifle (compact) | Belt (if tall enough), pack |
| 2×2 | 4 cells | Bedroll, folded hide, armor piece, cooking pot | Pack only |
| 3×1 | 3 horizontal | Rifle (full length), fishing rod, long tool | Pack only |
| 1×3 | 3 vertical | Rifle (slung vertically), staff, long pipe | Pack only |

**Belt constraint:** The belt is a 1D strip (1×N), so only 1-tall items fit. A shovel (1×2) doesn't fit on a belt — it goes in the pack or is carried in-hand. This is physically correct: you can't hang a full-length shovel from a belt. A hand axe (1×1) fits on a belt because it's short enough.

**Vest constraint:** The vest is 3×2, so nothing taller than 2 cells or wider than 3 cells fits. No bedrolls in a vest. Realistic: a vest pocket holds a compass, not a blanket.

### Tiny items (pouch grids only)

Tiny items don't exist on standard grids. They're too small — a single bullet in a fist-sized cell is absurd. They live exclusively inside pouches, which have their own sub-grid at a smaller scale.

| Tiny Size | Grid Cells | Examples | Stack Behavior |
|-----------|-----------|----------|---------------|
| 1×1 tiny | 1 tiny cell | Bullet stack, coin pile, seed packet, herb bundle, nails, match bundle, single gemstone, key | Stacks high within cell — limited by weight |
| 2×1 tiny | 2 tiny cells | Suture kit, key ring, small multi-tool, wire spool, writing charcoal + paper scrap | Occupies 2 cells, doesn't stack |
| 2×2 tiny | 4 tiny cells | Field surgery kit, lock pick set, cartography kit | Rare, only fits large pouches |

**Stacking within tiny cells.** A 1×1 tiny cell holds a *stack* of similar items. How many fit is determined by the pouch's weight budget divided by item weight:

```
stack_count = floor(cell_weight_budget / item_weight)

Example: Bullet pouch, 200g total, 9 cells → ~22g per cell
  Pistol round (1.2g each): 18 per cell
  Rifle round (3.5g each): 6 per cell
  Shotgun shell (5g each): 4 per cell

Example: Seed bag, 300g total, 8 cells → ~37g per cell
  Dustroot seed (0.5g each): 74 per cell (cap at 50 for sanity)
  Bitterbulb seed (2g each): 18 per cell

Example: Coin purse, 500g total, 6 cells → ~83g per cell
  Copper bit (3g each): 27 per cell
  Silver slug (15g each): 5 per cell
  Gold piece (30g each): 2 per cell
  Circuit scrap (8g each): 10 per cell
```

Heavier tiny items fill fewer per cell. Light items stack high. The weight system makes physical sense: a pouch of lead bullets is heavier than a pouch of seeds, even though the pouch is the same size.

---

## The Pouch System

Pouches are the bridge between the standard grid and the tiny-item world. A pouch is a standard-grid item (1×1, fits on belt or in vest) that opens into its own tiny sub-grid.

### Pouch Types

| Pouch | Tiny Grid | Weight Budget | Craft Cost | Primary User |
|-------|-----------|---------------|------------|-------------|
| Bullet pouch | 3×3 (9 cells) | 200g | Leather scrap | Shooter, hunter |
| Seed bag | 2×4 (8 cells) | 300g | Cloth + drawstring | Farmer |
| Herb satchel | 3×2 (6 cells) | 150g | Leather + strap | Doc, cook |
| Coin purse | 2×3 (6 cells) | 500g | Leather | Trader, any |
| Parts pouch | 3×3 (9 cells) | 400g | Leather + buckle | Mechanic, builder |
| Specimen bag | 2×3 (6 cells) | 250g | Cloth + padding | Prospector, scout |
| Spice pouch | 2×2 (4 cells) | 100g | Cloth | Cook |

### Pouch Behavior

**Auto-refill.** When a colonist with a pouch passes within range of a matching stockpile (storage crate flagged for the right item category), the pouch tops up automatically. The farmer walks past the seed crate → seed bag refills. The shooter passes the ammo box → bullet pouch refills. No player interaction.

**Auto-consume.** Items in pouches are consumed automatically when the situation requires:
- Bullet pouch: rounds consumed when firing (already tracked by `ammo_loaded` / `magazine_size`)
- Seed bag: seeds consumed when planting
- Herb satchel: herbs consumed when treating wounds (doc applies best available herb)
- Parts pouch: nails/screws consumed during construction (faster build with parts available)
- Coin purse: coins used automatically during trade interactions

**Pouch UI.** In the colonist panel, a pouch shows as a standard-grid item with an expand indicator. Clicking it reveals the tiny sub-grid with contents. Collapsed view shows a summary: "👝 Bullets: 42 rounds, 118g/200g". Expanded view shows the Diablo-style tiny grid with each cell's contents and stack count.

**Multiple pouches.** A colonist can carry multiple pouches (each taking a belt or vest slot). The doc might have an herb satchel on the belt AND a coin purse in the vest. The hunter has a bullet pouch on the belt AND a specimen bag in the vest for collecting trophies. The constraint is always slot space — two pouches on a 4-slot belt means only 2 slots left for tools.

---

## Auto-Deploy System

The core mechanism that prevents micromanagement. The colonist's current activity determines which belt/vest/pack item is "active" (in-hand or in-use).

### Activity → Item Mapping

| Activity | Required Item | Behavior |
|----------|--------------|----------|
| Chopping trees | Axe (any) | Drawn from belt automatically |
| Mining rock | Pick (any) | Drawn from belt |
| Digging terrain | Shovel (any) | Drawn from belt |
| Butchering creature | Knife | Drawn from belt. No knife = can't butcher |
| Harvesting crops | Knife or bare hands | Knife gives +30% harvest speed |
| Planting | Bare hands + seed bag | Seeds consumed from pouch automatically |
| Cooking | Knife (prep) | Used during ingredient prep phase |
| Building | Hammer or bare hands | Hammer (future) gives +20% build speed |
| Treating wounds | Bandage + herbs | Consumed from vest/pouch. Doc uses best herb. |
| Combat (ranged) | Pistol / rifle | Drawn from belt or pack. Ammo from bullet pouch. |
| Combat (melee) | Best melee weapon | Auto-selects highest-damage belt weapon |
| Hunting | Ranged weapon | Must have ranged weapon + ammo to accept hunt task |
| Trading | Coin purse | Coins accessed automatically during trade |
| Hauling | Bare hands (or rope for heavy) | Rope from belt for dragging logs/carcasses |
| Idle / walking | Nothing drawn | Weapons holstered, tools on belt |

### Missing Equipment Behavior

If a colonist lacks the required item for a task, they **skip it** and move to the next priority. The player sees a notification: "Kai can't butcher — no knife." This teaches the player to equip their colonists properly without blocking gameplay. The colonist does other work instead.

For optional-but-helpful items (knife for harvesting, hammer for building), the colonist works without it at reduced speed. Having the right tool is an optimization, not a hard gate (except for butchering, mining, and hunting which physically require the tool).

### Weapon Swap Timer

Switching from tool to weapon (or vice versa) takes 0.5–1.0 seconds. The `weapon_swap_timer` field already exists on `Pleb`. During swap, the colonist can't attack or work. This makes ambush situations dangerous — a farmer caught by duskweavers while holding a shovel needs a full second to draw their pistol.

The swap animation is visible: old item goes to belt, new item comes to hand. The sprite shows the transition (DN-009 sprite layers).

---

## Data Model

### Item Properties (items.toml extensions)

```toml
# New fields on item definitions:

[[item]]
id = 70
name = "Hunting Knife"
icon = "🔪"
category = "tool"
stack_max = 1

# Grid size (standard cells)
grid_w = 1               # width in standard grid cells
grid_h = 1               # height in standard grid cells

# Where it can be placed
fits_belt = true
fits_vest = true
fits_pack = true

# Tool capability
tool_type = "knife"       # enables: butcher, harvest_bonus, cooking_prep
tool_speed = 1.0          # multiplier for task speed

# Combat stats (dual-use)
melee_damage = 0.20
melee_speed = 1.8         # fast
melee_range = 0.8         # short
melee_knockback = 0.2
melee_bleed = 0.6         # knives cause bleeding
weapon_type = 5           # for sprite rendering

[[item]]
id = 71
name = "Pistol"
icon = "🔫"
category = "weapon"
stack_max = 1
grid_w = 2
grid_h = 1
fits_belt = true
fits_vest = false         # too big for vest pockets
fits_pack = true
is_ranged = true
ranged_spread = 0.10
ranged_aim_speed = 1.5
magazine_size = 6
reload_time = 2.5
weapon_type = 4

[[item]]
id = 80
name = "Bullet Pouch"
icon = "👝"
category = "container"
stack_max = 1
grid_w = 1
grid_h = 1
fits_belt = true
fits_vest = true
fits_pack = true
is_pouch = true
pouch_grid_w = 3          # tiny sub-grid dimensions
pouch_grid_h = 3
pouch_weight_budget = 200 # grams
pouch_accepts = ["ammo", "fire_starter"]  # item categories allowed
```

### Tiny Item Properties

```toml
# Tiny items — only exist inside pouches

[[item]]
id = 100
name = "Pistol Rounds"
icon = "•"
category = "ammo"
stack_max = 50            # max per tiny cell (overridden by weight if lower)
is_tiny = true
tiny_grid_w = 1           # footprint in tiny grid
tiny_grid_h = 1
weight_grams = 1.2        # per unit — determines stack size in pouch
ammo_type = "9mm"         # matches weapon's ammo requirement

[[item]]
id = 101
name = "Rifle Rounds"
icon = "◦"
category = "ammo"
stack_max = 20
is_tiny = true
tiny_grid_w = 1
tiny_grid_h = 1
weight_grams = 3.5
ammo_type = "rifle"

[[item]]
id = 110
name = "Copper Bits"
icon = "●"
category = "currency"
stack_max = 99
is_tiny = true
tiny_grid_w = 1
tiny_grid_h = 1
weight_grams = 3.0
trade_value = 1           # base unit of currency

[[item]]
id = 111
name = "Silver Slugs"
icon = "○"
category = "currency"
stack_max = 20
is_tiny = true
tiny_grid_w = 1
tiny_grid_h = 1
weight_grams = 15.0
trade_value = 10          # worth 10 copper

[[item]]
id = 112
name = "Circuit Scrap"
icon = "✦"
category = "currency"
stack_max = 10
is_tiny = true
tiny_grid_w = 1
tiny_grid_h = 1
weight_grams = 8.0
trade_value = 25          # rare, valuable salvage
```

---

## Pleb Struct Changes

Replace the current single-weapon system with the layered equipment model:

```rust
/// Equipment grid — a 2D array of item slots.
#[derive(Clone, Debug)]
pub struct EquipGrid {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<Option<EquipSlot>>,  // width × height, None = empty
}

/// A single cell in an equipment grid, or a multi-cell item's anchor.
#[derive(Clone, Debug)]
pub struct EquipSlot {
    pub item_id: u16,
    pub is_anchor: bool,         // true = top-left cell of multi-cell item
    pub anchor_index: usize,     // points to anchor cell (for non-anchor cells)
}

/// A pouch's contents — tiny sub-grid with weight tracking.
#[derive(Clone, Debug)]
pub struct PouchContents {
    pub grid: EquipGrid,         // the tiny sub-grid
    pub weight_budget: f32,      // max grams
    pub weight_used: f32,        // current grams
    pub accepts: Vec<String>,    // item category filter
    pub stacks: Vec<TinyStack>,  // actual item data per occupied cell
}

/// Stack of tiny items in one tiny-grid cell.
#[derive(Clone, Debug)]
pub struct TinyStack {
    pub item_id: u16,
    pub count: u16,
    pub cell_index: usize,       // which tiny-grid cell
}

/// All worn equipment on a pleb.
#[derive(Clone, Debug)]
pub struct PlebEquipment {
    pub pockets: [Option<u16>; 2],          // 2 tiny-item slots (no grid, just IDs)
    pub belt: Option<EquipGrid>,            // None = no belt equipped
    pub torso: Option<TorsoEquip>,          // vest or bandolier
    pub pack: Option<EquipGrid>,            // None = no pack
    pub special: Option<u16>,               // single rare item slot
    pub pouches: Vec<PouchContents>,        // all active pouches (referenced by grid cells)
    pub active_item: Option<u16>,           // currently in-hand (drawn from belt/pack)
}

#[derive(Clone, Debug)]
pub enum TorsoEquip {
    Vest(EquipGrid),       // 3×2 general utility
    Bandolier(EquipGrid),  // 1×8 ammo specialist
}
```

The existing `equipped_weapon: Option<u16>` maps to `equipment.active_item`. The existing `PlebInventory` remains for hauling (carried items separate from worn equipment). The `weapon_type` field on `GpuPleb` reads from `active_item`'s item definition.

---

## GpuPleb Rendering Extensions

The GpuPleb struct needs additional fields to render equipment layers:

```rust
// Additions to GpuPleb (must match WGSL struct):
pub belt_item_1: f32,    // item_id of belt slot 1 (0 = empty)
pub belt_item_2: f32,    // item_id of belt slot 2
pub belt_item_3: f32,    // etc.
pub has_vest: f32,       // 0.0 or 1.0
pub has_pack: f32,       // 0.0 or 1.0
pub has_bandolier: f32,  // 0.0 or 1.0
```

The raytrace shader uses these to draw equipment on the pleb sprite:
- Belt items render as small colored shapes at hip level (knife = short blade, pistol = holster bulge)
- Vest renders as a slightly different torso color with pocket bumps
- Pack renders as a rectangle on the back, colored by material
- Active item (in-hand) already renders via the existing weapon_type system

Full equipment rendering details belong in a DN-009 update; this DN defines the data model.

---

## Auto-Refill Logic

Pouches refill when the colonist is within `NEAR_INTERACT_RADIUS` (2.0 tiles) of a storage crate containing matching items.

```
fn try_refill_pouches(pleb: &mut Pleb, crates: &[StorageCrate]) {
    for pouch in &mut pleb.equipment.pouches {
        if pouch.weight_used >= pouch.weight_budget { continue; }

        for crate in crates_in_range(pleb.pos, NEAR_INTERACT_RADIUS, crates) {
            for stack in &mut crate.contents {
                if !pouch.accepts_category(stack.category) { continue; }
                let item_weight = item_registry.get(stack.item_id).weight_grams;
                let can_add = floor((pouch.weight_budget - pouch.weight_used) / item_weight);
                let transfer = min(can_add, stack.count);
                if transfer > 0 {
                    pouch.add(stack.item_id, transfer);
                    stack.count -= transfer;
                }
            }
        }
    }
}
```

This runs during the pleb's movement tick — no special "refill" activity needed. Walking past the ammo crate on the way to guard duty is enough. Colonists with regular patrol routes past stockpiles stay topped up naturally. A colony with good stockpile placement never has empty pouches.

---

## UI Design

### Colonist Panel — Equipment Tab

The existing colonist info panel (drawn in `ui.rs`) gets an equipment tab showing all layers:

```
┌─ Kai "Patches" Novak ────────────────────────┐
│ [Needs] [Equipment] [Skills] [Log]            │
├───────────────────────────────────────────────┤
│ POCKETS     🔥×8  🌿×2                        │
│                                               │
│ BELT [████░░]  4/6 slots                      │
│ ┌────┬────┬─────────┬────┐                    │
│ │ 🪓  │ 🔪  │   🔫    │ 👝  │                    │
│ │ Axe│Knife│ Pistol │Ammo│                    │
│ └────┴────┴─────────┴────┘                    │
│   └→ Bullet pouch: 36 rds, 118g/200g         │
│                                               │
│ VEST [███░░░]  3/6 slots                      │
│ ┌────┬────┬────┐                              │
│ │ 🩹  │ 🩹  │ 🔭  │                              │
│ │Band│Band│Spy-│                              │
│ ├────┼────┤glas│                              │
│ │ 🧭  │ 👝  │    │                              │
│ │Comp│Herb│    │                              │
│ └────┴────┴────┘                              │
│                                               │
│ PACK  Not equipped                            │
└───────────────────────────────────────────────┘
```

Clicking a pouch cell expands the tiny sub-grid inline. Clicking an empty cell opens a picker showing items available in nearby crates that fit the slot. Drag-and-drop between cells for manual arrangement (optional — auto-arrange available).

### Loadout Presets

To avoid per-colonist micromanagement, the player can define named loadout presets:

- **Hunter:** Belt: rifle, knife, rope, bullet pouch. Vest: bandage, compass, spyglass.
- **Farmer:** Belt: knife, shovel, canteen, seed bag. Vest: bandage, gloves.
- **Builder:** Belt: axe, knife, parts pouch. Vest: bandage, nails (in parts pouch).
- **Guard:** Belt: pistol, knife, bullet pouch. Bandolier: ammo stacks.
- **Doc:** Belt: knife, pistol, herb satchel, bullet pouch. Vest: bandages ×3, splint, suture kit.
- **Trader:** Belt: pistol, coin purse, bullet pouch. Pack: trade goods.

Assigning a preset to a colonist auto-fills their equipment from available stockpiles. If an item isn't available, the slot stays empty and the player gets a notification. New colonists can be assigned a preset immediately.

Presets are a UI convenience, not a game mechanic. The colonist doesn't know about presets — they just have whatever's on their belt. The player can always customize individual slots.

---

## Progression Summary

| Day | Milestone | Equipment State |
|-----|-----------|----------------|
| 1 | Crash landing | Pockets only. Crash rations, matches. No tools. |
| 2–5 | First crafting | Stone axe, stone pick, wooden shovel. Held in hand, no belt yet. |
| 5–10 | Fiber belt | 3-slot belt. Axe + pick + knife. First time a colonist has multiple tools. |
| 10–15 | Leather belt | 4-slot belt from ridgeback leather. First pouch (bullet or seed). |
| 15–25 | Diversification | Multiple pouch types. Different colonists have different loadouts. Roles emerge. |
| 20–30 | Vest | Utility pockets. Medical supplies on person. Colonists survive injuries in the field. |
| 30+ | Pack | Expeditions beyond base. Multi-day trips. Trade runs. Hauling efficiency. |
| 30+ | Bandolier | Combat specialists diverge from general workers. Build identity. |
| 40+ | Reinforced belt | 6-slot belt. Specialists carry everything they need. |
| 50+ | Special slot | Rare items define late-game character identity. The gunslinger's cross-draw rig. |

---

## Tiny Item Catalog

Categories of tiny items that justify the pouch system's existence. This is the RPG depth layer — hundreds of distinct small items that accumulate naturally through gameplay.

### Ammunition
- Pistol rounds (9mm), rifle rounds, shotgun shells, arrow heads
- Tracer rounds (visible in raytrace, reveals shooter position)
- Incendiary rounds (chance to ignite flammable targets — fire system)
- Slug rounds (shotgun, single target, higher damage)

### Currency
- Copper bits (common, low value — found in ruins, dropped by raiders)
- Silver slugs (uncommon — trade caravans, prospecting)
- Gold pieces (rare — deep ruins, valuable trade)
- Circuit scrap (valuable salvage — wreck sites, ancient infrastructure)
- Stamped tokens (faction-specific currency from different settlements)

### Seeds
- Dustroot, bitterbulb, sweetmoss, char-cap spores, sap-vine cuttings
- Bloodgrass seeds (only obtained after burning wild bloodgrass)
- Unknown seeds (found in ruins — plant to discover what grows)

### Medicinal Herbs
- Painroot (reduces pain, allows working through injuries)
- Fever leaf (treats infection, speeds recovery)
- Wound moss (applied directly to cuts, stops bleeding faster)
- Purgebark (induces vomiting — treats food poisoning, bitterbulb toxin)
- Numbleaf (local anesthetic — required for field surgery)
- Dried sweetmoss (general nutrition supplement for the sick)
- Anti-venom extract (processed from glintcrawler glands — treats stings)

### Mechanical Parts
- Iron nails (construction speed bonus, required for some blueprints)
- Copper rivets (pipe repairs, advanced construction)
- Wire scraps (electrical repairs, trap building)
- Springs (mechanical devices, clockwork, traps)
- Gears (windmill repair, advanced machinery)
- Screws (furniture quality bonus)
- Pins and cotters (axle repairs, wagon maintenance, future)

### Fire and Light
- Matches (limited supply from crash, later craftable with sulfur + wood)
- Flint (infinite fire starter, slower than matches — the permanent upgrade)
- Tinder bundle (dry fiber, speeds fire lighting)
- Tallow candle (portable light source, 30-minute burn, craftable)
- Oil flask (lamp fuel, fire accelerant — dual use per the-human-layer.md)

### Writing and Knowledge
- Charcoal stick (writing implement — colonists write letters, maps, notes)
- Paper scraps (salvaged from wreck — limited supply until papermaking)
- Ink vial (crafted from soot + water — higher quality writing)
- Map fragments (found in ruins — reveal terrain when assembled)
- Cipher key (decodes encrypted radio transmissions)
- Blueprint scraps (partial schematics — combine 3 to get a full blueprint card)

### Geological Specimens
- Ore samples (identify mineral deposits — prospector carries these)
- Quartz shards (trade value, lens crafting material)
- Gemstones (high trade value, beauty items for rooms)
- Fossil fragments (lore items — what lived here before?)
- Unknown crystals (ancient infrastructure material — the-human-layer.md ghost infrastructure)
- Soil samples (test fertility before farming — agronomist use)

### Cooking and Spice
- Salt (preservative — extends food shelf life, found in mineral deposits)
- Dried spice (from specific alien plants — cooking quality bonus)
- Yeast culture (fermentation starter — kept alive in warm pouch)
- Rendered fat (cooking ingredient, lamp fuel, waterproofing — triple use)
- Bone meal (fertilizer for crops, ground at workbench from butchered bones)

### Sewing and Repair
- Needle and thread (clothing/vest/belt repair — extends equipment durability)
- Leather scraps (patch material for worn equipment)
- Sinew cord (from butchered creatures — strong natural thread)
- Bone buttons (crafted decoration — clothing/vest quality bonus)
- Dye pigments (coloring for clothing — cosmetic, minor mood buff from personalization)

### Keys and Access
- Keys (unlock specific locked containers, doors, or ancient mechanisms)
- Lock picks (alternative to keys — skill check to open, consumed on failure)
- Access tokens (ancient infrastructure keycards — found in deep ruins)
- Wax seal (authenticates trade documents — reputation system connection)

The total tiny item count across all categories: ~60+ distinct types. Each one occupies a 1×1 or 2×1 tiny cell in a pouch. The player never manages these individually — they accumulate through gameplay (harvesting herbs, mining ore, butchering creatures, looting ruins, trading) and are consumed automatically. The richness is visible when you open a colonist's pouches, but invisible during normal play.

---

## Equipment Durability

Tools and wearables degrade with use:

```
durability: f32  // 1.0 = new, 0.0 = broken

Degradation per use:
  Stone axe:    -0.005 per chop (200 chops to break)
  Hunting knife: -0.003 per butcher/harvest (333 uses)
  Pistol:       -0.002 per shot (500 shots)
  Fiber belt:   -0.001 per day worn (1000 days — rarely breaks)
  Leather vest: -0.0005 per day (2000 days)
  Pack:         -0.002 per expedition day (500 expedition-days)
```

**Broken items** are non-functional but not destroyed. A broken axe can't chop but still occupies a belt slot. The colonist auto-swaps to the next available tool (if any) and a notification appears: "Kai's stone axe broke."

**Repair** at a workbench consumes materials (repair kit from parts pouch, or raw materials). A colonist with needle + thread in their sewing pouch can field-repair leather equipment without returning to base. The parts pouch enables field repair of tools. Self-sufficiency scales with what you carry.

**Quality tiers** (future): items crafted by higher-skill colonists have higher max durability and better stats. A masterwork hunting knife from a skilled craftsman lasts 3× longer and butchers faster. Connects to deeper-systems.md knowledge system — the craftsman's skill lives in the item they made.

---

## Loot and Drops

Dead enemies and creatures drop equipment and tiny items. This is a primary source of new gear:

**Raiders drop:**
- Their weapon (damaged, 30–70% durability)
- Belt contents (often a bullet pouch with remaining ammo, sometimes a coin purse)
- Vest contents if wearing one (bandages, misc)
- Occasionally: a better-quality item than what you can craft (incentive to fight)

**Creatures drop:**
- Hide/leather (ridgeback — crafting material for belts, vests, pouches)
- Bones (crafting material, bone meal)
- Meat (food system — food-and-survival.md)
- Thermogast plates (rare — armor crafting material)
- Glintcrawler venom glands (medicine crafting, poison weapon coating)

**Ruins yield:**
- Blueprint scraps (combine 3 → full blueprint card)
- Ancient keys / access tokens
- Unknown seeds
- Salvage components (circuit scrap, wire, unknown crystals)
- Occasionally: pre-catastrophe equipment (better quality than frontier-crafted)
- Map fragments
- Letters from previous inhabitants (the-human-layer.md)

**Trade caravans offer:**
- Items from other biomes (you can't get everything locally)
- Specialty tools (better than stone-tier)
- Exotic seeds
- Ammunition (if you can't manufacture it yet)
- Rare pouches (larger, better quality)
- Information (maps, cipher keys, rumors — traded for coins)

---

## Connection to Other Systems

| System | Equipment Connection |
|--------|---------------------|
| **Crafting** (crafting.md) | Belts, vests, packs, pouches, tools, weapons are all crafted items. The crafting chain produces equipment progression. |
| **Combat** (DN-011) | Weapon swap time, ammo from pouches, bandolier reload bonus, armor from vest. Equipment IS combat readiness. |
| **Wounds** (DN-012) | Bandages auto-applied from vest. Herb satchel used by doc. Splints, suture kits. Medical equipment saves lives. |
| **Sprites** (DN-009) | Equipment layers render on colonist. Belt, holster, vest, pack visible. Active item in hand. |
| **Food** (food-and-survival.md) | Knife required for butchering. Seeds from seed bag for planting. Spices from spice pouch for cooking quality. |
| **Alien fauna** (alien-fauna.md) | Hunting requires ranged weapon + ammo. Creature drops supply leather for equipment crafting. Scent from butchering (emergent-physics.md) attracts predators. |
| **Knowledge** (deeper-systems.md) | Craftsman skill determines item quality. Teaching transfers crafting knowledge. Blueprint cards unlock new equipment recipes. |
| **Trade** (gameplay-systems.md) | Coin purse enables trade. Equipment is tradeable. Caravans supply items you can't craft. Export surplus equipment. |
| **Exploration** (multi-level.md) | Pack required for multi-day expeditions. Compass improves navigation. Specimen bag collects geological samples. Bedroll enables overnight camps. |
| **Sound sim** (sound.wgsl) | Pack makes colonist louder (movement sound amplitude). Bandolier clinks (minor sound). Equipment swap has sound. Coin purse jingles near enemies = detected. |
| **The human layer** (the-human-layer.md) | Equipment is character identity. The hunter's loadout vs. the farmer's loadout. Letters reference specific items ("Kai's old knife"). Scrap economy drives equipment progression. |

---

## Implementation Plan

### Phase 1: Belt System (Minimal — replaces current weapon slot)

Replace `equipped_weapon: Option<u16>` with `belt: [Option<u16>; 4]` and `active_belt_slot: u8`. Add auto-selection logic in simulation.rs based on `PlebActivity`. Extend GpuPleb with belt item fields for rendering.

**Files:** pleb.rs, simulation.rs, gpu_init.rs, raytrace.wgsl
**Scope:** Small. The belt is the current weapon system expanded to 4 slots with auto-switching.
**Blocks:** Nothing. Can ship independently.

### Phase 2: Item Size Properties

Add `grid_w`, `grid_h`, `fits_belt`, `fits_vest`, `fits_pack`, `tool_type` fields to items.toml. Update ItemRegistry parsing in item_defs.rs. Add `is_tiny`, `tiny_grid_w`, `tiny_grid_h`, `weight_grams` for tiny items.

**Files:** items.toml, item_defs.rs
**Scope:** Data-only. No gameplay change until Phase 3.
**Blocks:** Phase 3.

### Phase 3: Pouches and Tiny Items

Implement `PouchContents` struct, tiny sub-grid, weight-based stacking, auto-refill from nearby crates. Add tiny item definitions to items.toml (bullets, coins, seeds, herbs). Add pouch item definitions.

**Files:** resources.rs (new PouchContents), pleb.rs (PlebEquipment), simulation.rs (auto-refill tick), items.toml
**Scope:** Medium. The pouch is a new data structure but reuses existing stockpile/crate interaction patterns.
**Blocks:** Nothing critical. Can ship without vest/pack.

### Phase 4: Vest and Pack

Implement `TorsoEquip` (vest vs bandolier choice), `EquipGrid` for 2D grids, pack with movement penalty. Craft recipes for leather vest, bandolier, pack.

**Files:** pleb.rs, simulation.rs (movement speed modifier), recipes.toml, items.toml
**Scope:** Medium. The grid data structure is the main new code.
**Blocks:** Phase 2 (needs item size properties).

### Phase 5: Equipment UI

Colonist panel equipment tab with layered grid visualization. Pouch expand/collapse. Loadout presets. Drag-and-drop arrangement. Item tooltips.

**Files:** ui.rs
**Scope:** Large (UI is always large). But the underlying system works without the UI — auto-deploy doesn't need a panel.
**Blocks:** Everything else (needs all layers implemented to display).

### Phase 6: Sprite Rendering

Equipment layers visible on colonist sprites. Belt items at hip, holstered weapons, vest texture, pack on back. Active item in hand (extends existing weapon_type rendering).

**Files:** raytrace.wgsl, gpu_init.rs, pleb.rs (GpuPleb extensions)
**Scope:** Medium. Extends existing pleb rendering, doesn't replace it.
**Blocks:** DN-009 (sprite system).

---

## Open Questions

1. **Belt as 1D vs 2D?** The current design uses a 1×N strip for the belt. An alternative: 2×3 grid (like a small Diablo panel). 1D is simpler and more thematic (a belt IS a strip) but limits item shapes. 2×3 allows 1×2 items (shovel, torch) on the belt directly. Recommendation: start 1D, upgrade to 2D only if players want it.

2. **Pouch weight budget vs. cell count?** Current design: pouches have both a grid (limited cells) and a weight budget (limited grams per cell). Alternative: weight-only (no tiny grid, just a weight pool with a sorted list). The grid is more visually satisfying (Diablo factor) but adds complexity. Recommendation: grid for the UI, weight for the simulation. The grid is a visualization of the weight budget.

3. **How many pouch types is too many?** Seven types are listed. Some overlap (parts pouch and mechanic's kit). Consolidate or keep distinct? Recommendation: start with 4 core types (bullet, seed, herb, coin) and add specialist types as backstories demand them.

4. **Vest vs. bandolier — is exclusivity right?** Both take the torso slot, forcing a choice. Alternative: stack both (vest under bandolier). Recommendation: keep exclusive. The choice creates character identity. A combat specialist looks and plays differently from a utility generalist.

5. **Should the pack slow movement in all cases?** Current: pack always slows. Alternative: empty pack = no penalty, weight-proportional penalty. More realistic but harder to communicate. Recommendation: weight-proportional. An empty pack on your back is barely noticeable. A full pack of ore is heavy.

6. **Auto-equip pack for hauling — should this be default?** If a colonist is assigned hauling work and a pack is in storage, should they auto-grab it? Risk: the player doesn't realize their guard is now slow because they grabbed a pack to haul one item. Recommendation: auto-equip only if the colonist's work priority has hauling set to 1 (primary).

---

## Summary

The equipment system transforms colonists from interchangeable workers into distinct characters with visible, meaningful loadouts. The layered body model (pockets → belt → vest/bandolier → pack → special) provides natural progression. Pouches bridge the gap between "RPG-rich item variety" and "no micromanagement" by nesting hundreds of tiny items inside auto-refilling sub-containers. The Diablo-style grid gives every item physical shape and every container a spatial puzzle — but the auto-deploy system means the player thinks about loadouts, not individual item placement.

The system extends naturally: new item types are data entries in items.toml. New pouch types are items with `is_pouch = true`. New equipment layers are future additions that slot into the existing `PlebEquipment` struct. The dual-use principle (the-human-layer.md) means every new tool is also a weapon, and every new container enables new gameplay without new mechanics.

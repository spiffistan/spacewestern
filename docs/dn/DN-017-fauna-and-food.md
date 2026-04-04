# DN-017: Fauna, Hunting, and the Early Food Loop

## Problem

Food pressure is currently nonexistent. Berry bushes are infinite, farming is the only food system, and there's no reason food should feel urgent. A colony can park near berry bushes and never worry. The first 3-5 days need a tight resource curve that forces interesting decisions.

## Design Goal

Make food the central tension of the early game. The player should juggle short-term foraging, medium-term hunting/trapping, and long-term farming. No single strategy should be optimal. Each day should present a new challenge or capability.

## Day-by-Day Progression

| Day | Food Situation | New Capability |
|-----|---------------|----------------|
| 1 | Forage berries (finite per bush). Go to bed a little hungry. | Shelter, campfire |
| 2 | Local bushes depleted. Spot dusthares on the plains. | Snare crafting, first hunts |
| 3 | First snare catches overnight. Cooking campfire meals. | Trapping income, fishing |
| 4-5 | Traps + fishing + hunting stabilize food. First crop sprouts. | Farming begins to pay off |

## Fauna System

### Design Principles

Wildlife is part of the ecosystem, not just a combat encounter. Fauna are passive creatures that exist on the map from worldgen. They wander, graze, flee from threats, and reproduce slowly. Killing them yields food resources.

### Creature Types

| Species | ID | Health | Speed | Damage | Behavior | Habitat | Drops |
|---------|----|--------|-------|--------|----------|---------|-------|
| **Dusthare** | 2 | 8 | 4.5 | 0 | Wanders, hops, flees when approached | Open ground, common | 1 raw meat |
| **Mudcrawler** | 3 | 3 | 0.5 | 0 | Slow, near water | Shorelines, ponds | 1 raw shellfish |
| **Thornback** | 4 | 25 | 2.5 | 8 | Wanders, fights if cornered | Rocky terrain, sparse | 2-3 raw meat |

### Dusthare Behavior

Dusthares are small, skittish lizard-rabbits. They're the primary early-game food animal.

**State machine:**
- **Browse**: Wander randomly in short hops (2-4 tiles). Pause 1-3s between hops. Graze animation.
- **Alert**: When a pleb comes within ~8 tiles, freeze briefly (0.5s), then transition to Scatter.
- **Scatter**: Burst of speed (1.5x) away from the threat for 4-6 tiles, then resume Browse. Scatter direction is roughly away from the threat with some randomness.
- **Flee**: If damaged or very close threat (<3 tiles), full sprint to 15+ tiles away.

**Key traits:**
- Non-aggressive (damage = 0)
- Flee from plebs, not from other fauna
- Scatter in random directions (not all the same way) — a group of dusthares near each other scatter like a flock
- Can be killed by ranged weapons (pistol) or melee (chase down, hard due to speed)
- Snares are the intended early-game catch method

**Population:**
- 8-15 spawned at worldgen, scattered across open terrain (not in forests or water)
- Reproduce: every ~120 game-seconds, if population < 20 and at least 2 alive, spawn 1 new dusthare near an existing one
- No nocturnal despawn — they're always present

### Mudcrawler Behavior (Future)

Slow crustaceans near water. Can be picked up by hand (gather activity, like harvesting berries). Trivial food source for water-adjacent colonies.

### Thornback Behavior (Future)

Medium predator-lizard. Wanders peacefully but fights back when attacked (8 damage per hit). Yields 2-3 meat. Risk/reward tradeoff vs dusthares.

## Food Items

| Item | ID | Nutrition | Sickness | Source | Stack |
|------|-----|-----------|----------|--------|-------|
| Berries | 0 | 0.15 (nerf from 0.20) | None | Bush foraging | 10 |
| Raw Meat | 40 | 0.12 | 15% nausea | Hunting, trapping | 5 |
| Cooked Meat | 41 | 0.35 | None | Cook raw meat at campfire | 5 |
| Raw Shellfish | 42 | 0.10 | None | Mudcrawler gathering | 5 |
| Cooked Shellfish | 43 | 0.25 | None | Cook at campfire | 5 |
| Raw Fish | 44 | 0.10 | 10% nausea | Fishing | 5 |
| Cooked Fish | 45 | 0.30 | None | Cook at campfire | 5 |

### Sickness mechanic

When eating raw meat with a sickness chance, if the roll fails: pleb vomits (the nutrition is wasted), mood drops by 10, and a "Nauseous" status tag appears for ~30s. Not dangerous, just wasteful — creates incentive to cook.

## Cooking

Cooking is a work task, not a separate system. When raw meat (or fish/shellfish) exists in pleb inventory or on the ground near a lit campfire/fireplace:

1. Pleb picks up raw food
2. Walks to nearest lit campfire
3. Cooking activity: 5 seconds
4. Raw item consumed, cooked item produced in inventory

Campfires/fireplaces need to be lit (fuel level > 0) to cook. This connects food preparation to the existing fire/fuel system.

## Trapping (Phase 2)

### Snare

- **Craft**: 3 sticks + 1 fiber, by hand, 4 seconds
- **Place**: Like a building (1x1 tile, walkable)
- **Mechanic**: Every ~60 game-seconds, if any dusthare is within 15 tiles, 25% chance to catch it. Caught dusthare is removed from the world, 1 raw meat deposited at snare location.
- **Durability**: Snare wears out after 5-6 catches (visual: broken snare block, needs rebuild)
- **Work task**: Plebs check snares as a farm-priority task — collect meat, snare auto-resets

### Bait (Future)

Placing berries on a snare doubles catch chance. Creates a use for berries beyond eating.

## Fishing (Phase 3)

- **Craft**: Fishing Line — 2 fiber + 1 stick, by hand
- **Activity**: Pleb stands at water's edge (adjacent to tile with water depth > 0.3), 20s per attempt, 35% catch rate
- **Yield**: 1 raw fish per catch
- **Assignment**: Zone-based (place a "Fishing Spot" zone at water's edge) or manual command
- **Peaceful and safe** — good for low-skill plebs or nighttime (near camp lights)

## Berry Bush Changes

- Each bush: 3-5 berries, then **visually empty** (bare branches, different sprite/color)
- Regrow: 2-3 full day cycles (120-180 game-seconds at 1x speed)
- Berry nutrition nerfed: 0.20 -> 0.15
- This makes berries a stopgap, not a solution

## Cooking Combinations (Future — "Zelda-style")

A **cooking pot** (craftable, placed on campfire) accepts 2-3 ingredients and produces a named dish. Discovery-based: first time you combine ingredients, you learn the recipe.

| Ingredients | Result | Nutrition | Bonus Effect |
|-------------|--------|-----------|--------------|
| Meat + berries | Berry Glaze | 0.45 | +mood |
| Fish + herbs | Herb Fish | 0.40 | +rest recovery |
| Meat + root + salt | Salted Stew | 0.50 | +warmth |
| Shellfish + herbs | Spiced Crawl | 0.35 | +stress relief |
| 2x meat + root | Hearty Roast | 0.55 | +health regen |

Wild herbs and roots would be new forageable items found on specific terrain types.

## Food Preservation (Future)

- **Salt**: Mineable resource near rocky terrain. Used to cure meat (extends shelf life).
- **Drying rack**: Sticks + rope, placed outdoors. Slow preservation (1-2 days).
- **Smoke shed**: Small enclosed room with active campfire. Faster preservation.
- Preserved food lasts 5-10x longer before spoilage (when spoilage system is added).

## Butchering

Dead creatures must be butchered before yielding meat. No auto-drop on death.

1. Kill creature (ranged/melee) — corpse remains for 60 seconds
2. Right-click corpse → "Butcher [creature] ([pleb])"
3. Pleb walks to corpse, Butchering activity (~4 seconds with progress bar)
4. On completion: corpse removed, raw meat dropped at pleb's feet

This adds a step between killing and eating that creates decisions: butcher now (safe?) or leave it (might despawn)?

## Hunting Command

Right-click a living creature with a pleb selected → "Hunt [creature]". The pleb:
1. Paths toward the creature
2. When within 12 tiles, aims with precision (uses `aim_pos`)
3. If target moves out of range, follows
4. Cancellable via Stop button [S] or Escape

## Equipment Protection

Equipped items (weapons, tools) are protected from hauling/storage. `equipped_weapon` persists even when not drafted. Hauling code skips items matching `pleb.is_equipped(item_id)`. Right-click ground weapons to Equip — old weapon is swapped to the ground.

## Implementation Status

### Done
- [x] Dusthare fauna — creature def, Browse/Scatter AI, worldgen spawn, reproduction
- [x] Raw/cooked meat items — item defs, constants
- [x] Butchering system — Butcher context action, activity, corpse→meat conversion
- [x] Hunting command — Hunt context action, follow+aim behavior
- [x] Equipment system — equip/unequip, haul protection, inventory indicators
- [x] Need emotes — thought bubbles at hunger/thirst/rest/warmth thresholds
- [x] Status labels — priority-based with colored pill backgrounds
- [x] Per-pleb event log — timestamped, shown in character sheet Log tab
- [x] Modifiers tab — Rimworld-style derived buffs/debuffs on character sheet

### Pending
- [ ] Cooking work task — carry raw meat to campfire, produce cooked meat
- [ ] Berry bush finite supply — deplete on harvest, visual empty state, slow regrow
- [ ] Snare trapping — craft recipe, placement, catch mechanic
- [ ] Fishing — activity, zone, water proximity check
- [ ] Stalking mechanics — crouch reduces detection, sound/light awareness
- [ ] Cooking combinations — cooking pot, recipe discovery
- [ ] Food preservation — salt, drying, smoking
- [ ] Mudcrawler creature
- [ ] Thornback creature

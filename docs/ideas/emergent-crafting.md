# Emergent Crafting and Per-Map Discovery

How crafting discovery varies between playthroughs. Not all recipes are universal — alien materials have properties that change per map seed, creating unique discovery signatures.

## Core Concept: Material Calibration

Each map seed generates hidden properties for alien materials. The same plant or mineral can have different uses on different maps.

### Example: Duskbloom Nectar

| Map Seed | Hidden Property | Useful Combination | Result |
|----------|----------------|-------------------|--------|
| Seed A | Adhesive | Nectar + fiber | Strong rope (2x durability) |
| Seed B | Antiseptic | Nectar + cloth | Poultice (healing) |
| Seed C | Bioluminescent | Nectar + resin | Glow lamp (no fuel needed) |
| Seed D | Mild toxin | Nectar + arrowhead | Poison arrows |

### Example: Thornbrake Thorns

| Map Seed | Hidden Property | Useful Combination | Result |
|----------|----------------|-------------------|--------|
| Seed A | Barbed | Thorns + stick | Barbed spear |
| Seed B | Hollow | Thorns + reed | Blowpipe needle |
| Seed C | Resinous | Thorns + fire | Aromatic smoke (mood boost) |
| Seed D | Brittle | No useful property | Just fuel |

Basic recipes (hammerstone, stone axe, cooking) are universal. Alien material recipes are calibrated per seed.

## How Discovery Happens

### 1. Idle Wondering
A pleb near unusual materials occasionally has an idea. Not every pleb, not every time — depends on curiosity trait, relevant skill, and whether they've recently handled the materials.

Thought bubble: *"This nectar is oddly sticky..."*

A few game-minutes later, if they're still idle: *"What if I coated rope with this?"*

The player can right-click the pleb → "Try it" to start an experiment. Or ignore it — the idea fades after a while.

### 2. Accidental Discovery
The physics simulation creates conditions:
- Food dropped near fire → discover smoking/toasting
- Rain on stretched hide → discover wet curing
- Lightning strikes near ore → notice color change (smelting clue)
- Pleb falls in water carrying certain materials → waterproofing discovery
- Fire near alien plants → aromatic/toxic smoke depending on species

These aren't scripted events. They emerge from the simulation state. If it never rains while a pleb has a hide out, wet curing is never discovered on that playthrough.

### 3. Skill-Gated Perception
Each pleb's backstory and skills filter what they notice:
- **Botanist**: notices plant reactions, growth patterns, edibility
- **Engineer**: sees structural properties, load-bearing potential, conductivity
- **Geologist**: identifies mineral types, ore indicators, stone quality
- **Medic**: recognizes antiseptic, analgesic, toxic properties
- **Cook**: notices flavor, preservation potential, fermentation

Same map, different crew = different discoveries. A team with no botanist might never discover that saltbrush extract preserves leather.

### 4. Failure Is Content
Most experiments produce nothing useful. Some produce memorable results:

| Outcome | Frequency | Example |
|---------|-----------|---------|
| Nothing | 60% | "Mixed them together. Nothing happened." |
| Interesting but useless | 20% | "It fizzed and turned purple. Stained my hands." |
| Useful discovery | 15% | "It hardened into a waterproof coating!" |
| Dangerous failure | 5% | "It caught fire. Singed my eyebrows." |

Failed experiments are logged in the pleb's journal and never suggested again. The journal of failures is itself interesting: *"Day 12: Tried mixing nectar with clay. Sticky mess. Don't do that. Day 15: Tried nectar with fiber. It worked — much stronger binding!"*

### 5. Cross-Pollination Through Conversation
Per DN-019's social knowledge transfer:
- Ada discovers adhesive nectar
- Ada tells Marcus about it (conversation event)
- Marcus, who has construction skill, realizes a second-order use: *"If that's adhesive, we could use it instead of rope for binding tool handles"*
- New recipe discovered: nectar-bound tools (better durability)

Second-order discoveries require TWO plebs with complementary knowledge. Solo survivors miss these entirely. Larger colonies discover more.

## Implementation Architecture

### Material Properties Table
Generated at worldgen from map seed. ~20 alien materials × 4-6 possible properties each. Per seed, each material gets 1-2 active properties from the pool.

```
MaterialProperty {
    material_id: u16,      // which alien material
    property: PropertyKind, // adhesive, antiseptic, etc.
    strength: f32,         // 0.0-1.0 how strong the effect is
    discovery_hint: String, // "oddly sticky", "sharp medicinal smell"
}
```

### Discovery Recipes
A separate table from normal recipes. Each entry:
```
DiscoveryRecipe {
    inputs: [(material, count)],
    required_property: PropertyKind,
    output: ItemId,
    skill_needed: SkillType,
    min_skill_level: f32,
    discovery_text: String,
}
```

The recipe only works if the map's material calibration gives the input material the required property. On maps where duskbloom isn't adhesive, the "strong rope" recipe simply doesn't exist.

### Experiment Activity
New PlebActivity::Experimenting(recipe_attempt, progress)
- Takes 10-20 seconds game time
- Pleb stands at work surface, handles materials
- On completion: roll success/failure based on skill + property strength
- Success: recipe added to colony knowledge, notification event
- Failure: materials consumed, journal entry logged

## Replayability Impact

- Each map has a unique "discovery signature" — 10-15 useful alien recipes out of 50+ possible
- Players share discoveries: "On seed 42, duskbloom + salt makes an explosive"
- Crew composition matters: a botanist-heavy crew discovers different things than an engineer-heavy crew
- The ledger becomes a personal record of YOUR unique discoveries on YOUR alien world
- No two playthroughs have the same crafting options in the mid-to-late game

## What This Doesn't Change

- Basic survival recipes are universal and known from the start
- The primitive tool chain (rock → blade → axe) is always the same
- Cooking, drying, salting — universal food preservation
- Building recipes — universal construction

The alien discovery layer sits ON TOP of the reliable base game. You can always survive. The unique discoveries make each run special, not required.

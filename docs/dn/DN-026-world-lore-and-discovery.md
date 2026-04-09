# DN-026: World Lore and Discovery

**Status:** Draft
**Depends on:** DN-019 (knowledge/crafting), DN-024 (terrain/wilderness), DN-023 (mining)
**Related:** DN-010 (discovery layer), DN-020 (traits/aptitudes)

## Problem

The player currently sees the full world from the start — every plant is labeled, every rock is identified, creature behavior is transparent. There's no sense of being on an alien world where things are unknown. The map is data, not discovery.

We want the world to reveal itself gradually. The colonists arrive ignorant of this planet. Everything is unfamiliar. Through observation, interaction, and occasional disaster, they build understanding. The same map that felt hostile and opaque on day 1 feels readable and navigable on day 30 — not because it changed, but because the colony's comprehension grew.

## Core Principle: The World Doesn't Change, Your Understanding Does

A dustwhisker patch on day 1 is "unknown plant (fibrous stalks)." On day 5, after someone harvested it, it's "Dustwhisker (fiber)." On day 15, after a grass fire, it's "Dustwhisker (fiber, highly flammable — keep clear of buildings)." The plant was always the same. The colonists learned.

This applies to everything: rocks, creatures, weather, terrain, even the buildings you construct. Understanding accumulates, displayed through UI labels, thought bubbles, journal entries, and eventually through the shader itself (veins visible to trained eyes).

## Lore Stages

Knowledge about the world is organized into **lore stages** — concentric rings of understanding that roughly correspond to how long the colony has existed and how actively it has explored.

### Stage 1: Survival (Day 0-5)

What colonists arrive knowing. Basic universal knowledge that any frontier settler would have.

**Known:**
- Fire: how to build and maintain campfires
- Shelter: lean-to, basic wall construction
- Water: find surface water, dig for groundwater
- Food: cooking meat is safer than raw, berries are testable
- Danger: darkness is dangerous, stay near light at night
- Tools: stone knapping (crude), stick binding

**Unknown (everything else):**
- Plant species: "Unknown plant" with visual descriptor ("pale stalks," "spiny shrub")
- Rock types: "Rock" — no distinction between chalk and granite
- Creatures: "Unknown creature" with size/behavior cues ("small, hops away," "large, stalking")
- Weather: no prediction, just observation
- Terrain: no ability to read surface signs for underground resources

**How it surfaces:**
- Hover info shows "Unknown plant (fibrous, pale amber stalks)"
- Minimap labels are absent for natural features
- Crafting menu only shows Tier 0-1 recipes
- No weather forecast
- Rock in the mining UI is just "Rock" with unknown hardness

### Stage 2: Adaptation (Day 5-20)

Colonists begin identifying the world around them through direct interaction. Each first encounter triggers a discovery.

**Discovery triggers:**
- **First harvest**: "Ada harvested an unknown plant → Dustwhisker discovered (yields fiber)" — entry added to lore journal
- **First mining**: "Ben mined rock → Sandstone identified (soft, layers visible)" — rock type revealed
- **First creature encounter**: "Clara saw an unknown creature → Dusthare observed (passive, hops, flees from people)"
- **First weather event**: "First rain → Rain pattern observed (clouds build from west, ~2 minute warning)"
- **First illness**: "Dave ate unknown berries → Toxic reaction! Nightshade berry identified (DO NOT EAT)"
- **First night attack**: "Duskweaver attacked → Duskweaver documented (nocturnal, avoids light, hunts exposed colonists)"

**What changes when something is discovered:**
- Hover labels switch from "Unknown" to the species name
- The lore journal gets an entry with observations
- Related recipes may unlock (dustwhisker → fiber rope recipe becomes visible)
- Minimap can show icons for identified resources
- Plebs reference discoveries in thought bubbles and logs

**Knowledge is per-pleb initially:**
- Ada knows what dustwhisker is. Ben doesn't (he was mining).
- When Ada and Ben share a meal, knowledge transfers: "Ada told Ben about dustwhisker."
- Transfer is passive and time-based: proximity + shared activity = knowledge spread
- If Ada dies before telling anyone, the colony "forgets" — dustwhisker goes back to "unknown plant" until someone else harvests it

### Stage 3: Comprehension (Day 20-60)

The colony understands the basics and begins seeing deeper patterns. This is where expertise develops.

**Geology comprehension:**
- Experienced miners start identifying rock types from surface color (without mining)
- "This reddish stone might contain iron" — vein prediction before exposure
- Understanding which rock types appear together (flint in chalk, copper in basalt)
- Geological zone awareness: "The stone changes character to the northwest"

**Ecological comprehension:**
- Dusthare migration patterns between glades (predictable hunting)
- Duskweaver denning behavior (they return to the same glade)
- Plant growth cycles (dustwhisker regrows, saltbrush doesn't)
- Which plants grow near water (reeds = underground water indicator)
- Fire ecology: grass fires clear land but destroy forage

**Climate comprehension:**
- Rain frequency and seasonal variation
- Temperature patterns (cold nights, warm days, how buildings buffer)
- Drought prediction ("It hasn't rained in 8 days — wells may lower")
- Wind patterns (smoke direction, fire spread prediction)

**How it surfaces:**
- Geology skill reveals vein hints in the shader (subtle color on unmined rock)
- Weather forecast appears in UI for experienced colonists
- Creature behavior predictions in thought bubbles: "Dusthares will be at Quiet Basin at dusk"
- Plant regrowth timers visible on hover
- Terrain reading: "This wet patch suggests a shallow aquifer"

### Stage 4: Mastery (Day 60+)

Deep knowledge that transforms the colony's relationship with the world. Not everyone reaches this — specific experts in specific domains.

**Geological mastery:**
- Full mineral awareness (veins visible through rock surface in shader)
- Structural assessment (which rock will hold, which will collapse)
- Underground prediction (surface features indicate cave systems)
- Rare material identification (hearthstone, resonite characteristics)

**Ecological mastery:**
- Creature domestication potential (dusthare pens? Future system)
- Medicinal plant combinations (multiple plants → specific remedies)
- Deliberate ecosystem management (controlled burns, strategic planting)
- Duskweaver territory mapping and avoidance routing

**Climate mastery:**
- Multi-day weather forecasting
- Optimal building orientation for climate
- Drought/flood mitigation strategies
- Using wind for power/ventilation intentionally

### Stage 5: Ancient Lore (Exploration-dependent, not time-dependent)

Found, not learned. Requires discovering specific locations and objects in the deep wilderness.

**Discovery sources:**
- Standing stones in glades (inscriptions, once deciphered)
- Buried objects (found while mining or digging)
- Cave markings (accessible through sinkholes or mined tunnels)
- Alien technology fragments (rare, specific glades)

**What ancient lore reveals:**
- The world's history: who was here before? Why did they leave?
- Alien construction techniques (stronger buildings, unusual materials)
- Hearthstone and resonite properties (how to use alien materials)
- Possible planetary hazards (why the previous inhabitants left?)
- Deep underground geography (cave network maps from inscriptions)

**How it works mechanically:**
- Each lore fragment is a unique collectible with flavor text
- Some fragments combine to reveal larger truths
- Practical unlocks are embedded in lore (a diagram teaches a construction technique)
- A pleb with Alien Tech knowledge (DN-019) deciphers fragments faster
- Some fragments are deliberately ambiguous — the player interprets

## The Lore Journal

A colony-wide document that grows with discoveries. Organized by domain:

```
THE JOURNAL
├── Flora (7 of ~15 discovered)
│   ├── Dustwhisker — fiber plant, open plains, regrows in ~8 days
│   ├── Hollow Reed — rigid tubes, near water, indicates groundwater
│   ├── Thornbrake — sticks + thorns, rocky terrain, duskweavers avoid
│   ├── Saltbrush — salt crystals, rare, does not regrow
│   ├── Duskbloom — blooms at dusk, nectar (night) or petals (day)
│   ├── Berry Bush — edible berries, forest edges, slow regrowth
│   └── ??? (8 more undiscovered entries shown as silhouettes)
├── Fauna (3 of ~5 discovered)
│   ├── Dusthare — passive herbivore, hops, flees, drops raw meat
│   ├── Duskweaver — nocturnal predator, avoids light, hunts exposed colonists
│   └── Hollowcall — invisible stalker, visible only in thermal, flees when shot
├── Geology (2 of ~8 discovered)
│   ├── Sandstone — soft, layered, easy to mine, erodes in rain
│   └── Chalk — very soft, contains flint nodules
├── Climate (1 of ~4 discovered)
│   └── Rain — builds from west, ~2 minute warning from cloud cover increase
└── Ancient (0 of ??? discovered)
    └── (no entries yet)
```

The "???" entries are important — they tell the player there IS more to find without revealing what. The count is approximate (the player doesn't know the exact total). This creates exploration motivation.

## How Labels Change in the World

The same tile, at different knowledge levels:

| Knowledge level | Hover label | Shader |
|----------------|-------------|--------|
| Unaware | "Unknown plant (pale amber stalks)" | Default procedural color |
| Aware | "Dustwhisker" | Unchanged |
| Familiar | "Dustwhisker (fiber — harvest by hand)" | Unchanged |
| Competent | "Dustwhisker (fiber, 2-3 per plant, regrows ~8 days)" | Unchanged |
| Expert | "Dustwhisker (fiber, flammable, dusthares browse here at dusk)" | Subtle shimmer on harvestable plants |

For rocks:

| Knowledge level | Hover label | Shader |
|----------------|-------------|--------|
| Unaware | "Rock" | Default grey |
| Aware | "Sandstone" | Warm tint |
| Familiar | "Sandstone (soft, mine with any tool)" | Visible strata |
| Competent | "Sandstone (may contain coal seams)" | Faint vein hints |
| Expert | "Sandstone (coal seam runs NE, iron unlikely)" | Clear vein overlay |

## Implementation Considerations

### Data Structure

```rust
// Per-pleb knowledge of the world (separate from crafting knowledge in DN-019)
struct WorldKnowledge {
    flora: HashMap<u32, KnowledgeLevel>,    // BT_* → level
    fauna: HashMap<u8, KnowledgeLevel>,     // creature species_id → level
    geology: HashMap<u8, KnowledgeLevel>,   // ROCK_* → level
    climate: HashSet<String>,               // discovered climate facts
    ancient: Vec<LoreFragment>,             // collected lore pieces
}

enum KnowledgeLevel {
    Unaware,
    Aware,     // knows name
    Familiar,  // knows basic use
    Competent, // knows details
    Expert,    // knows hidden properties
}
```

### Discovery Events

```rust
enum DiscoveryEvent {
    PlantIdentified { pleb: usize, block_type: u32 },
    RockIdentified { pleb: usize, rock_type: u8 },
    CreatureObserved { pleb: usize, species: u8 },
    WeatherPatternLearned { pattern: String },
    LoreFragmentFound { pleb: usize, fragment_id: u16 },
    PropertyDiscovered { pleb: usize, subject: String, property: String },
}
```

### Knowledge Transfer

Transfer happens during shared activities (not instant):
- Eating together at campfire: ~10% chance per meal to share one discovery
- Working adjacent: ~5% chance per game-hour to share relevant domain knowledge
- Explicit teaching (future): faster, directed, requires idle time

### UI Integration

- Hover labels: check selected pleb's knowledge → show appropriate label
- Lore journal: accessible from character sheet or dedicated button
- Discovery notifications: card-style event ("Ada discovered: Dustwhisker")
- Minimap icons: only show for identified resources
- Crafting menu: filter by colony's collective knowledge

## Relationship to DN-019

DN-019 covers **crafting knowledge** — can you make a flint blade? Can you smelt iron? That's about recipes and techniques.

This DN covers **world knowledge** — do you know what this plant is? Can you read this rock? Do you understand the weather? It's about comprehension of the environment.

They share the same 6-level gradient and social transfer mechanics. A pleb's "Geology" domain in DN-019 corresponds to their geology WorldKnowledge here. The systems reinforce each other: you can't learn "Stoneworking" (DN-019) until you've identified at least one rock type (DN-026).

## Design Principles

1. **Ignorance is the starting state.** The world is alien. Act like it.
2. **Discovery is the reward.** Not just resources — understanding IS the progression.
3. **Knowledge is fragile.** It lives in people. People die. Knowledge can be lost.
4. **The world has depth.** There's always more to learn. The "???" entries prove it.
5. **Understanding has practical value.** It's not just flavor — knowing what a plant is unlocks recipes, knowing a rock type reveals veins, knowing weather saves lives.

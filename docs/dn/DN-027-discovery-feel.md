# DN-027: How Discovery Should Feel

**Status:** Draft
**Depends on:** DN-026 (world lore), DN-019 (knowledge system)
**Related:** DN-010 (discovery layer), DN-024 (terrain/wilderness)

## Problem

Games handle discovery on a spectrum from intrusive to invisible. Intrusive: "!" markers, popup tutorials, quest logs pointing at every unknwon thing. Invisible: the player figures everything out by reading wiki pages because the game never signals anything.

We want the middle. Discovery should feel serendipitous — the world quietly reveals itself through normal play. The player's first encounter with a dustwhisker patch doesn't interrupt them. They don't even notice the moment it gets identified. They just realize at some point that they can read the landscape now.

But RARE discoveries — a crystal vein, an ancient inscription, a creature behavior pattern — should land with weight. A brief thought bubble, a journal entry worth reading, a moment of "what is this?"

The system should be invisible on the second playthrough. Everything common resolves in the first minute. Everything rare still takes days.

## Design Principles

1. **The world is complete from the start.** Discovery is the colonists catching up, not the world spawning content. The dustwhisker was always there. Ada just walked close enough to look at it.

2. **Routine discovery is silent.** No bubble, no sound, no notification popup. The hover label changes. The journal gets an entry. The player may or may not notice. This is fine.

3. **Rare discovery gets a moment.** A brief thought bubble ("What is this...?"), an event log entry, maybe a notification card for truly significant finds. These are rare — maybe 5-10 across an entire playthrough.

4. **Discovery requires the right conditions, not the right clicks.** No "Examine" action for routine identification. Plebs identify things by proximity while going about their business. Some discoveries require specific circumstances — time of day, skill level, patience, luck.

5. **Serendipity over system.** The player shouldn't be able to optimize discovery. There's no "send all plebs to explore" strategy that's faster than just playing naturally. The best discoveries happen when you weren't looking for them.

## The Two Tiers

### Tier 1: Passive Identification (most things)

**Mechanism:** Pleb walks within ~3 tiles of an unidentified thing → it gets identified for that pleb. Knowledge spreads to other plebs over time through proximity (meals, shared work).

**What it covers:**
- Common plant species (dustwhisker, holly reed, thornbrake, berry bush)
- Common rock types (sandstone, chalk)
- Surface terrain features
- Basic creature recognition (dusthare = "small hopping creature")

**UX:**
- No bubble on pleb
- No notification sound
- Hover label quietly changes from "Unknown plant (pale amber stalks)" to "Dustwhisker (fiber)"
- Journal entry added: "Ada identified a new plant: Dustwhisker"
- If the player is watching the pleb, they might see the label change. If not, they discover the discovery later

**On subsequent playthroughs:** All common things identified in the first 1-2 minutes as plebs walk around the landing zone. Zero friction.

### Tier 2: Conditional Discovery (rare/deep things)

**Mechanism:** Requires specific circumstances beyond proximity. The right person, the right place, the right time, or the right activity.

**What it covers and what triggers it:**

| Discovery | Trigger | Why it's not automatic |
|-----------|---------|----------------------|
| Duskbloom identity | Proximity at dusk/night (when blooming) | Looks like a grey bud during the day — nothing to notice |
| Mineral veins | Exposed by mining | Hidden inside rock, can't see from surface |
| Iron deposits in granite | Geologist (skill 4+) near granite | Untrained eye sees "rock." Trained eye sees red staining |
| Creature behavior patterns | Pleb idle near creature for ~30 seconds | You learn by watching, not by glancing |
| Duskweaver hunting routes | Survive 3+ night attacks | Pattern emerges from repeated observation |
| Weather prediction | Experience 3+ rain cycles | First rain is "rain." Third rain: "pattern identified" |
| Underground water signs | Geologist near reeds/wet ground | Connecting "reeds here" → "water below" requires knowledge |
| Ancient lore fragments | Find specific objects in glades/caves | Must physically discover the location |
| Hearthstone properties | Mining + geology skill 6+ | Warm stone just feels like "warm rock" to untrained hands |
| Creature weakness to light | Observe duskweaver fleeing torch | Must witness the behavior, not just know duskweavers exist |

**UX for Tier 2:**
- Brief thought bubble (1.5s, Thought priority — won't override important bubbles)
- Event log entry with flavor text
- Journal entry with more detail than Tier 1
- For truly significant finds (first crystal, first lore fragment): notification card in the left-edge system

**Examples of Tier 2 thought bubbles:**
- Mining reveals crystal: "These crystals... I've never seen anything like this."
- Geologist spots iron in granite: "This reddish staining — could be iron ore underneath."
- Watching dusthare at dusk: "They gather near those flowers when they bloom..."
- Finding ancient inscription: "Strange markings on the stone. They look deliberate."
- First duskweaver attack survived: "It fled from the torchlight. They fear fire."

These are CHARACTER thoughts, not game instructions. They tell you what the pleb noticed, in their voice. The player draws their own conclusions.

## What the Player NEVER Gets

- "?" markers floating over unidentified things
- "Press E to examine" prompts
- A "Discovery" tab with a progress bar
- Minimap icons pointing to undiscovered things
- Any notification for Tier 1 discoveries (unless checking the journal)
- Tutorial text explaining how discovery works
- A "discover all" cheat or fast-forward

The system is deliberately opaque. The player is supposed to play naturally and realize after a few game-days that they now know things they didn't before. The HOW is invisible.

## Skill-Gated Perception

Not all plebs notice the same things. A pleb's knowledge level (DN-026) and relevant skill (DN-022) affect what they can identify passively:

| Flora | Minimum requirement |
|-------|-------------------|
| Common plants | Anyone (proximity) |
| Medicinal properties | Foraging skill 3+ |
| Growth cycles/regrowth | Farming skill 4+ or repeated harvest |
| Fire behavior (flammability) | Experience a grass fire |

| Geology | Minimum requirement |
|---------|-------------------|
| Common rock types | Anyone (proximity + mining) |
| Mineral indicators on surface | Geology skill 4+ |
| Vein direction prediction | Geology skill 7+ |
| Structural assessment | Geology skill 6+ AND construction skill 3+ |

| Fauna | Minimum requirement |
|-------|-------------------|
| Species identification | Anyone (visual contact) |
| Behavior patterns | Idle observation, ~30 seconds |
| Weaknesses | Witness specific interaction (light, sound, thornbrake) |
| Migration/nesting | Extended observation over multiple days |

This means crew composition affects what you discover and when. A colony with a skilled geologist identifies rock types from day 1. A colony without one might mine for a week before someone happens to notice the sandstone has layers.

## The Journal as Discovery Record

The journal (DN-026) is the ONLY place the player can review what's been discovered. It's not a quest log or checklist — it's a field notebook.

**Presentation:**
- Organized by domain (Flora, Fauna, Geology, Climate, Ancient)
- Each entry has: name, one-line description, who discovered it, when
- Undiscovered entries shown as "??? — (X more species undiscovered)" with approximate counts
- No exact total revealed. The player never knows if they've found everything.
- Entries accumulate organically. The journal GROWS, never shrinks.

**Tone:** The entries should read like a naturalist's field notes, not a game wiki:

> **Dustwhisker** — Pale amber grass-like stalks, common in open terrain. Yields useful fiber when harvested by hand. Regrows within days. Highly flammable — keep clear of campfires. Ada noticed dusthares browsing near dustwhisker patches at dusk.

Not:

> Dustwhisker: Fiber ×2-3. Regrow time: 8 days. Flammability: HIGH. Spawns: plains biome.

The journal is a story, not a spreadsheet.

## Implementation Notes

### What Triggers Passive ID

```rust
// During pleb movement tick, check nearby tiles for unidentified block types
fn check_nearby_discoveries(pleb: &mut Pleb, grid: &[u32], radius: f32) {
    let bx = pleb.x.floor() as i32;
    let by = pleb.y.floor() as i32;
    let r = radius as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            let nx = bx + dx;
            let ny = by + dy;
            // ... bounds check ...
            let bt = block_type(grid[idx]);
            if is_discoverable(bt) && !pleb.world_knowledge.knows_flora(bt) {
                pleb.world_knowledge.identify_flora(bt);
                // Journal entry + event log (no bubble)
            }
        }
    }
}
```

Throttled: check every ~60 frames (once per game-second), not every frame.

### What Triggers Conditional Discovery

Embedded in existing activity handlers:
- Mining: check mined cell material → if mineral, trigger discovery for that mineral type
- Idle near creature: accumulate observation timer → threshold triggers behavior discovery
- Weather: count rain events → threshold triggers pattern discovery
- Time-gated: duskbloom proximity check includes time-of-day filter

### Knowledge Spread

Handled in the social tick (meals, proximity):
- Each shared meal: ~10% chance to transfer one Tier 1 discovery
- Working adjacent: ~5% per game-hour
- Explicit teaching: not needed for Tier 1, could be useful for Tier 2

### Label Display

The hover info system checks the SELECTED pleb's knowledge:
```
if selected_pleb.knows_flora(bt) {
    show "{species_name} ({uses})"
} else {
    show "Unknown plant ({visual_descriptor})"
}
```

If no pleb is selected, use the colony's collective knowledge (any pleb knows it → labeled).

## Relationship to Other Systems

- **DN-019 (crafting):** Tier 1 plant ID unlocks related recipes in the crafting menu. You can't see "fiber rope" recipe until someone has identified dustwhisker.
- **DN-023 (mining):** Mineral discoveries happen through mining. Geology skill gates surface-level identification.
- **DN-024 (terrain):** Glade discovery is a Tier 2 event. Finding a glade with a spring triggers a named-location discovery.
- **DN-025 (tools):** Flint discovery (finding it in chalk) is a Tier 2 event that unlocks the flint tool recipes.

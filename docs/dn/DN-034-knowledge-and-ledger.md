# DN-034: Knowledge System and Colony Ledger

**Status:** Draft
**Depends on:** DN-019 (knowledge gradients), DN-026 (world discovery), DN-027 (discovery feel)
**Related:** DN-033 (work surfaces), emergent-crafting.md

## Problem

The player needs to understand what the colony can do, what it might be able to do, and how to get there — without a predetermined tech tree that spoils discovery.

## Design: Three Knowledge States

Every piece of knowledge exists in one of three states per colony:

| State | Visual | Meaning | Craft menu |
|-------|--------|---------|------------|
| **Known** | Bright icon, full label | Colony understands this fully | Recipe visible, craftable |
| **Noticed** | Faded icon, vague text | A pleb saw something but doesn't understand it | Not in craft menu |
| **Unknown** | Not shown | Nobody has encountered this | Invisible |

**Unknown → Noticed**: Proximity, exploration, environmental encounter.
**Noticed → Known**: Examination by a skilled pleb, experimentation, being taught.

## Knowledge Domains

Knowledge is organized into domains, not a linear tree:

| Domain | Examples |
|--------|----------|
| Stone & Earth | Rock types, knapping, mining, geology |
| Wood & Fiber | Tree species, carving, rope-making, weaving |
| Fire & Heat | Fuel, charcoal, cooking, smelting, kilns |
| Food & Medicine | Edibility, preservation, cooking, healing |
| Building | Wall types, roofing, structural strength |
| Creatures | Species, behavior, danger, drops, taming |
| Alien Materials | Per-map calibrated properties (see emergent-crafting.md) |

Each pleb has a knowledge level (0.0-10.0) per domain, following DN-022's skill scale.

## Dependency Model

### Explicit Dependencies (shown as arrows between known nodes)
Once the player KNOWS about two related things, the connection between them is shown:

```
Rock ──→ Hammerstone ──→ Stone Blade
                     ├──→ Stone Axe ←── Stick + Fiber
                     └──→ Stone Pick ←── Stick + Fiber
```

These arrows represent "you need A to make B." They only appear between known items. A known item pointing toward a noticed item shows a dashed line: *"this leads somewhere you haven't figured out yet."*

### Implicit Dependencies (spatial proximity)
Related knowledge clusters near each other. The player sees "Stone Working" and "Fire & Heat" as separate clusters. When smelting is discovered, a connection appears between them.

### Social Dependencies (who can teach whom)
Some knowledge requires specific pleb skills to unlock:
- Noticed geology node + pleb with geology skill → can examine and identify
- Two plebs with complementary knowledge talking → cross-domain discovery

## Colony Ledger UI

### Layout
Single panel, opened from a book icon in the HUD. Two views:

**Colony View** — all knowledge across all plebs, merged. Shows the "best" knowledge level for each item (whichever pleb knows most). Organized by domain clusters. This is the strategic planning view.

**Pleb View** — one pleb's personal knowledge. Accessed from character sheet. Shows what THIS person knows, what they've noticed, what they could learn from others.

### Colony View Detail

```
┌─ Colony Ledger ──────────────────────────────────────────┐
│                                                          │
│  ┌─ Stone & Earth ──────┐  ┌─ Fire & Heat ──────────┐   │
│  │ ◉ Rock         ████  │  │ ◉ Campfire       ████  │   │
│  │ ◉ Knapping     ███░  │  │ ◉ Charcoal       ██░░  │   │
│  │ ◉ Stone Blade  ████  │  │ ◉ Cooking        ████  │   │
│  │ ◉ Stone Axe    ███░  │  │ ◌ "hot clay..."  ░░░░  │   │
│  │ ◌ "pale stone" ░░░░  │  │                        │   │
│  │           └─→ ?       │  └────────────────────────┘   │
│  └───────────────────────┘                               │
│  ┌─ Food ───────────────┐  ┌─ Building ─────────────┐   │
│  │ ◉ Berries      ████  │  │ ◉ Wattle Wall    ████  │   │
│  │ ◉ Cooking      ████  │  │ ◉ Rough Floor    ███░  │   │
│  │ ◉ Drying       ███░  │  │ ◌ "strong clay"  ░░░░  │   │
│  │ ◉ Salting      ██░░  │  │                        │   │
│  │ ◌ "bitter root" ░░░░ │  └────────────────────────┘   │
│  └───────────────────────┘                               │
│                                                          │
│  ◉ = Known  ◌ = Noticed  ████ = Mastery level           │
└──────────────────────────────────────────────────────────┘
```

The bars (████) show the colony's best knowledge level for that item. Multiple plebs might contribute — the bar represents the highest individual level.

Clicking a known item shows:
- Description (narrative text, not stats)
- Who knows it (pleb portraits with their individual level)
- What it enables (recipes, capabilities)
- Related items (arrows to dependencies)

Clicking a noticed item shows:
- The vague observation text
- Who noticed it
- What might help: *"Someone with geology knowledge could examine this"*

### Pleb View Detail

```
┌─ Ada's Knowledge ────────────────────────────────────────┐
│  Backstory: Prospector  │  Aptitude: Stone & Earth       │
│                                                          │
│  Stone & Earth  ████████░░  (8.2 — Expert)               │
│    ◉ Rock types, knapping, flint identification          │
│    ◉ Can teach: knapping, stone tool assembly            │
│                                                          │
│  Fire & Heat    ███░░░░░░░  (3.1 — Familiar)             │
│    ◉ Basic fire-making, charcoal                         │
│    ◌ Could learn: kiln operation (needs teaching)        │
│                                                          │
│  Food           ██░░░░░░░░  (2.0 — Aware)                │
│    ◉ Basic cooking                                       │
│    ◌ Could learn: preservation (needs botanist)          │
│                                                          │
│  Recent Discoveries:                                     │
│    Day 3: "Found dark nodules in chalky ground"          │
│    Day 5: "Identified as flint — harder than stone"      │
│    Day 7: "Taught Marcus how to knap flint edges"        │
└──────────────────────────────────────────────────────────┘
```

### Knowledge Heatmap

A compact overview showing all plebs × all domains as a color grid:

```
              Stone  Fire  Food  Build  Creatures  Alien
  Ada         ████   ██░   █░░   ██░    █░░        ░░░
  Marcus      ██░░   ████   ██░   ████   ░░░        █░░
  Elena       █░░░   ██░   ████   █░░    ███        ██░
```

Colors: dark = ignorant, warm = familiar, bright = expert. At a glance, the player sees: "Ada is our stone expert, Marcus handles fire and building, Elena knows food and creatures."

This heatmap answers the key strategic question: **who should I send where?** Send Ada to the chalky ground. Send Elena to investigate the strange creature tracks.

## Recipe Visibility Rules

1. **Universal recipes** (hammerstone, cooking, basic building) — always visible in craft menu from day 1
2. **Material-gated recipes** — appear when ANY pleb has the required materials knowledge. First time a pleb handles flint → flint recipes appear.
3. **Skill-gated recipes** — appear when a pleb reaches the required skill level in the relevant domain. Kiln operation requires Fire knowledge ≥ 4.
4. **Discovery recipes** — appear only after successful experimentation. Per-map alien material recipes.

The craft menu grows organically. A new game shows ~8 recipes. By day 30, it might show ~25. By day 100, ~50+. At no point does the player see "47 locked recipes" — they see what they know.

## Implementation Plan

### Phase 1: Knowledge Data
- Add `knowledge: [f32; DOMAIN_COUNT]` to Pleb struct
- Add `colony_knowledge: ColonyKnowledge` to App struct
- ColonyKnowledge tracks: known items, noticed items, discovery log

### Phase 2: Colony Ledger UI
- New panel (egui window) opened from HUD book icon
- Colony view with domain clusters
- Pleb view accessible from character sheet
- Knowledge heatmap as compact overview

### Phase 3: Progressive Craft Menu
- Recipes have a `knowledge_required: Option<(Domain, f32)>` field
- Craft menu filters based on colony knowledge
- Grayed items show "needs [domain] knowledge level [N]"

### Phase 4: Discovery Events
- Noticed items tracked per-pleb
- Examination activity (pleb + noticed item + relevant skill → known)
- Cross-pleb teaching (conversation → knowledge transfer)
- Experimentation (work surface + materials → attempt discovery)

## Interaction with Existing Systems

- **DN-019 (knowledge gradients)**: The 6-level gradient maps to 0-10 skill scale
- **DN-022 (skill scale)**: Knowledge levels use the same XP curve
- **DN-026 (world lore)**: Lore stages correspond to knowledge domain progression
- **DN-027 (discovery feel)**: Silent identification for common items, moment events for rare
- **DN-033 (work surfaces)**: Capabilities on surfaces require knowledge to use
- **emergent-crafting.md**: Per-map alien recipes discovered through experimentation

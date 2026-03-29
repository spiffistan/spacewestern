# Lore, Research, and the Knowledge Economy

Knowledge on the frontier isn't a tech tree you click through. It's fragile, physical, carried by people, written in journals, stored on alien data slates, traded with caravans, and lost when the library burns down.

## The Core Idea

Knowledge exists in three states:

```
    TACIT                    WRITTEN                  SHARED
 (in someone's head)     (on paper/slate)        (multiple people know)

  ┌──────────┐          ┌──────────┐           ┌──────────┐
  │ Colonist │──write──→│ Journal  │──study───→│ Colony   │
  │ discovers│  (desk,  │ (item,   │ (library, │ knows it │
  │ something│   time)  │  tradeable│  time)   │ redundant│
  └──────────┘          └──────────┘           └──────────┘
       │                      │                      │
   dies? GONE.          burns? GONE.           resilient.
                        stolen? THEIRS.        but cost time.
```

**Tacit knowledge** lives in a colonist's mind. They learned by doing — observing creatures, experimenting at the workbench, studying an artifact. If they die, it's gone. Period.

**Written knowledge** is a physical item. A journal entry, a notebook, a data slate. It takes time to create (writing is work). But once it exists, it can be stored, traded, stolen, burned. It's the colony's intellectual property, as real as planks and iron.

**Shared knowledge** means multiple colonists have studied the written form. The colony is now resilient — losing one person doesn't lose the knowledge. But getting there costs time: someone had to discover it, write it down, and others had to read it.

## How Research Works

No research bench. No progress bars on abstract topics. Research is **doing things and paying attention**.

### Observation

A colonist assigned to watch something learns about it.

- **Watching duskweavers** (from a safe distance, at night): After several nights → "Duskweaver Behavioral Notes." Contains: pack coordination patterns, light-avoidance radius, feeding habits. **Effect:** Colony can predict duskweaver behavior; game shows their flee radius when selected.
- **Watching weather patterns**: A colonist on watchtower duty over weeks → "Weather Journal." **Effect:** Weather forecast appears 1 day ahead instead of on arrival.
- **Watching crops**: A farmer noting growth patterns → "Cultivation Notes." **Effect:** +15% crop yield for that crop type.

Observation requires proximity + time + the right backdrop. You don't learn about duskweavers by reading about them (initially) — you learn by being near them. This creates tension: the knowledge you need most comes from situations you'd rather avoid.

### Experimentation

A colonist at a workbench with materials can try things. Experimentation is a task like building — it takes time and consumes materials. Some experiments fail (materials wasted). Success creates a lore item.

- Combine venom sac + berries → "Antivenom Recipe" (or fail: wasted materials)
- Heat alien metal fragment in kiln → "Alien Metallurgy Notes" (or fail: fragment destroyed)
- Plant seeds in different soils → "Soil Composition Study" (slow, but guaranteed over a season)

The failure chance makes experimentation feel like real science. You're investing resources on an uncertain outcome. The more exotic the materials, the higher the payoff but also the higher the risk of wasting a rare component.

### Autopsy / Study

Dead creatures and alien artifacts yield knowledge when examined.

- Dead duskweaver + colonist with free time → "Duskweaver Anatomy." **Effect:** Unlocks creature-material recipes (pelts for warm clothing, bones for tools).
- Alien fragment (from DN-010) + study time → "Fragment Analysis #N." **Effect:** Contributes to a larger understanding. Collect enough → major recipe unlock.
- Settler remains (from DN-010) → "Settler's Journal." **Effect:** Lore text + sometimes a recipe the previous inhabitants knew.

Study requires a flat surface (workbench, table) and time. The colonist doesn't do anything else while studying. This is a real resource cost — that's a worker not hauling, not building, not farming. Knowledge has an opportunity cost.

## Lore Items

Lore is physical. It's an inventory item like planks or berries, but it goes on shelves instead of in crates.

### Item Properties

```
Lore Item:
  name: "Duskweaver Behavioral Notes"
  category: Xenobiology
  tier: Intermediate
  effect: TACTICAL_DUSKWEAVER  (shows flee radius, predicts spawn timing)
  author: "Jeff"               (the colonist who wrote it)
  source: observation          (how it was created)
```

### Categories

| Category | What It Covers | Example Items |
|----------|---------------|---------------|
| **Survival** | Water, food, weather | "Water Divining Techniques", "Edible Flora Guide" |
| **Construction** | Building, insulation | "Advanced Masonry", "Thermal Insulation Methods" |
| **Medicine** | Wounds, venom, disease | "Antivenom Formula", "Wound Treatment Protocol" |
| **Xenobiology** | Alien creatures | "Duskweaver Anatomy", "Thermogast Thermal Behavior" |
| **Alien Tech** | Artifacts, deciphered | "Fragment Analysis: Power Cells", "Alien Alloy Smelting" |
| **Geology** | Minerals, metallurgy | "Iron Smelting Process", "Copper Extraction" |
| **Agriculture** | Crops, soil, seasons | "Crop Rotation Guide", "Alien Soil Composition" |

### Tiers

| Tier | How Obtained | Example |
|------|-------------|---------|
| **Innate** | All colonists know this | Campfire, mud walls, basic shelter |
| **Basic** | Easy observation/experimentation | Berry preservation, fiber weaving |
| **Intermediate** | Extended observation, risky experimentation | Creature anatomy, mineral smelting |
| **Advanced** | Artifact study, rare materials | Alien metallurgy, advanced medicine |
| **Unique** | One-of-a-kind finds | Specific artifact abilities, lost civilization secrets |

## The Library

A building that stores and activates knowledge.

**Physical structure:** Bookshelves (new block type) placed in a room. Each shelf holds N lore items. A writing desk (new block type) where colonists create lore items from tacit knowledge.

**Activation:** A lore item stored on a library shelf applies its effect to the colony. A lore item in a crate or on the ground is just an object — it doesn't do anything until shelved.

**Study:** Colonists visit the library to read. Reading a lore item transfers the knowledge from "written" to "shared" — that colonist now has tacit knowledge of the topic, so even if the book burns, they still know it. But reading takes time (hours for basic, days for advanced).

**Vulnerability:** The library is the single most valuable building in the colony. A fire that destroys it erases accumulated knowledge. Raiders who steal from it take your research. This creates a powerful "protect the library" motivation and a devastating loss when it falls.

## Knowledge as Currency

On a frontier where everyone is struggling to survive, knowledge is more valuable than gold. Nobody has gold mines — but the colony that figured out alien metallurgy has something nobody else has.

### Trading Lore

Caravans buy and sell lore items like any other trade good, but with asymmetric value:

- A caravan from a desert settlement values "Water Divining Techniques" enormously but wouldn't pay much for "Crop Rotation Guide" (they can't grow crops anyway).
- Your "Duskweaver Anatomy" is worthless to a settlement that doesn't have duskweavers, but invaluable to one that does.
- A trader might offer a rare "Steel Forging" manual in exchange for your "Alien Fragment Analysis" — a technology exchange between settlements.

Knowledge value depends on context. This makes trading feel like negotiation, not just price comparison.

### Knowledge Diplomacy

- **Sharing freely:** Giving knowledge to traders/settlements builds reputation. You become known as generous/scholarly. Attracts better immigrants, better trade deals.
- **Hoarding:** Keeping knowledge to yourself maintains a competitive edge. But you miss out on trades and relationships.
- **Exclusivity deals:** "We'll sell you our smelting notes, but only to you." Creates alliances.
- **Theft:** Raiders target the library. Losing a unique lore item to enemies means they now have it and you don't (unless a colonist memorized it through study).

### Cross-Playthrough Echo

A subtle feature: when a colony falls (total wipe), a future playthrough's traders might occasionally arrive carrying journals "from a settlement that didn't make it." The lore items from your previous colony showing up in a new game — your past failure's knowledge surviving through trade networks. Not guaranteed, not mechanical — just a thematic echo.

## The Apprenticeship Layer

Beyond books, knowledge transmits between people directly.

**Master-apprentice:** An experienced colonist working alongside a novice transfers skill passively. The apprentice gains competence in that task over days/weeks. This is slower than reading a book but doesn't require literacy materials.

**Oral tradition:** A colonist who knows something but hasn't written it down can teach others directly. "Evening lessons" at the campfire — a social activity where the knowledgeable colonist shares what they know with those present. This creates a beautiful scene: colonists gathered around the fire, one telling the others about duskweaver behavior, everyone learning together. The sound sim carries the voice. Colonists who aren't present don't learn.

**The risk:** If the only person who knows something dies before teaching or writing it down, it's gone. This is the knowledge system's core tension — and the library's core value proposition.

## What This Does To Gameplay

### Early Game (Days 1-10)
All knowledge is tacit. Jeff knows how to make a campfire (innate). He learns from experience — building with mud, harvesting berries, watching the night sky. No written knowledge exists yet because there's no writing desk. The player focuses on survival.

### Mid Game (Days 10-30)
The colony builds a writing desk. The most experienced colonists start writing down what they've learned. "Berry Preservation Guide" goes on the shelf → food lasts longer. "Construction Methods" → buildings go up faster. A trader arrives with "Iron Smelting" notes from another settlement → a breakthrough the colony couldn't have discovered on its own.

### Late Game (Days 30+)
The library has shelves of knowledge. New colonists study existing lore to get up to speed quickly. A fire in the library is a crisis. The colony trades its unique alien research to other settlements for techniques they need. Knowledge IS the colony's competitive advantage.

### The Devastating Moment
The Doc is the only one who knows advanced wound treatment. She wrote it down last week — the journal is on the shelf. A raid hits. The library catches fire. The Doc runs in to save the journal and doesn't come out. Knowledge and knowledge-keeper, both lost. The colony now has to rediscover wound treatment from scratch — or trade for it.

This is the kind of moment PHILOSOPHY.md talks about. Not punishing — consequential. The player who didn't build a fireproof library, who didn't ensure multiple colonists studied the Doc's notes, faces a real loss. Next time, they'll build the library out of stone.

## Connection to Existing Systems

| System | Integration |
|--------|------------|
| **Discovery layer (DN-010)** | Artifacts are studied to create lore items. Mineral deposits are identified through geological lore. |
| **Creature system** | Observing/autopsying creatures creates xenobiology lore. Tactical lore shows creature stats. |
| **Crafting** | Intermediate+ recipes require lore items on library shelves. No lore = no recipe access. |
| **Card system** | Blueprint cards from CARDS.md are one form of lore item — found in ruins, traded, collected. |
| **Trading** | Lore items are high-value trade goods with context-dependent pricing. |
| **Sound sim** | Evening lessons at the campfire — teaching is a social sound event. |
| **Fire system** | Libraries burn. This is the system's primary risk vector. |
| **Building** | Library (room with shelves), writing desk, study activity. |

## What NOT To Build

- No tech tree. No research points. No "select topic → wait → unlocked."
- No guaranteed outcomes. Experimentation can fail. Observation requires patience.
- No global unlocks. Knowledge lives somewhere specific — a person, a book, a shelf. If it's gone, it's gone.
- No abstractions. You don't "research metallurgy level 3." You find an alien metal sample, your smartest colonist studies it for three days, and she writes "Alien Alloy Properties" in a journal that goes on the shelf. That journal is an object with a location and an author and a vulnerability.

## Implementation Notes

Mechanically, this is simpler than it sounds:

- Lore items are items (like berries, planks) with additional metadata (category, tier, effect ID)
- The library is a room + bookshelf blocks (like crates but for lore items)
- "Discovery" effects are flags checked at relevant points (crafting station checks shelf for recipe lore, creature system checks for tactical lore, etc.)
- Writing is a PlebActivity with a timer that produces an item
- Study is a PlebActivity with a timer that adds a skill/flag to the colonist
- Trading uses the existing item value system with context modifiers

The deep narrative feeling comes not from complex code but from the physical, losable, tradeable nature of the items and the organic way they're discovered.

---

## Unexplored Angles

### Lore as Map Annotations

What if written knowledge is spatially tied to the map? A colonist's "Water Divining Notes" doesn't just unlock a global bonus — it marks specific locations on the map where underground water was detected. "Mineral Survey, Eastern Ridge" highlights actual tiles. Trade this journal to another settlement and they get YOUR map data — including the location of your colony.

This makes lore items contain **intelligence**, not just knowledge. Selling your geological survey means the buyer knows where your iron deposits are. Do you trust them? Do you sell a redacted copy (less valuable, but safe)?

### The Unreliable Narrator

Not all lore is correct. A colonist who observed duskweavers for only one night might write "Duskweaver Notes" that contain inaccuracies — wrong flee radius, wrong pack thresholds. A more experienced observer writes better notes. Lore items could have a hidden **accuracy** stat:

- Quick observation → 60% accurate (some values slightly wrong)
- Extended study → 90% accurate (reliable)
- Expert observation → 99% accurate (definitive)

Inaccurate lore still helps — it's better than nothing. But acting on wrong information ("the notes say they won't attack groups of two") could be dangerous. Over time the colony corrects its knowledge through experience. A second edition of the notes replaces the first.

This creates a world where knowledge is uncertain, like real frontier survival. You think you understand the creatures. Mostly you're right. Sometimes you're not.

### Language and Translation

The alien artifacts aren't immediately readable. They're in an alien script. Deciphering requires:

1. **Finding multiple fragments** — each fragment is a partial key
2. **Pattern matching** — a colonist with high intelligence works on translation
3. **Breakthrough moments** — at certain thresholds (3 fragments, 7 fragments, 12 fragments), large portions of the language unlock at once
4. **Reading alien texts** — once partially translated, alien data slates reveal layered information. First pass: basic meaning. Second pass (more language knowledge): deeper technical detail. Third pass: cultural context, implications.

This turns artifact collection into a long-term archaeological puzzle. Each fragment makes ALL existing artifacts slightly more readable. There's a compounding return to exploration.

### Competing Knowledge Traditions

If other settlements exist, they may have developed different approaches to the same problems:

- Settlement A figured out irrigation through engineering (pipes, pumps)
- Settlement B figured out irrigation through biology (alien drought-resistant plants)
- Both solutions work. Trading knowledge between them gives your colony BOTH approaches.

But some knowledge might be **contradictory**:
- "Desert Survival Guide (Northern Methods)" says conserve water above all
- "Desert Survival Guide (Southern Methods)" says dig deep wells and use freely
- Both can exist on your shelf. Which do your colonists follow? The one they studied most recently? The one from the more reputable source?

This could be simple (last-studied wins) or complex (colonists develop preferences based on personality). Even simple, it creates narrative texture — your colony has an intellectual lineage, influenced by whose knowledge you acquired.

### Dangerous Knowledge

Some things are better not known.

- Studying an alien bio-weapon fragment: unlocks a powerful defense... but also reveals that the previous civilization destroyed themselves with it. Now YOUR colonists know how to make it. Can that knowledge be un-learned?
- Deep artifact study reveals the planet's ecology is artificial — the creatures were engineered. The Hollowcall isn't natural. This changes nothing mechanically but changes everything narratively. Some colonists might have a stress reaction to the discovery.
- A trader offers "Redskull Communication Codes." You can now eavesdrop on enemy communications (forewarning of raids). But possessing these codes means you understand their language. Some colonists might empathize, refuse to fight.

Knowledge has psychological weight. Some discoveries change how your people see the world. PHILOSOPHY.md's "permanence" theme extends to ideas — you can't un-know something.

### The Scribe Role

A colonist with high writing skill becomes the colony's most valuable non-combat asset:

- They write faster (more lore items per day)
- They write more accurately (higher accuracy stat on notes)
- They can **copy** existing lore items (creating duplicates for trade or backup)
- They can **compile** — combine multiple related notes into a comprehensive volume (e.g., three separate duskweaver studies → "Comprehensive Duskweaver Manual" with combined, higher-accuracy data)
- They write **beautifully** — their journals have higher trade value because other settlements prize the quality

This creates the Norland-scribe dynamic: a scholar whose physical output (books) is the colony's most tradeable and most fragile asset. Losing the scribe doesn't destroy existing books, but no new ones are written until someone else develops the skill.

### Knowledge Decay (Oral Tradition)

Tacit knowledge that is never written down degrades over time:

- A colonist who observed duskweavers 30 days ago remembers less detail than 5 days ago
- "Fuzzy memory" — the accuracy of tacit knowledge decreases slowly
- Writing it down "freezes" the knowledge at its current accuracy level
- This creates pressure to write things down soon after discovery, while the memory is fresh

But also: **well-studied knowledge is robust**. A colonist who spent a week observing duskweavers has strong memories that decay slowly. A colonist who glimpsed one briefly has weak memories that fade fast. Investment in observation pays off in knowledge permanence.

### Physical Form Factor

Different knowledge carriers have different properties:

| Medium | Durability | Portability | Capacity | Source |
|--------|-----------|-------------|----------|--------|
| **Oral** | Dies with person | Travels with them | Small (one topic) | Default |
| **Notebook** | Burns, gets wet | Light, tradeable | Medium (one topic detailed) | Writing desk + paper |
| **Data slate** | Nearly indestructible | Heavy | Large (complex diagrams) | Alien artifacts |
| **Wall carving** | Permanent, fire-resistant | Immovable | Small (symbols, warnings) | Stone wall + tools |
| **Teaching** | Spreads to many minds | Requires presence | Medium | Campfire + time |

A wall carving at the colony entrance: "Duskweavers flee from torchlight — always carry fire at night." Permanent, can't be stolen, can't be burned. But limited to short messages and can't be traded.

A data slate recovered from ruins: dense with alien technical knowledge. Fire-resistant. But heavy, and if traded away, the knowledge goes with it (unless someone studied it first).

Paper notebooks: the workhorse medium. Cheap to create, easy to trade, but burns.

The choice of medium matters. Carving your most critical knowledge into stone walls is slow but permanent — a hedge against catastrophe.

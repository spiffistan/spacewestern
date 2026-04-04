# DN-019: Knowledge and Crafting System

**Status:** Proposed
**Depends on:** DN-018 (equipment system), DN-017 (fauna and food)
**Supersedes:** None (integrates and extends lore-and-research.md, social-knowledge.md)
**Affects:** pleb.rs, simulation.rs, resources.rs, items.toml, recipes.toml, ui.rs, zones.rs

## Summary

A unified system where knowledge is a 6-level gradient embodied in people, spreads socially through conversation, degrades through retelling, and can be deliberately falsified by enemies. Crafting recipes are gated by knowledge level per domain — not a tech tree but a knowledge landscape where each colony traces a unique path based on what it discovers, who understands it, and what materials are available. No research bench. No progress bars. Knowledge is physical, losable, social, and occasionally wrong.

## Design Philosophy

**No tech tree.** No research bench. No progress bars on abstract topics. Research is doing things and paying attention. The progression structure is a landscape of knowledge domains with prerequisite dependencies — it FEELS like a tree when you trace the chains, but it's embodied in people rather than an abstract UI.

**Three-lock gating.** Every advanced capability requires three things to converge: Knowledge (someone understands how), Materials (the physical inputs exist), and Infrastructure (the right station/facility). These three locks rarely align simultaneously, creating natural pacing without artificial timers.

**Knowledge lives in people.** If the Expert metallurgist dies, the colony doesn't lose a tech unlock — it loses a PERSON'S understanding. Graceful degradation: someone at Familiar level can attempt the work with high failure rate. Total loss: if nobody even reached Aware, the capability is gone until rediscovered or traded for.

**Social transfer is automatic and imperfect.** Colonists talk. Knowledge spreads through conversation — at meals, at the saloon, during shared work. But each retelling degrades accuracy. Rumors distort. Enemies lie. What the colony "knows" might not be true.

**Every playthrough is unique.** Per-map calibration drift, hidden colonist aptitudes, random eureka moments, and seasonal knowledge windows ensure no two colonies develop the same way.

---

## Part 1: The Knowledge Gradient

### Six Levels

Every colonist has a level (0–5) per knowledge domain. Each level has distinct gameplay effects:

| Level | Name | Gameplay Effect |
|-------|------|-----------------|
| 0 | Unaware | Can't see related resources. Tasks don't appear. Walking past iron ore sees "unusual rock." |
| 1 | Aware | Recognizes related materials with "?" marker. Can mark for experts. Can't attempt. Understands the concept exists. |
| 2 | Familiar | Can attempt with ~60% failure rate. Slow. Wastes materials. But in a crisis, better than nothing. |
| 3 | Competent | Reliable execution. Normal speed and quality. ~5% failure. The working baseline. |
| 4 | Expert | +20% speed, -15% material waste. Can experiment (discover new recipes). Teaches effectively. Writes accurate lore. |
| 5 | Master | Rare. Masterwork quality. Invents techniques. Signature style on crafted items. Maybe 1 in 20 colonists ever reaches this. |

### Level Transitions

Each transition has different mechanisms, costs, and timescales:

**Unaware → Aware** (fast — just exposure):
- Overhear a conversation at the saloon about the topic
- A trader mentions it. A radio transmission references it.
- See someone else do it, even briefly
- Find a reference in a ruin or artifact
- Hear about a colonist's death from a related cause
- Fastest through extroverts and leaders (social amplifiers)

**Aware → Familiar** (moderate — requires deliberate engagement):
- Read a lore item about it in the library (hours)
- Watch an Expert demonstrate over an extended period (apprenticeship, full shift)
- Attend a teaching session (campfire lessons — Expert addresses all present)
- Study a related artifact
- One-on-one explanation from someone Competent+ (slower than Expert)

**Familiar → Competent** (slow — requires hands-on practice):
- Only crossing this gap through DOING. Books and teaching get you Familiar. Competence is earned.
- Each work attempt (success or failure) grants experience
- Failures grant MORE experience than successes (learning from mistakes)
- Working alongside Competent+ colonist reduces failure rate but doesn't speed transition
- Hidden aptitude affects speed: some cross in 5 sessions, others need 20
- Typical timeframe: days to weeks depending on domain and aptitude

**Competent → Expert** (very slow — sustained investment):
- Months of regular practice
- Encountering variety (different ores, different injuries, different crops)
- Teaching others deepens understanding (the teacher learns by teaching)
- Natural aptitude matters — some plateau at Competent permanently
- Some colonists never reach Expert. That's fine. Experts are valuable because they're uncommon.

**Expert → Master** (rare — talent + time + innovation):
- Years of focused practice
- Successfully innovating (new techniques through experimentation)
- Teaching widely (transcending their own skill)
- Requires both high domain aptitude AND the right personality
- A colony might never have a Master in any domain

### Knowledge Forgetting

A colonist who was Competent but hasn't practiced in ~3 months degrades toward Familiar. Use it or lose it. Practice component degrades faster than theoretical understanding — a lapsed Expert remembers what to do but their hands are rusty (slower execution, higher error rate). This creates a maintenance cost for capability.

### Knowledge Texture

Beyond level, each colonist's knowledge has properties that make it unique:

- **Accuracy** — How correct is their understanding? An Expert at 95% vs. a Familiar at 70%. Inaccurate knowledge causes occasional errors even at Competent level.
- **Breadth** — Narrow (knows iron only) vs. wide (understands iron, copper, alloys). Specialists vs. generalists within a domain.
- **Source** — Practical (learned by doing: fast hands, weak theory) vs. Theoretical (learned from books: understands principles, slow hands). Best practitioners have both.
- **Local calibration** — Adapted to THIS map's parameters? A traded recipe gives general knowledge; local practice gives calibrated knowledge.
- **Confidence** — High-confidence colonists work faster but catch fewer errors. Low-confidence colonists are slower but more careful. Overconfidence after early success leads to costly mistakes.

Two "Competent metallurgists" can be very different. Together they're better than either alone — collaborative knowledge compounds partial understanding.

---

## Part 2: Knowledge Domains

Knowledge is organized into domains, each with prerequisite dependencies. Dependencies gate the Unaware → Aware transition — you can't even START learning about smelting if your fire skills aren't Expert. Once Aware, progression follows normal learning mechanisms.

### Domain Registry

| Domain | What It Covers | Key Outputs |
|--------|---------------|-------------|
| Fire making | Starting fires, controlling heat, fuel management | Campfires, kilns, controlled burns |
| Basic construction | Shelter, walls, roofing | Mud walls, wood walls, thatch roofs |
| Woodworking | Shaping, joinery, planks | Furniture, planks, saw horse |
| Stoneworking | Cutting, stacking, masonry | Stone walls, stone tools, foundations |
| Clay working | Pottery, brick, kiln firing | Jugs, bricks, kiln |
| Cooking | Heat + ingredients → meals | All cooked food, baking, soup |
| Agriculture (basic) | Planting, harvesting, seasons | Crop farming, basic irrigation |
| Agriculture (advanced) | Crop rotation, soil science, greenhouse | Optimized yields, year-round growing |
| Foraging | Wild plant identification, safe eating | Berry harvesting, wild herb gathering |
| Hide curing | Tanning, leather working | Belts, vests, pouches, leather armor |
| Metallurgy (smelting) | Ore identification, furnace operation | Iron ingots, copper ingots |
| Metallurgy (forging) | Shaping metal, tempering | Iron tools, weapons, metal fittings |
| Chemistry (basic) | Reactions, distillation, compounds | Antiseptic, dyes, soap |
| Chemistry (gunpowder) | Explosive compounds | Gunpowder, ammunition, explosives |
| Fermentation | Controlled spoilage, yeast, alcohol | Wine, beer, vinegar, preserved food |
| Medicine (basic) | First aid, wound care, hygiene | Bandaging, splinting, wound cleaning |
| Medicine (advanced) | Surgery, pharmacology, disease | Internal treatment, antivenom, quarantine |
| Xenobiology | Alien creature behavior, anatomy | Tactical creature info, butchering techniques |
| Glass making | Melting sand, shaping, lenses | Glass panes, greenhouse, lenses |
| Alien tech (basic) | Recognizing alien artifacts, translation | Fragment analysis, basic decipherment |
| Alien tech (power) | Alien energy systems | Power taps, conduit interfaces |
| Alien tech (biology) | Alien biotech, engineered ecosystems | Creature understanding, bio-materials |
| Textiles | Fiber processing, weaving, sewing | Clothing, rope, sails |
| Power systems | Electricity, circuits, generators | Solar panels, wiring, batteries |

### Dependency Graph

Prerequisites specify the minimum level required in another domain before you can become Aware of the target domain:

```
INNATE (no prereqs — everyone starts here):
  Fire making
  Basic construction
  Foraging (basic wild plant recognition)
  Textiles (basic fiber twisting)

EARLY (require innate-tier foundations):
  Woodworking          ← Fire making ≥ Competent
  Clay working         ← Fire making ≥ Competent
  Cooking              ← Fire making ≥ Familiar
  Stoneworking         ← Basic construction ≥ Familiar
  Agriculture (basic)  ← Foraging ≥ Familiar

MID (require early-tier foundations):
  Hide curing          ← Cooking ≥ Aware AND Textiles ≥ Familiar
  Metallurgy (smelting)← Fire making ≥ Expert
  Chemistry (basic)    ← Cooking ≥ Familiar
  Fermentation         ← Cooking ≥ Aware
  Glass making         ← Fire making ≥ Expert AND Clay working ≥ Familiar
  Agriculture (adv.)   ← Agriculture (basic) ≥ Competent + survived 1 seasonal cycle
  Medicine (basic)     ← Foraging ≥ Familiar (herb identification)
  Power systems        ← Basic construction ≥ Competent

LATE (require mid-tier foundations):
  Metallurgy (forging) ← Metallurgy (smelting) ≥ Familiar
  Chemistry (gunpowder)← Chemistry (basic) ≥ Familiar AND Fire making ≥ Competent
  Medicine (advanced)  ← Medicine (basic) ≥ Competent
  Alien tech (basic)   ← Basic construction ≥ Familiar (recognize structural patterns)

DEEP (require late-tier foundations):
  Alien tech (power)   ← Alien tech (basic) ≥ Familiar AND Power systems ≥ Aware
  Alien tech (biology) ← Alien tech (basic) ≥ Familiar AND Medicine (basic) ≥ Aware
```

These dependencies create natural chains that feel like a tech tree but are embodied in people. The mechanic (Expert fire, Familiar alien tech) can start learning alien power immediately. The farmer (Unaware power systems) must first learn basic electricity. The tree is traversed through PEOPLE, not menus.

### Backstory Starting Knowledge

Each backstory sets initial knowledge levels, bypassing the normal Unaware state:

| Domain | Crash Surv. | Doc | Mechanic | Ranch Hand | Scout | Outlaw | Preacher | Engineer |
|--------|:-----------:|:---:|:--------:|:----------:|:-----:|:------:|:--------:|:--------:|
| Fire making | 3 | 3 | 3 | 3 | 4 | 3 | 3 | 3 |
| Construction | 2 | 1 | 4 | 3 | 2 | 2 | 2 | 4 |
| Foraging | 2 | 3 | 0 | 4 | 4 | 2 | 2 | 1 |
| Cooking | 1 | 2 | 1 | 3 | 2 | 2 | 3 | 1 |
| Medicine | 2 | 4 | 2 | 2 | 2 | 2 | 2 | 1 |
| Metallurgy | 0 | 0 | 4 | 0 | 0 | 2 | 0 | 3 |
| Agriculture | 0 | 1 | 0 | 4 | 1 | 0 | 2 | 0 |
| Combat | 2 | 1 | 2 | 2 | 3 | 4 | 2 | 1 |
| Alien tech | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 1 |
| Power systems | 1 | 0 | 3 | 0 | 0 | 0 | 0 | 4 |
| Textiles | 1 | 1 | 1 | 3 | 2 | 1 | 2 | 0 |

*(0=Unaware, 1=Aware, 2=Familiar, 3=Competent, 4=Expert. Master (5) is never a starting state.)*

The mechanic starts at Familiar with alien tech — they recognize technological principles even in alien form. Nobody else does. This makes the mechanic uniquely valuable for the alien tech track. The ranch hand starts Expert at farming — they don't need to learn what they already know. The crew selection (chargen.md "Manifest") is the colony's first knowledge-allocation decision.

---

## Part 3: Three-Lock Crafting

Every recipe beyond innate-tier requires three things to converge:

### Lock 1: Knowledge

Someone in the colony understands how to do it. Checked against two sources:

1. Is a lore item with the matching knowledge effect on a library shelf? (Colony-wide access.)
2. Does the specific crafting colonist have the domain at the required level? (Personal knowledge.)

If either is true, the recipe is available to that colonist. A library full of books means any colonist can attempt any documented recipe (at the book's accuracy level). A colonist with personal Expert knowledge doesn't need the book.

### Lock 2: Materials

The physical inputs exist in the colony's stockpiles or the crafting colonist's inventory. Materials come from:

- **Local harvesting:** Wood, clay, fiber, berries — available from day 1.
- **Mining:** Iron ore, copper, sulfur, stone — requires finding deposits and digging.
- **Salvage:** Crash wreck components, ruin artifacts — finite, non-renewable.
- **Hunting:** Hides, bones, venom glands, meat — requires combat + butchering (DN-018 knife).
- **Trade:** Materials from other biomes you can't produce locally.
- **Ancient infrastructure:** Alien materials from deep underground.

### Lock 3: Infrastructure

The crafting station or facility exists. Stations are themselves recipes with knowledge and material requirements:

| Station | Knowledge to Build | Materials | What It Enables |
|---------|-------------------|-----------|-----------------|
| Bare hands | Innate | — | Stone tools, fiber rope, mud walls |
| Campfire | Innate | 1 wood | Cooking, warmth, light |
| Saw horse | Woodworking ≥ Familiar | 2 wood | Planks |
| Workbench | Woodworking ≥ Familiar | 4 planks | Most crafting recipes |
| Kiln | Clay working ≥ Familiar | 10 clay | Pottery, bricks, glass (with knowledge) |
| Cookfire | Cooking ≥ Aware | Campfire + pot (crafted) | Cooking recipes |
| Smokehouse | Cooking ≥ Familiar AND Woodworking ≥ Familiar | Wood + stone (room build) | Smoked meat preservation |
| Fermenter | Fermentation ≥ Familiar | Clay jug + seal (workbench) | Alcohol, vinegar, preserved food |
| Smelter | Metallurgy ≥ Familiar | Stone + clay + firebrick | Iron/copper ingots |
| Forge | Metallurgy (forging) ≥ Familiar | Stone + iron ingots | Metal tools, weapons, fittings |
| Still | Chemistry ≥ Familiar AND Fermentation ≥ Familiar | Copper pipe + fire + vessel | Distilled spirits, concentrated medicine |
| Library | Woodworking ≥ Competent | Planks + shelves | Store/activate lore items |
| Writing desk | Textiles ≥ Aware (paper/ink) | Planks + ink + paper | Create lore items from tacit knowledge |
| Butcher table | Cooking ≥ Aware | Planks | Butcher creatures → meat + hide + bone |

**Circular dependency resolution:** You can't build a smelter if nobody knows smelting. But you can't learn smelting without a smelter. This resolves through:
1. A blueprint card found in ruins (grants Familiar-equivalent knowledge of the specific recipe)
2. A colonist with the Mechanic backstory (starts at Expert metallurgy — they already know)
3. An Expert fire-maker who experiments at the kiln with ore samples (risky, material-consuming, but possible)
4. Knowledge traded from a caravan (a "Smelting Methods" lore item purchased for trade goods)

Multiple paths to the same capability. Each colony finds a different path.

### Recipe Knowledge Requirements

Extending recipes.toml with knowledge gating:

```toml
# ── Innate recipes (no knowledge required) ──

[[recipe]]
id = 8
name = "Stone Axe"
station = "hand"
time = 4.0
inputs = [{item = 2, count = 2}, {item = 5, count = 1}]
output = {item = 20, count = 1}
# No knowledge field → always available

# ── Knowledge-gated recipes ──

[[recipe]]
id = 30
name = "Iron Ingot"
station = "smelter"
time = 15.0
knowledge = {domain = "metallurgy_smelting", min_level = 2}  # Familiar+
inputs = [{item = "iron_ore", count = 3}]
output = {item = "iron_ingot", count = 1}
failure_rate_by_level = [0.0, 0.0, 0.60, 0.05, 0.02, 0.00]  # per knowledge level 0-5
speed_multiplier_by_level = [0.0, 0.0, 0.5, 1.0, 1.2, 1.5]

[[recipe]]
id = 31
name = "Iron Knife"
station = "forge"
time = 12.0
knowledge = {domain = "metallurgy_forging", min_level = 2}
inputs = [{item = "iron_ingot", count = 1}, {item = 1, count = 1}]
output = {item = 70, count = 1}
quality_by_level = [0, 0, 30, 60, 80, 100]  # item quality 0-100, affects durability and performance
failure_rate_by_level = [0.0, 0.0, 0.50, 0.05, 0.01, 0.00]

[[recipe]]
id = 32
name = "Bitterbulb Stew"
station = "cookfire"
time = 60.0
knowledge = {domain = "cooking", min_level = 2}
min_temp = 80.0                                    # thermal sim: station must be ≥ 80°C
cook_duration = 60.0                               # must hold temp for this long
inputs = [{item = 51, count = 1}, {item = "water", count = 1}]
output = {item = 51, count = 1, cooked = true}
failure_rate_by_level = [0.0, 0.0, 0.40, 0.05, 0.01, 0.00]
under_cook_consequence = "food_poisoning"           # if pulled early or temp too low
```

### Knowledge Check Logic

```rust
fn can_craft(recipe: &Recipe, colonist: &Pleb, library: &Library) -> bool {
    if let Some(req) = &recipe.knowledge {
        let personal_level = colonist.knowledge.level(req.domain);
        let library_level = library.effective_level(req.domain);
        let best_level = personal_level.max(library_level);
        best_level >= req.min_level
    } else {
        true // innate recipe, always available
    }
}

fn craft_attempt(recipe: &Recipe, colonist: &Pleb) -> CraftResult {
    let level = colonist.knowledge.level(recipe.knowledge.domain);
    let fail_chance = recipe.failure_rate_by_level[level];
    let speed = recipe.speed_multiplier_by_level[level];
    let quality = recipe.quality_by_level[level];

    if rng.gen::<f32>() < fail_chance {
        // Failed — materials consumed, experience gained (more than success)
        colonist.knowledge.add_experience(recipe.knowledge.domain, FAIL_XP);
        CraftResult::Failed
    } else {
        colonist.knowledge.add_experience(recipe.knowledge.domain, SUCCESS_XP);
        CraftResult::Success { quality, speed_modifier: speed }
    }
}
```

---

## Part 4: Social Knowledge Transfer

Knowledge moves between people through conversation. This is automatic — colonists talk when in proximity with compatible activity states. The player designs social spaces; the colonists spread knowledge through them.

### Conversation Triggers

**High-frequency** (most transfer happens here): eating at same table, working at adjacent stations, resting near campfire, saloon evenings, walking together to shared work site.

**Low-frequency:** Passing on paths, visiting another's workspace, night watch together.

**Never:** One asleep, in combat, or in crisis. Both sprinting.

Social architecture IS knowledge architecture. A colony with communal meals has fast Awareness diffusion (days). Scattered workstations mean slow diffusion (weeks). The saloon is an information exchange, not just a mood building.

### Conversation Topics

Topic selected from weighted pool based on what participants know, feel, and recently experienced:

**Experience-driven** (highest weight): "I saw duskweavers at the perimeter" → Awareness spreads. Recent events dominate conversation.

**Expertise-driven** (medium weight): The mechanic explains a principle → listeners gain Awareness. Expert teaching Familiar = structured learning, faster progression.

**Emotional** (mood-triggered): Stressed colonist vents → stress spreads. Happy colonist shares → mood buff spreads. Grief is processed through shared conversation.

**Rumor** (secondhand): "Kai said there's iron east." Secondhand information, accuracy degraded from retelling.

**Idle** (low weight): Small talk. No knowledge content but builds social bonds over time.

### Emotional Valence of Awareness

What spreads, and how it makes people feel, depends on the knowledge:

| Type | Example | Effect |
|------|---------|--------|
| Hopeful | "The mechanic says she can smelt iron" | +2 mood: "Future feels possible" |
| Fearful | "Something hunts silently in the dark" | +3–8 stress, changes night behavior |
| Practical | "Bitterbulb poisons you raw" | Changes AI: colonists refuse raw bitterbulb |
| Forbidden | "The planet was engineered" | +3 permanent baseline stress, unlocks alien tech domain |
| Contested | "Thermogast won't cross water" vs. "I saw one wade" | Two competing beliefs; resolution through testing |

### Personality and Information Flow

| Trait | Effect on Knowledge Network |
|-------|----------------------------|
| Extrovert | Initiates frequently. Information hub. Knowledge flows through them ~2x. Spreads fear/rumors fast too. |
| Introvert | Rarely initiates. Accumulates from listening. Critical knowledge can get stuck. |
| Leader | Attracts listeners. ~3x Awareness transfer rate. Outsized mood/stress amplification. |
| Contrarian | Argues with claims. Slows consensus but catches errors. Colony immune system. |
| Storyteller | Enhances memorability. Higher retention but lower accuracy. Entertaining, not reliable. |
| Suspicious | Discounts stranger/newcomer/prisoner info. Slow to adopt but rarely fooled by lies. |

### The Chat Bubble System

Conversations are visible but not text-heavy. Small icon bubbles above colonist heads:

- **Icons by topic category:** Claw shape (creatures), hammer (tools/craft), plant (food), exclamation (danger), speech (social), book (teaching), crossed lines (argument), curve (laughter).
- **Tinted by emotional valence:** Neutral (default), warm (hopeful/teaching), cool/red (fear/argument), faded (uncertainty/rumor).
- **Click to expand:** Shows log detail — "Kai told Jeb about duskweavers. Jeb is now Aware. Jeb seems nervous."

**Sound sim integration:** Conversations are sound sources. Louder topics (arguments, excited teaching) propagate further. Whispers between high-trust pairs are low-amplitude — eavesdroppers must be adjacent. Colonists in the next room hear emotional tone through walls (muffled) without catching content.

### The Telephone Effect

Information degrades through retelling. Each handoff introduces drift:

1. Kai (Expert, direct observation): "5 duskweavers near the south wall at dusk"
2. Kai → Jeb: "A pack near the south wall" (count lost)
3. Jeb → dinner table: "A big pack south of us" (direction vague, "big" is editorializing)
4. Marcus → night watch: "Swarm of creatures from the south" (species uncertain, threat inflated)

Each retelling: specifics lost, emotional tone amplified, speaker's personality colors the message. The colony ends up more scared than the situation warrants — but also more prepared. Distortion has a survival function.

---

## Part 5: Lies and Deception

Deliberate falsehood enters the knowledge system through multiple vectors.

### Information Veracity

Every shared piece of information has a hidden veracity level:

| Veracity | Meaning | Source |
|----------|---------|--------|
| True | Matches game state | Direct Expert observation, verified experiment |
| Approximately true | Directionally correct, imprecise | Familiar observation, brief sighting |
| Outdated | Was true, no longer | Old observation, pre-drought water source |
| Mistaken | Speaker believes it, but wrong | Learning gaps, bad source, incomplete observation |
| Exaggerated | Core truth inflated | Fear, storyteller personality, retelling |
| Fabricated | Deliberately false | Enemy deception, manipulative prisoners, lying newcomers |

The listener sees only: "Kai says X." They can't see the veracity tag. Trust is assessed through speaker identity, past reliability, and corroboration.

### Deception Vectors

**Enemy prisoners:** Captured raiders have a hidden `deception` stat. When sharing information, each claim rolls for truth: `truth_chance = base_cooperation - deception_skill + trust_built_over_time`. Hostile prisoners lie strategically (inflated force estimates, fake vulnerabilities). Cooperative ones mostly tell truth. Even honest prisoners might withhold key details. The player never reaches 100% certainty.

**Lying newcomers:** Strangers arrive with `claimed_skills` and `actual_skills`. UI initially shows claimed. Each work attempt has a reveal chance: `|claimed - actual| × 0.1 per attempt`. Larger lies are exposed faster. Not all liars are malicious — some exaggerate from desperation or self-delusion. How the colony responds to the exposed lie (sympathy vs. distrust) depends on the context.

**Enemy radio disinformation:** Intercepted coded communications might be deliberately planted — false troop movements, fake supply routes, inflated force estimates. The cipher key decodes the message but doesn't verify it. Intelligence is information with unknown veracity.

**Trader deception:** A trader claims an alien fragment is authentic. Only an Expert in alien tech can appraise accurately. Seed quality, medicine efficacy, material purity — all can be misrepresented. Trust in specific traders builds over multiple visits based on past accuracy.

### Misinformation Cascades

A single lie cascades through the social system:
1. Prisoner tells Jeb: "50 raiders in the hills" (fabricated — actually 12)
2. Jeb tells dinner table. Colony becomes Aware of "large enemy force"
3. Resources diverted to defense. Stress increases.
4. Scout sent to verify — discovers 12 raiders. Truth revealed.
5. Colony mood: relief + anger at prisoner + lasting distrust of future prisoner claims

The inverse — truth dismissed as lie — is equally possible. Both emerge from the same system, neither scripted.

### Verification Mechanisms

- **Direct observation:** Send a scout. Costly but definitive.
- **Expertise check:** Ask the domain Expert to evaluate a claim.
- **Experiment:** Test it. If it works, confirmed. If not, refuted. Costs materials.
- **Consensus:** Multiple independent sources agree → higher confidence.
- **Track record:** Implicit trust score per colonist. High-trust information spreads faster and is believed more readily.

---

## Part 6: Randomness and Replayability

### Per-Map Calibration Drift

Physical constants vary slightly by map seed. General knowledge is approximately correct; local calibration requires practice:

| Parameter | Range | What It Means |
|-----------|-------|---------------|
| Bitterbulb denaturation temp | 75–90°C | A traded recipe says "82°C" but this map needs 87°C |
| Iron smelting optimal temp | 800–1100°C | Ore from different deposits requires different heat |
| Duskweaver flee radius | 4–8 tiles | Varies with temperature — bolder on cold nights |
| Crop growth rates | ±20% per crop per map | Dustroot grows fast HERE; bitterbulb grows slow |
| Ore purity per deposit | Variable | Some veins smelt clean, others need flux |
| Fermentation temperature range | ±5°C from base | "15–25°C" might be "18–28°C" on this map |

**Effect on veterans:** A veteran player knows the general shape (fire before smelting). But they don't know the specifics of this run. Meta-knowledge starts them at Aware, but they still calibrate through practice. The game always teaches.

### Hidden Learning Aptitudes

Every colonist has hidden affinities per domain, rolled at creation, invisible in UI:

```
aptitude: f32  // 0.5 (struggles) to 2.0 (natural talent)

Learning speed = base_rate × aptitude
```

Discovered through play: "Kai seems to pick up smelting faster than Jeb." Optional subtle hints — a thought bubble: "This feels natural to me" or "I don't think I'm cut out for this."

Two runs with the same starting crew diverge. Run 1: Kai is your smith. Run 2: Kai is terrible at smithing but an amazing farmer.

### Eureka Moments

Small random chance per work attempt of a breakthrough — a sudden insight that either accelerates knowledge progression or discovers a novel technique unique to this colony:

```
Eureka chance per work attempt:
  Familiar:   1%
  Competent:  3%
  Expert:     5%

Eureka types:
  - Accelerated learning: jump forward in experience (skip days of grinding)
  - Novel recipe: discover a recipe no blueprint teaches (unique to this colony)
  - Calibration insight: instantly learn a local parameter (no more trial and error)
  - Teaching moment: nearby observers gain a burst of understanding
```

Can't be farmed — happens during real work as a side effect. The cook who accidentally burns bitterbulb discovers that charred bitterbulb + sap = adhesive. Unique recipe, unique to this playthrough, named by the colony ("Kai's Mistake"). Each eureka makes this colony's knowledge unique.

### Seasonal Knowledge

Some knowledge can only be gained at certain times:
- Spring flooding → learned by experiencing it
- Winter crop failure → learned by failing
- Autumn preservation timing → learned by missing the window
- Underground farming → discovered by trying in winter

A colony that played it safe in year 1 has narrow knowledge. A colony that experimented aggressively has broad-but-shallow knowledge. Both viable strategies.

---

## Part 7: Data Model

### Colonist Knowledge State

```rust
/// Knowledge domain identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KnowledgeDomain {
    FireMaking,
    BasicConstruction,
    Woodworking,
    Stoneworking,
    ClayWorking,
    Cooking,
    AgricultureBasic,
    AgricultureAdvanced,
    Foraging,
    HideCuring,
    MetallurgySmelting,
    MetallurgyForging,
    ChemistryBasic,
    ChemistryGunpowder,
    Fermentation,
    MedicineBasic,
    MedicineAdvanced,
    Xenobiology,
    GlassMaking,
    AlienTechBasic,
    AlienTechPower,
    AlienTechBiology,
    Textiles,
    PowerSystems,
}

/// Knowledge level 0-5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum KnowledgeLevel {
    Unaware = 0,
    Aware = 1,
    Familiar = 2,
    Competent = 3,
    Expert = 4,
    Master = 5,
}

/// A single domain's knowledge state for one colonist.
#[derive(Clone, Debug)]
pub struct DomainKnowledge {
    pub level: KnowledgeLevel,
    pub experience: f32,           // progress toward next level (0.0–1.0)
    pub accuracy: f32,             // 0.0–1.0, how correct their understanding is
    pub source: KnowledgeSource,   // how they learned (practical, theoretical, mixed)
    pub locally_calibrated: bool,  // adapted to this map's specific parameters
    pub confidence: f32,           // 0.0–1.0, affects speed vs error tradeoff
    pub last_practiced: f32,       // game time of last use (for forgetting)
}

#[derive(Clone, Copy, Debug)]
pub enum KnowledgeSource {
    Practical,   // learned by doing — fast hands, weak theory
    Theoretical, // learned from books — understands why, slow hands
    Mixed,       // both — the best practitioners
}

/// All knowledge for one colonist.
#[derive(Clone, Debug)]
pub struct PlebKnowledge {
    pub domains: HashMap<KnowledgeDomain, DomainKnowledge>,
    pub aptitudes: HashMap<KnowledgeDomain, f32>,  // hidden, 0.5–2.0
    pub trust_score: f32,                           // how much others believe this person
}

impl PlebKnowledge {
    pub fn level(&self, domain: KnowledgeDomain) -> KnowledgeLevel {
        self.domains.get(&domain).map(|d| d.level).unwrap_or(KnowledgeLevel::Unaware)
    }
}
```

### Conversation State

```rust
/// An active conversation between colonists.
#[derive(Clone, Debug)]
pub struct Conversation {
    pub participants: Vec<usize>,        // pleb indices
    pub topic: ConversationTopic,
    pub emotional_valence: f32,          // -1.0 (fearful) to +1.0 (hopeful)
    pub sound_amplitude: f32,            // injected into sound sim at midpoint position
    pub duration_remaining: f32,         // seconds
    pub knowledge_transferred: bool,     // has Awareness transfer happened this conversation
}

#[derive(Clone, Debug)]
pub enum ConversationTopic {
    Experience(KnowledgeDomain, String),  // domain + event description
    Teaching(KnowledgeDomain),            // Expert teaching Familiar+
    Emotional(MoodEvent),                 // stress/joy/grief sharing
    Rumor(KnowledgeDomain, f32),          // domain + veracity (0.0-1.0)
    Idle,                                 // no knowledge content, builds bonds
    Argument(KnowledgeDomain, usize, usize), // domain + two disagreeing pleb indices
}
```

### Information Veracity

```rust
/// How true a piece of shared information is.
#[derive(Clone, Copy, Debug)]
pub enum Veracity {
    True,              // matches game state
    ApproximatelyTrue, // directionally correct, imprecise
    Outdated,          // was true, no longer
    Mistaken,          // speaker believes it, but wrong
    Exaggerated,       // core truth inflated
    Fabricated,        // deliberately false
}

/// A piece of information in the colony's knowledge network.
#[derive(Clone, Debug)]
pub struct SharedInfo {
    pub domain: KnowledgeDomain,
    pub content: String,                // human-readable description
    pub veracity: Veracity,             // hidden from player
    pub original_source: usize,         // pleb index who first reported
    pub retelling_count: u8,            // how many times retold (degrades accuracy)
    pub believed_by: Vec<usize>,        // pleb indices who believe this
    pub disputed_by: Vec<usize>,        // pleb indices who doubt this
    pub verified: Option<bool>,         // None=unverified, Some(true)=confirmed, Some(false)=debunked
}
```

### Lore Items

```rust
/// A physical knowledge artifact — book, journal, data slate.
#[derive(Clone, Debug)]
pub struct LoreItem {
    pub id: u16,
    pub name: String,
    pub domain: KnowledgeDomain,
    pub knowledge_effect: String,       // effect ID checked by recipe system
    pub effective_level: KnowledgeLevel,// what level of understanding it conveys
    pub accuracy: f32,                  // 0.0–1.0, author's knowledge quality at time of writing
    pub author: Option<String>,         // colonist name who wrote it
    pub medium: LoreMedium,
}

#[derive(Clone, Copy, Debug)]
pub enum LoreMedium {
    Notebook,       // burns, tradeable, common
    DataSlate,      // fire-resistant, heavy, alien origin
    WallCarving,    // permanent, immovable, short messages
    OralOnly,       // no physical form, lives in memory
}
```

### Per-Map Calibration

```rust
/// Map-specific parameter variations, seeded from map RNG.
/// General knowledge gives approximate values; local practice reveals exact ones.
#[derive(Clone, Debug)]
pub struct MapCalibration {
    pub bitterbulb_denature_temp: f32,   // 75.0–90.0°C
    pub iron_smelt_optimal_temp: f32,    // 800.0–1100.0°C
    pub duskweaver_flee_radius: f32,     // 4.0–8.0 tiles
    pub duskweaver_cold_boldness: f32,   // how much flee_radius shrinks in cold
    pub crop_growth_modifiers: HashMap<String, f32>,  // per-crop ±20%
    pub ore_purity: HashMap<(i32, i32), f32>,  // per-deposit 0.5–1.0
    pub fermentation_temp_range: (f32, f32),  // shifted ±5°C from base
    pub soil_fertility_variance: f32,    // map-wide modifier
}

impl MapCalibration {
    pub fn from_seed(seed: u64) -> Self {
        let mut rng = Rng::seeded(seed);
        MapCalibration {
            bitterbulb_denature_temp: 75.0 + rng.gen::<f32>() * 15.0,
            iron_smelt_optimal_temp: 800.0 + rng.gen::<f32>() * 300.0,
            duskweaver_flee_radius: 4.0 + rng.gen::<f32>() * 4.0,
            // ... etc
        }
    }
}
```

---

## Part 8: Progression Timeline

What the player experiences, day by day. Not scripted — emergent from the three-lock system.

### Days 1–5: Pure Survival (Innate Only)

All knowledge is innate-tier. Stone tools, mud walls, campfire, berry foraging. No knowledge barriers. The colony is a handful of people trying not to die. Crash rations provide a 15-day grace period.

Conversations happen around the campfire at night. Colonists become Aware of each other's backstory knowledge: "The mechanic mentioned something about metal." "The doc knows about healing plants." Awareness spreads freely through social contact. The colony learns what it COULD become, even though it can't act on most of it yet.

### Days 5–15: First Knowledge Gates

The first "I can't do that" moments. A colonist tries to smelt a rock they found — nothing happens (Unaware of metallurgy). Another tries to cook bitterbulb and gets sick (Familiar at cooking, but didn't calibrate for this map's denaturation temp).

The mechanic builds a saw horse (Familiar with woodworking from backstory). Planks flow. The workbench follows. The crafting chain starts opening up. These feel like accomplishments because they're gated by WHO YOU HAVE, not just what materials are around.

A trade caravan arrives. They mention glass-making. Three colonists are now Aware of glass-making (hopeful awareness: "Future feels possible: +2 mood"). But nobody is Familiar yet, and there's no kiln temperature knowledge. The seed is planted for a future capability.

### Days 15–30: Specialization Diverges

Colonies diverge here based on what they found and who they have:

**Colony A** (mechanic + iron deposit nearby): Metallurgy track opens. Smelter built. Iron tools. The colony's identity crystallizes around metal-working. They trade iron goods to caravans.

**Colony B** (ranch hand + good farmland): Agriculture track deepens. Multiple crops, irrigation, food surplus. They trade preserved food. Metallurgy is locked — no ore found, no one knows smelting.

**Colony C** (doc + herb-rich biome): Medicine and chemistry advance. Antivenom, fermentation, herbal remedies. They become the settlement other caravans visit when injured.

No colony has everything. Trade fills gaps. The social knowledge system means Awareness of other domains is widespread (everyone KNOWS about metallurgy from the mechanic's campfire stories) but actual capability is narrow and deep.

### Days 30–60: The Library Age

The writing desk is built. The most experienced colonists begin writing down what they've learned. "Berry Preservation Guide" goes on the shelf → food spoilage knowledge is now colony-wide. "Local Iron Smelting" → anyone can attempt smelting (from the book, at Familiar level).

Knowledge transitions from fragile (in one person's head) to durable (on a shelf). But the library is now the colony's most valuable building. A fire that destroys it erases accumulated knowledge. Raiders who steal from it take intellectual property.

Teaching sessions happen at the campfire — the Expert metallurgist explains principles to the group. Three colonists reach Familiar in metallurgy. The colony now has redundancy. The single-point-of-failure flag disappears from the knowledge panel.

### Days 60+: Deep Exploration

Underground ruins yield alien fragments. The mechanic (Familiar with alien tech from backstory) begins analysis. Translation progresses — each fragment makes ALL existing fragments slightly more readable. Compounding return on exploration.

An alien power conduit is discovered. The mechanic, now Aware of alien power (through fragment study), can attempt to interface with it — at Familiar level, with high failure rate. But the potential payoff is enormous: free energy that doesn't need fuel.

Eureka moments have accumulated. The colony has 2–3 unique recipes that no other colony has, each named for the colonist who stumbled on them. "Kai's Mistake" (the adhesive), "Jeb's Stew" (the accidental superfood), "Marcus's Tonic" (the hangover cure that turned out to be an effective antiseptic). Each one a story. Each one irreplaceable.

### Year 2+: Knowledge as Identity

The colony has a library with 15+ lore items. New arrivals study them to get up to speed. The colony's unique tech profile — what it knows, how it learned it, what it's missing — IS its identity. Trading knowledge with other settlements creates alliances. The colony that figured out alien power conduits has something no one else has. The one across the mountains figured out alien biology. Neither has both. Trade connects them.

### The Devastating Moment

The Expert metallurgist is the only one at Competent+ in the domain. She wrote her techniques down last week — the journal is on the library shelf. A raid hits. The library catches fire. She runs in to save the journal and doesn't come out.

Knowledge and knowledge-keeper, both lost. The colony has Familiar-level metallurgists who studied from her teaching, and whatever they remember. The smelter still works. The recipes are still known (from the two Familiar colonists' memory). But quality drops. Failure rate jumps. The colony's iron output plummets.

This is PHILOSOPHY.md's "consequential, not punishing." The player who didn't build the library from stone, who didn't ensure multiple Competent metallurgists, faces a real loss. Next time, they'll build redundancy.

---

## Part 9: Buffs and Debuffs

### Colony-Wide

| Condition | Effect |
|-----------|--------|
| Expert teaching regularly | "Learning culture: +3 mood" |
| Library with 10+ lore items | "Accumulated wisdom: +2 mood" |
| No one Competent in medicine | "No doctor: -5 mood" (scales with injuries) |
| Eureka discovery this week | "Breakthrough!: +5 mood" for 3 days |
| Knowledge lost to death | "We lost so much: -8 mood" scaling with level lost |
| Misinformation recently exposed | "Trust shaken: -3 mood" for 5 days |
| Multiple unresolved contested beliefs | "Uncertain times: -2 mood" |
| All critical domains have redundancy | "Self-sufficient: +3 mood" |

### Individual

| Event | Effect |
|-------|--------|
| Becoming Aware (hopeful) | +2 mood for 2 days |
| Becoming Aware (fearful) | +3–8 stress |
| Becoming Aware (forbidden) | +3 permanent baseline stress |
| Reaching Competent | +5 mood for 3 days ("I can do this") |
| Reaching Expert | +8 mood for 5 days, +1 permanent mood in domain workspace |
| Reaching Master | +12 mood for 7 days, colony event notification |
| Failed experiment | -2 mood, +extra experience toward next level |
| Teaching successfully | +3 mood for teacher, +2 for student |
| Learning a lied-about claim was false | -5 mood, permanent trust reduction toward source |
| Expertise questioned (contrarian was right) | -2 then +1 mood (humbling but educational) |
| Practicing a skill to prevent forgetting | No buff. The work itself is the reward — maintaining capability. |

---

## Part 10: Connection to All Systems

| System | Knowledge-Craft Connection |
|--------|---------------------------|
| **Pleb struct** (pleb.rs) | `PlebKnowledge` replaces/extends current skill system. Knowledge level determines work speed, quality, failure rate. |
| **Recipes** (recipes.toml) | `knowledge` field gates recipes. `failure_rate_by_level` and `quality_by_level` arrays per recipe. |
| **Items** (items.toml) | Lore items are inventory items with knowledge metadata. Tiny items (DN-018) in pouches include knowledge-adjacent materials (herbs, specimens). |
| **Crafting stations** | Stations are themselves knowledge-gated recipes. Building a smelter requires metallurgy knowledge. |
| **Needs system** (needs.rs) | Knowledge-derived mood buffs/debuffs feed into existing mood calculation. |
| **Simulation** (simulation.rs) | Conversation tick runs alongside existing pleb AI. Knowledge spreading happens during social activities. |
| **Sound sim** (sound.wgsl) | Conversations are sound sources. Eavesdropping is physical. Teaching range limited by acoustic propagation. |
| **Thermal sim** (thermal.wgsl) | Cooking recipes check block_temps. Per-map calibration means temperature thresholds vary. Spoilage rate driven by temperature × knowledge accuracy. |
| **Fluid sim** (fluid.wgsl) | Smokehouse curing driven by smoke density. Knowledge level determines whether colonist reads conditions correctly. |
| **UI** (ui.rs) | Chat bubbles on colonists. Knowledge panel per colonist. Colony knowledge overview. Library contents view. Vulnerability flags. |
| **Equipment** (DN-018) | Tool proficiency tied to knowledge. Can't effectively use what you don't understand. Pouch contents (herbs, specimens) are knowledge-adjacent. |
| **Creatures** (alien-fauna.md) | Xenobiology knowledge gates tactical info display. Creature observation is a knowledge-generating activity. |
| **Food** (food-and-survival.md) | Cooking knowledge with real temperature physics. Recipe accuracy matters. Under-cooking has consequences. |
| **Combat** (DN-011) | Prisoner interrogation generates SharedInfo with hidden veracity. Enemy deception enters the knowledge system. |
| **Library** (lore-and-research.md) | Physical building that stores/activates lore items. Fire vulnerability. Trade value of contents. |
| **Trade** (gameplay-systems.md) | Knowledge is a high-value trade good. Context-dependent pricing. Trader claims have hidden veracity. |
| **Radio** (the-human-layer.md) | Information source with unknown veracity. Disinformation vector. Broadcasts trigger Awareness events. |
| **Chargen** (chargen.md) | Backstory determines starting knowledge levels across all domains. Crew selection IS initial knowledge allocation. |

---

## Part 11: Implementation Plan

### Phase 1: Knowledge Gradient on Plebs (Minimal)

Add `PlebKnowledge` to the Pleb struct. Initialize from backstory table. Display in colonist panel as simple bars per domain. No gameplay effect yet — just visible state.

**Files:** pleb.rs, ui.rs
**Scope:** Small. Data structure + display. Existing skills array can coexist during transition.

### Phase 2: Recipe Knowledge Gating

Add `knowledge` field to recipes.toml. Implement the `can_craft` check in crafting code. Recipes without the field remain innate (backwards compatible). Add `failure_rate_by_level` so Familiar colonists fail frequently.

**Files:** recipes.toml, recipe_defs.rs, simulation.rs (crafting activity)
**Scope:** Medium. The core mechanic — knowledge gates recipes.
**Blocks:** Phase 1.

### Phase 3: Knowledge Experience and Progression

Implement experience gain from work attempts. Failures grant more XP than successes. Track `last_practiced` for forgetting. Level-up events with mood effects. The Familiar → Competent grind becomes real.

**Files:** pleb.rs, simulation.rs, needs.rs (mood integration)
**Scope:** Medium. Numbers tuning will be iterative.
**Blocks:** Phase 2.

### Phase 4: Social Knowledge Transfer

Implement `Conversation` system. Awareness spreads through proximity + social activity. Chat bubble rendering. Topic selection from weighted pool. The saloon becomes an information exchange.

**Files:** simulation.rs (conversation tick), pleb.rs (conversation state), ui.rs (bubble rendering)
**Scope:** Large. This is a new simulation subsystem. But it runs alongside existing pleb AI.
**Blocks:** Phase 1.

### Phase 5: Lore Items and Library

Implement `LoreItem` as inventory item type. Library block (bookshelf). Writing desk activity (tacit → written). Study activity (written → shared). Lore items on shelves grant colony-wide knowledge access.

**Files:** resources.rs, items.toml, simulation.rs (write/study activities), ui.rs (library view)
**Scope:** Medium. Extends existing item and activity systems.
**Blocks:** Phase 3.

### Phase 6: Information Veracity and Deception

Implement `Veracity` on `SharedInfo`. Prisoner interrogation with hidden deception stat. Newcomer claimed vs. actual skills. The telephone effect (accuracy degrades per retelling). Verification mechanisms.

**Files:** pleb.rs (prisoner/newcomer data), simulation.rs (interrogation activity, retelling degradation)
**Scope:** Medium. Mostly data + checks. The dramatic payoff is in emergent narratives, not complex code.
**Blocks:** Phase 4 (needs conversation system).

### Phase 7: Per-Map Calibration and Randomness

Implement `MapCalibration` seeded from map RNG. Hidden colonist aptitudes. Eureka moment rolls during work attempts. Seasonal knowledge gates.

**Files:** grid.rs or new calibration.rs, pleb.rs (aptitudes), simulation.rs (eureka rolls)
**Scope:** Small-Medium. Mostly parameter tables and random rolls. The replayability impact is large relative to code cost.
**Blocks:** Phase 3 (needs experience system for eureka moments).

### Phase 8: Colony Knowledge UI

Colony-wide knowledge overview panel. Vulnerability flags. Knowledge gap alerts. Conversation log. Lore item browser. Trust scores. The player's window into the knowledge landscape.

**Files:** ui.rs
**Scope:** Large (UI always is). But all underlying data exists from prior phases.
**Blocks:** All prior phases.

---

## Open Questions

1. **How many domains?** The current list has 24. Is this too many for the UI? Could some be merged (MetallurgySmelting + MetallurgyForging → Metallurgy with sub-specializations)? Recommendation: start with 12–15 visible domains; sub-domains are invisible internal specializations that affect quality/speed.

2. **Should the player see veracity?** Current design: hidden. Player infers trust from context. Alternative: show a "reliability" indicator on information (color-coded confidence). Recommendation: hidden for immersion. The player should feel the same uncertainty the colonists feel. Optional "trust network" debug overlay for testing.

3. **How fast should Awareness spread?** Too fast and the whole colony is Aware of everything in a week — no value in the gradient. Too slow and knowledge feels stuck. Recommendation: base rate = 1 Awareness transfer per meaningful social interaction (meal, saloon evening). An extrovert + leader combo can Awareness-flood the colony in 3–5 days. An all-introvert colony takes 2–3 weeks. These feel right for the pacing.

4. **Should forgotten knowledge go all the way to Unaware?** If a Competent metallurgist stops for 6 months, do they degrade to Familiar, or all the way to Unaware? Recommendation: never below Aware for something you were once Competent at. You can forget how to DO it, but you can't forget that it EXISTS. Familiar is the floor for former-Competent colonists. They can re-learn faster (residual experience).

5. **Calibration drift amount — how much per-map variation?** Too little and veterans figure it out in one attempt. Too much and traded knowledge is useless. Recommendation: ±15% on most parameters. Enough that a recipe from another settlement needs 2–3 attempts to calibrate, not 20. The variation is meaningful but not crippling.

6. **Can Awareness be wrong?** Can a colonist be Aware of something that doesn't exist on this map? ("I heard there's iron in these hills" — there isn't.) Recommendation: yes. Awareness from rumors can be wrong. Only Familiar+ (through direct observation or verified sources) is guaranteed accurate. This gives the Aware level its distinctive flavor: "I've heard of it, but I haven't seen it myself."

7. **Should there be a colony "knowledge momentum"?** A colony that's been learning and teaching has a faster base learning rate than a stagnant one. A "culture of curiosity" buff from sustained knowledge growth. Recommendation: yes, as a subtle modifier. +10% learning speed if 3+ colonists gained a level in the past 30 days. The colony that invests in knowledge accelerates.

8. **Relationship to existing `skills: [u8; 6]`?** The current Pleb struct has a skills array [shooting, melee, crafting, farming, medical, construction]. The knowledge system replaces this with a richer model. Migration path: map existing skills to starting domain levels (crafting → average of relevant domains, farming → agriculture, etc.) and phase out the flat array. The knowledge system is strictly more expressive.

---

## Summary

This system replaces a traditional tech tree with something more organic: knowledge as a lived, social, physical, and occasionally unreliable resource that shapes every colony uniquely.

**The gradient (Unaware → Master)** gives knowledge weight. Each level has distinct gameplay: Unaware can't even see opportunities. Aware knows possibilities exist. Familiar can attempt with risk. Competent is reliable. Expert innovates. Master transcends. The levels create drama — a colony losing its only Expert doesn't hit a binary lock, it degrades to Familiar-level capability and fights to rebuild.

**Three-lock gating (Knowledge + Materials + Infrastructure)** creates natural pacing. You can't rush what you haven't found, can't use what you don't understand, can't make what you can't build the tools for. Each lock comes from a different part of the game — exploration provides materials, social learning provides knowledge, crafting provides infrastructure. They converge unpredictably.

**Social spreading** makes the colony a knowledge network. The saloon, the campfire, the shared workspace — these aren't just mood buildings, they're information exchanges. Extroverts are hubs. Introverts are bottlenecks. Leaders amplify. Contrarians correct. The social architecture the player designs for mood also determines how fast the colony learns.

**Deception** makes information dangerous. Prisoners lie. Newcomers exaggerate. Traders misrepresent. Radio broadcasts deceive. The telephone effect degrades truth through retelling. The colony must verify, test, and evaluate — turning information into knowledge is itself a skill. Trust is earned, and losable.

**Randomness** makes every playthrough unique. Per-map calibration drift means general knowledge requires local adaptation. Hidden aptitudes mean the same colonist plays differently each run. Eureka moments create colony-specific recipes. Seasonal windows restrict when certain knowledge can be gained. A veteran player starts at Aware, but never stops learning.

**The crafting connection** is clean: a `knowledge` field on recipes, checked against library shelves and colonist personal knowledge. `failure_rate_by_level` and `quality_by_level` arrays give each knowledge level mechanical weight. The recipe system doesn't change — it just gains a gate that makes every unlock feel earned, personal, and precarious.

The result: a colony sim where the most valuable resource is understanding, the most devastating loss is expertise, and the most interesting question is "who knows what, and can we trust them?"

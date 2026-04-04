# Social Knowledge

How information moves between people. Knowledge isn't just discovered — it's shared, distorted, debated, and sometimes fabricated. The social layer determines what the colony collectively believes, how fast it learns, and whether what it "knows" is actually true.

## Relationship to Other Docs

This doc is the connective tissue between:
- **lore-and-research.md** — defines knowledge states (tacit/written/shared) and how research works
- **the-human-layer.md** — defines the saloon, naming, superstition, moral drift
- **alien-fauna.md** — creatures whose behavior becomes the subject of shared (and sometimes wrong) knowledge
- **food-and-survival.md** — cooking knowledge that transfers between people with real consequences for errors
- **DN-018** — equipment loadouts that reflect what a colonist knows how to use

This doc adds: the MECHANISM by which knowledge flows between people, how that flow distorts information, and how enemies exploit the system through deception.

---

## Knowledge Gradient (6 Levels)

Every colonist has a level per knowledge domain. Not binary — a spectrum with gameplay implications at each point:

| Level | Label | What it means | Gameplay effect |
|-------|-------|---------------|-----------------|
| 0 | **Unaware** | Doesn't know this exists | Can't see related resources. Task doesn't appear as option. |
| 1 | **Aware** | Has heard of it | Recognizes related materials with "?" marker. Can mark locations for experts. Can't attempt the task. |
| 2 | **Familiar** | Understands the principle | Can attempt with high failure rate (~60%). Wastes materials. Slow. But in a crisis — better than nothing. |
| 3 | **Competent** | Reliable execution | Normal speed, quality, ~5% failure. The working baseline. Gets here only through sustained practice. |
| 4 | **Expert** | Fast, efficient, innovative | +20% speed, -15% material waste. Can experiment (discover new recipes). Can teach effectively. Writes high-accuracy lore. |
| 5 | **Master** | Exceptional, signature quality | Rare. Creates masterwork items. Invents techniques blueprints can't teach. Maybe 1 in 20 colonists reaches this in anything. |

**The critical insight:** Familiar is where the most interesting gameplay happens. Your Expert metallurgist dies. Your farmer (Familiar — she watched the Expert work) is now your best option. She fails twice, wasting ore. Third attempt: crude ingot. Over weeks she grinds to Competent. She'll never be what the Expert was, but the colony survives. That story only exists because knowledge has a gradient, not a switch.

---

## Conversation as Game Mechanic

Two colonists in proximity with compatible activity states can converse. Conversations are:
- **Sound sources** in the GPU sound sim — amplitude varies by topic intensity
- **Information channels** — topics flow between participants
- **Mood modifiers** — emotional valence spreads to participants and eavesdroppers
- **Visible** — icon bubbles above heads show topic category, not text

### When Conversations Happen

**High-frequency** (most knowledge transfer happens here):
- Eating at the same table
- Working at adjacent stations
- Resting near the same campfire
- Drinking at the saloon in the evening
- Walking together to a shared work site

**Low-frequency:**
- Passing each other on a path
- Visiting another's workspace
- Night watch together
- Waking up in the same sleeping area

**Never:**
- One is asleep, in combat, or in crisis state
- Both are sprinting

Colonies where people eat together and work nearby have fast knowledge diffusion. Colonies where everyone works in isolation have slow diffusion. Social architecture IS knowledge architecture. The saloon isn't just a mood building — it's an information exchange.

### What People Talk About

A conversation selects a **topic** from a weighted pool based on what participants know, feel, and have recently experienced:

**Experience-driven** (highest weight — people talk about what just happened):
- "I saw duskweavers at the perimeter" → Awareness of duskweavers spreads
- "The new ore is hard to smelt" → Awareness of local metallurgy challenge
- "I nearly stepped on a glintcrawler nest" → Awareness + fear spreads

**Expertise-driven** (medium weight — people share what they know):
- The mechanic explains a principle → Listeners gain Awareness in that domain
- The farmer describes crop timing → Listeners gain Awareness of agriculture
- The doc discusses wound treatment → Listeners gain Awareness of medicine
- An Expert teaching a Familiar colonist → Structured learning, faster progression

**Emotional** (triggered by mood state):
- Stressed colonist vents → Stress can spread to listener (emotional contagion)
- Happy colonist shares enthusiasm → Small mood buff to listener
- Grieving colonist talks about the dead → Shared grief processing (both benefit)
- Scared colonist warns others → Fear spreads, but also preparedness

**Rumor** (secondhand information, lower reliability):
- "Kai said there's iron to the east" → Secondhand Awareness, accuracy degrades
- "I heard the trader is coming next week" → May be true, may be outdated
- "Someone said the hollowcall means rain" → Superstition forming

**Idle chat** (low weight, no knowledge content, builds social bonds):
- Frequency increases with friendship, decreases with rivalry

---

## Awareness Spreads Socially

The most natural mechanic: **Awareness is contagious.** If one colonist knows about metallurgy (Competent+), every colonist who spends time near them gradually becomes Aware. This happens through conversation — at meals, at the saloon, during shared work. You don't control it. People talk.

The rate depends on social contact density:

- Colony with communal meals → fast Awareness diffusion (days)
- Colony with scattered eating → slow diffusion (weeks)
- Colony with active saloon → fastest (multiple topics per evening)
- Colony with no social space → almost no passive spread

### Awareness Has Emotional Valence

What spreads, and how it makes people feel, depends on the knowledge category:

**Hopeful awareness** — "The mechanic says she can smelt iron if we find ore." Spreads with a mood buff: "Future feels possible: +2 mood." Colonists Aware of achievable-but-not-yet-achieved capabilities feel motivated. Frontier optimism.

**Fearful awareness** — "Jeb says there's something in the dark that hunts silently." Awareness of the mistmaw spreads from whoever first encounters evidence. Nervous colonists get a stress bump: "Heard about the silent hunter: +5 stress." But they also change behavior — won't walk alone at night. Fear teaches.

**Practical awareness** — "The doc says bitterbulb poisons you if you don't cook it right." Spreads and prevents colonists from eating raw bitterbulb. Awareness changes AI decisions before anyone reaches Familiar. Knowledge is protective at the lowest level.

**Forbidden awareness** — "The engineer deciphered an ancient text. She hasn't been the same since." Some deep lore creates existential dread. Colonists Aware of the planet's engineered history get a permanent minor stress modifier: "Knows too much: +3 baseline stress." But they gain access to the alien tech domain. The cost of understanding is psychological weight.

**Contested awareness** — Two colonists disagree. "The thermogast won't cross water" vs. "I saw one wade through the creek." Both spread their version. Which sticks depends on speaker authority (Expert > Familiar), social dynamics (who has more friends), and eventual testing. Misinformation and correction as emergent social process.

---

## Personality Shapes Information Flow

Colonist personality traits determine how they participate in the knowledge network:

**Extroverts** initiate conversations frequently. They're the colony's information hubs — knowledge flows through them. An extrovert who knows something ensures the whole colony knows within days. But they also spread fear and rumors faster. Losing an extrovert Expert is doubly costly: the knowledge AND the distribution channel both die.

**Introverts** rarely initiate. They listen more than they talk. They accumulate knowledge from overhearing but don't share it. An introvert who discovers something critical might not mention it for days — until specifically asked, or until it comes up in a rare conversation. Critical knowledge can get stuck in an introvert. The player who notices this assigns them to shared workstations or campfire evenings.

**Leaders** (existing `is_leader` trait) attract listeners. When a leader speaks, nearby colonists stop and listen — higher Awareness transfer rate. A leader sharing optimism has outsized mood effect. A leader sharing fear has outsized stress effect. Leaders are amplifiers.

**Contrarians** argue with whatever's being said. They slow consensus but occasionally catch errors in group thinking. A contrarian who argues against a false rumor prevents bad information from solidifying. Annoying but valuable.

**Storytellers** (new trait) enhance emotional impact of any topic. Their version of events is more memorable — knowledge they share has higher retention. But their version is also more dramatized — accuracy drops slightly as narrative quality rises. They're the colony's bards: entertaining, influential, not always reliable.

**Suspicious** colonists discount information from strangers, newcomers, and prisoners. Slow to adopt new Awareness but rarely fooled by lies. Paired with the Liar system (below), they're the colony's immune system against misinformation.

---

## Information Veracity — Not Everything Is True

Every piece of shared information has a hidden veracity level the listener can't see:

| Veracity | Meaning | Source |
|----------|---------|--------|
| **True** | Matches game state | Direct Expert observation, verified experiment |
| **Approximately true** | Directionally correct, imprecise | Familiar-level observation, brief sighting |
| **Outdated** | Was true, no longer | Old observation, pre-drought water sources |
| **Mistaken** | Speaker genuinely believes it, but wrong | Learning gaps, bad source material, incomplete observation |
| **Exaggerated** | Core truth inflated | Fear, storytelling personality, secondhand retelling |
| **Fabricated** | Deliberately false | Enemy deception, manipulative prisoners, lying newcomers |

The listener sees: "Kai says there are duskweavers at the perimeter." They don't see the veracity tag. They decide whether to trust it based on who Kai is, how she seems, and whether it matches their own experience.

### The Telephone Effect

Information degrades through retelling. Each handoff introduces drift:

1. Kai (Expert, direct observation): "I saw 5 duskweavers near the south wall at dusk."
2. Kai tells Jeb: "A pack of duskweavers near the south wall." (Count lost — Jeb is Aware, not Expert)
3. Jeb tells the dinner table: "A big pack south of us." (Direction vague, "big" is editorializing)
4. Marcus tells the night watch: "Swarm of creatures coming from the south." (Species uncertain, threat inflated)

By the fourth retelling, the information is Exaggerated. The colony is more scared than the situation warrants. But they're also more prepared. The distortion has a survival function — overreacting to threats is safer than underreacting.

---

## Lies and Deception

The most exciting possibility: deliberate falsehood entering the knowledge system.

### Enemy Prisoners

A captured raider is a potential information source — and a potential information weapon. What they share depends on:

- **Disposition** (hostile / neutral / cooperative) — shifts with treatment over time
- **Deception skill** (hidden stat) — how convincingly they lie
- **Intent** — hostile prisoners lie strategically; cooperative ones mostly share truth

**A hostile prisoner says:** "There are 50 of us in the hills." (Actually 12.) The information enters the social system. Colonists who hear about it get stressed. Resources get diverted to defense. A scouting expedition is the only way to verify — risky, but necessary.

**A manipulative prisoner says:** "I know about a water spring two days east. I can show you." Is it a trap? An ambush? Or genuinely useful? The veracity is invisible. The player assesses risk.

**Modeling deception:**
```
truth_chance = base_cooperation - deception_skill + trust_built_over_time

Hostile prisoner, high deception, day 1:  truth_chance ≈ 15%
Neutral prisoner, moderate deception, day 5:  truth_chance ≈ 50%  
Cooperative prisoner, low deception, day 10:  truth_chance ≈ 85%
```

Even cooperative prisoners might withhold information. Even hostile ones occasionally tell truth (to build credibility for a bigger lie later). The player can never be 100% certain.

### Lying Newcomers

A stranger arrives claiming to be an Expert blacksmith. They're actually Familiar at best. Their skills in the UI initially show what they CLAIM, not what they ARE.

Over time, the truth emerges: they fumble tasks an Expert wouldn't struggle with. The colony notices. A perceptive colonist (high social skill) detects the discrepancy sooner: "Something's off about the new smith. She didn't recognize basic flux technique."

**Modeling this:** Newcomers have `claimed_skills[]` and `actual_skills[]`. The UI initially shows claimed. Each work attempt has a chance to reveal actual level: `reveal_chance = |claimed - actual| × 0.1 per attempt`. Larger lies are exposed faster. A colonist claiming Expert when they're Familiar is caught within days. A colonist claiming Competent when they're high-Familiar might pass for weeks.

Not all lying newcomers are malicious. Some exaggerate out of desperation (want to be accepted), fear (don't want to seem useless), or genuine self-delusion (think they're better than they are). The REASON matters for how the colony responds — an exposed liar who was desperate gets sympathy; one who was manipulative gets distrust.

### Enemy Radio Disinformation

If you intercept coded enemy communications (radio system from the-human-layer.md), the information might be deliberately planted. Enemies who know you have a radio might broadcast:
- False troop movements
- Fake supply routes (ambush sites)
- Phony calls for surrender
- Inflated force estimates

The cipher key decodes the message but doesn't verify it. Intelligence is information with unknown veracity. Acting on false intelligence wastes resources or walks into traps. Ignoring intelligence might miss a genuine warning.

### Trader Deception

A trader claims the alien fragment they're selling is authentic. Maybe it is. Maybe it's worthless rock. Only a colonist with Expert alien-tech knowledge can appraise accurately. Without that colonist, you buy on trust. The item's true properties are hidden until examined by a specialist.

Similarly: "This seed grows a drought-resistant crop." Does it? You won't know until you plant it and wait a season. If the trader lied, you wasted a growing season on a useless crop. Trust in specific traders builds or erodes over multiple visits based on whether their claims held up.

### Misinformation Cascades

A single lie can cascade through the social system:

1. Captured raider tells Jeb: "Our leader has a weapon that shoots lightning." (Fabricated.)
2. Jeb, stressed, tells everyone at dinner. (Rumor spreads as Fearful Awareness.)
3. Colony becomes Aware of "enemy lightning weapon." Stress colony-wide.
4. Resources diverted to insulated walls and lightning defense. (Wasted effort.)
5. Scout encounters Redskulls. Regular rifles. (Truth revealed.)
6. Colony mood: relief + anger at prisoner + lasting distrust of future prisoner claims.

Or the inverse — a truth that's dismissed as a lie:

1. Cooperative prisoner says: "Healing plant grows near our camp." (True.)
2. Contrarian colonist argues: "Why would they help us? It's a trap." (Reasonable skepticism.)
3. Colony debates at the saloon. Player decides: trust or not?
4. If ignored: a genuine resource is missed. If acted on: colony gains medicine AND increased trust in that prisoner.

Both cascades emerge from the same system. Neither is scripted.

---

## Information Verification

The colony needs ways to check whether shared information is true:

- **Direct observation:** Send a scout to verify. See for yourself. Costly but definitive.
- **Expertise check:** Ask the domain Expert to evaluate a claim. "The mechanic says that technique sounds wrong." Faster but requires having the Expert.
- **Experiment:** Test the claim in controlled conditions. If it works, confirmed. If it fails, refuted. Costs materials.
- **Consensus:** Multiple independent sources agree → higher confidence. Two scouts both seeing the camp is stronger than one.
- **Track record:** Colonists earn an implicit trust score. High-trust colonists' information spreads faster and is believed more. Low-trust colonists are discounted. Trust is social currency, earned through accuracy.

A **Suspicious** colonist automatically demands verification for claims from low-trust sources. Annoying when information is true. Lifesaving when it isn't.

---

## Randomness and Replayability

The knowledge system creates unique playthroughs through several randomness layers:

### Per-Map Calibration Drift

Physical constants vary slightly by map seed. Not wildly — same order of magnitude — but enough that knowledge needs local calibration:

- **Bitterbulb denaturation temperature:** 75–90°C (varies by map). A traded recipe says "82°C" — useful starting point, but THIS map's variant might need 87°C.
- **Duskweaver flee radius:** 4–8 tiles of torchlight. Varies by temperature — bolder on cold nights. The "Duskweaver Notes" are accurate for the conditions they were observed under.
- **Iron ore purity:** Varies by deposit. One vein smelts clean. Another has sulfur impurities requiring flux (a sub-technique discovered through experimentation or trade).

**Effect on veterans:** A veteran player knows the GENERAL shape (fire before smelting, cooking before preservation). But they don't know the SPECIFICS of this run. Meta-knowledge makes them Aware, but they still calibrate. The game always has something to teach.

### Hidden Learning Aptitudes

Every colonist has hidden learning affinities, rolled at creation but invisible until they attempt the skill. Kai might be a natural smith (learns metallurgy 2x faster). Or she might struggle with it but have a gift for agriculture nobody anticipated.

These are never shown in the UI. Discovered through play: "Kai seems to be picking up smelting faster than Jeb." A thought bubble: "This feels natural to me" or "I don't think I'm cut out for this."

**Effect on replayability:** Two runs with the same starting crew diverge based on who's good at what. Run 1: Kai is your smith. Run 2: Kai is terrible at smithing but an amazing farmer. Same colonist, different story.

### Eureka Moments

A colonist practicing at Familiar or Competent level has a small random chance per attempt of a breakthrough — a sudden insight that advances their knowledge faster than normal, or discovers a novel recipe.

The cook who accidentally burns bitterbulb stew discovers that charred bitterbulb mixed with sap makes a potent adhesive. Nobody planned this. It emerged from a failed cooking attempt + a random roll + specific materials present. This recipe is unique to THIS colony — discoverable only through this particular accident.

```
Eureka chance per work attempt:
  Familiar:   1%
  Competent:  3%
  Expert:     5%
```

Can't be farmed — happens during real work as a side effect of normal activity. Each eureka is a gift from the RNG that makes this colony's knowledge unique.

### Seasonal Knowledge

Some things can only be learned at certain times. You learn about spring flooding by experiencing spring flooding. You learn about winter crop failure by failing in winter. Char-cap underground farming is only discovered if someone tries growing fungus underground during the cold season.

A colony that played it safe in year 1 enters year 2 with narrow knowledge. A colony that experimented aggressively enters year 2 with broad-but-shallow knowledge. Both viable, different risk/reward.

### Knowledge Forgetting

A colonist Competent at smelting who hasn't smelted in 3 months degrades toward Familiar. Use it or lose it. The colony must keep practicing skills even without urgent need — maintaining capability has a labor cost. This creates a reason to occasionally smelt iron even when you have enough: keeping the skill alive.

Degradation is slow and only affects the practice component. A colonist who learned from books retains theoretical understanding longer than muscle memory. An Expert who stops practicing might drop to Competent in theory but Familiar in execution speed — they remember what to do but their hands are rusty.

---

## The Chat Bubble System

Conversations are visible but not text-heavy. Colonists show small icon bubbles representing the category of discussion:

**Bubble icons** (simple glyphs, not text):
- Creature topic: small claw/fang shape
- Tool/craft topic: small hammer shape
- Food topic: small plant shape
- Danger/fear topic: exclamation shape
- Social/idle: speech shape
- Teaching: book/scroll shape
- Argument: crossed lines
- Laughter: small curve (smile)

**Bubble tinting** by emotional valence:
- Neutral: default text color
- Positive: warm tint (hopeful awareness, laughter, teaching)
- Negative: cool/red tint (fear, argument, stress)
- Question: faded (uncertainty, rumor, "I heard...")

The player reads the colony's social state at a glance: two colonists by the fire with creature-topic bubbles, a cluster at the saloon with tool-topic bubbles and laughter indicators. No text needed.

**Clicking a conversation** shows the log detail: "Kai told Jeb about the duskweaver pack. Jeb is now Aware of duskweavers. Jeb seems nervous." Available on demand, doesn't clutter the screen.

**Sound sim integration:** Louder conversations (arguments, excited teaching, leader speeches) propagate further through the sound sim. Eavesdroppers within range get partial information — they hear the topic but not the details. A colonist in the next room hears an argument through the wall (muffled) and gets a stress bump from the emotional tone without knowing what it's about.

**Whispers:** Low-amplitude conversations between two colonists with high mutual trust. Barely audible in the sound sim. Other colonists can't overhear unless adjacent. Used for: conspiracies, private concerns, sensitive information sharing. The player notices two colonists whispering but can't see the topic bubble unless they click. Suspicion mechanic: other colonists who see frequent whispering between two people become curious or suspicious (personality-dependent).

---

## Knowledge Level Transitions

How colonists move between levels, and what accelerates or blocks each transition:

### Unaware → Aware (Fast — just exposure)

This is the transition that spreads socially. Mechanisms:
- Overhear a conversation about the topic (saloon, shared meals)
- A trader mentions it in passing
- See someone else do it (even briefly — watching the mechanic smelt for 10 seconds)
- Find a reference in a ruin or artifact
- Radio transmission mentions the concept
- Awareness of a colonist's death spreads the knowledge of what killed them

**Speed modifiers:** Extroverts spread Awareness ~2x faster. Leaders ~3x. Storytellers give Awareness higher emotional weight (stickier). Introverts accumulate Awareness normally from listening, but rarely CAUSE it in others.

### Aware → Familiar (Moderate — requires engagement)

Awareness isn't enough to attempt the task. Familiar requires deliberate learning:
- Read a lore item about it (library, takes hours)
- Watch an Expert demonstrate it over an extended period (apprenticeship, full work shift)
- Attend a teaching session (campfire evening lessons — Expert addresses all present listeners)
- Study an artifact related to the domain
- Have it explained one-on-one by someone Competent+ (slower than Expert teaching)

**The apprenticeship mechanic:** Assign a Familiar colonist to work alongside a Competent+ colonist at the same station. The Familiar colonist works slower but gains experience. The Competent+ colonist works slightly slower (they're teaching). Over days, the Familiar colonist progresses. The Expert teaching modifier is ~2x faster than Competent.

### Familiar → Competent (Slow — requires doing)

The critical gap. **Only hands-on practice crosses this threshold.** Books and teaching get you Familiar. Competence is earned by doing the work, repeatedly, with real failure costs.

- Each work attempt (success or failure) grants experience
- Failures grant MORE experience than successes (you learn more from mistakes)
- Working alongside a Competent+ colonist reduces failure rate but doesn't speed the transition
- Natural aptitude (hidden) affects speed: some colonists cross in days, others take weeks
- Timeframe: typically 5-20 work sessions depending on domain complexity and aptitude

### Competent → Expert (Very slow — sustained investment)

Months of regular practice plus encountering variety:
- Smelting different ores teaches more than smelting the same one
- Treating different injuries teaches more than bandaging the same wound
- Growing different crops in different seasons teaches more than monoculture
- Teaching others deepens understanding (the teacher learns by teaching)
- Natural aptitude matters — some plateau at Competent permanently

Some colonists never reach Expert. That's fine. Most domains are serviced by Competent colonists. Experts are valuable BECAUSE they're uncommon.

### Expert → Master (Rare — talent + time + innovation)

Years of practice. Successfully innovating (discovering new techniques through experimentation). Teaching widely (the master teacher transcends their own skill). Only colonists with both high domain aptitude AND the right personality can reach Master. A colony might never have a Master in any domain. When they do, that person becomes legendary.

---

## Knowledge Dependencies

Some domains require minimum levels in prerequisites before learning can even begin:

```
Fire making          → no prereqs (innate)
Basic construction   → no prereqs (innate)
Woodworking          → Fire making ≥ Competent (need to harden wood, shape with heat)
Clay working         → Fire making ≥ Competent (kiln requires fire mastery)
Cooking              → Fire making ≥ Familiar (you at least need to light a fire)
Hide curing          → Cooking ≥ Aware (basic understanding of heat + chemical processes)
Metallurgy (smelting)  → Fire making ≥ Expert (need precise heat control)
Metallurgy (forging)   → Metallurgy (smelting) ≥ Familiar
Chemistry (basic)      → Cooking ≥ Familiar (understanding heat + reactions)
Chemistry (gunpowder)  → Chemistry ≥ Familiar AND Fire making ≥ Competent
Fermentation           → Cooking ≥ Aware (basic understanding of biological processes)
Agriculture (advanced) → Agriculture (basic) ≥ Competent + survived 1 full seasonal cycle
Glass making           → Fire making ≥ Expert AND Clay working ≥ Familiar
Alien tech (basic)     → Construction ≥ Familiar (recognize structural patterns)
Alien tech (power)     → Alien tech (basic) ≥ Familiar AND any electrical knowledge ≥ Aware
Alien tech (biology)   → Alien tech (basic) ≥ Familiar AND Medicine ≥ Aware
```

Dependencies create natural chains that FEEL like a tech tree but are embodied in people. The mechanic (Expert fire, Familiar alien tech) can start learning alien power immediately. The farmer (unaware of electrical systems) needs to first learn basic power before alien tech even registers as a concept.

**Dependencies gate the Aware transition, not higher levels.** You can't even START becoming Aware of metallurgy unless your fire skills are Expert. But once Aware, progression to Familiar/Competent follows the normal learning mechanisms.

---

## Colony Knowledge View

The player's aggregate view of what the colony knows:

**Per-domain bar** showing the highest level any colonist has reached, plus how many colonists are at each level. "Metallurgy: Expert (1), Competent (2), Familiar (3), Aware (8)." At a glance: one expert, backed by two competent workers, and most people at least know what it is.

**Vulnerability flags:** "Single point of failure: Kai is the only colonist Competent+ in metallurgy." The player sees which knowledge areas are fragile — one death away from losing the capability. This drives teaching and apprenticeship decisions.

**Knowledge gap alerts:** "No one in the colony is Aware of medicine." The player sees blind spots — domains where even Awareness hasn't penetrated. These gaps are trade targets or exploration priorities.

---

## Knowledge Texture

Beyond level, knowledge has properties that make each instance unique:

- **Accuracy:** How correct is their understanding? An Expert with 95% accuracy vs. a Familiar at 70%. Inaccurate knowledge produces occasional errors even at Competent level.
- **Breadth:** Do they know just iron, or also copper and alloys? Narrow expertise vs. general knowledge within a domain.
- **Source:** Practical (learned by doing — fast hands, weak theory) vs. Theoretical (learned from books — understands why, slow hands). Best practitioners have both.
- **Local calibration:** Have they adapted to THIS map's specific parameters? A traded book gives general knowledge; local practice gives calibrated knowledge.
- **Confidence:** How sure are they? High-confidence colonists work faster but catch fewer errors. Low-confidence colonists are slower but more careful. Overconfidence after early successes can lead to costly mistakes.

Two colonists both "Competent at metallurgy" can be very different: one is a practical smith (fast, narrow, locally calibrated) and the other is a bookish metallurgist (slow, broad, uncalibrated). Together they're better than either alone — collaborative knowledge compounding.

---

## Buffs and Debuffs from Social Knowledge

### Colony-wide effects from aggregate knowledge state

| Condition | Effect |
|-----------|--------|
| Expert in a domain teaching regularly | Colony mood: "Learning culture: +3" |
| Multiple competing beliefs unresolved | Colony mood: "Uncertain times: -2" |
| Library with 10+ lore items | Colony mood: "Accumulated wisdom: +2" |
| No one Competent in medicine | Colony mood: "No doctor: -5" (scaling with injuries) |
| Misinformation recently exposed | Colony mood: "Trust shaken: -3" for 5 days |
| Eureka discovery | Colony mood: "Breakthrough!: +5" for 3 days |
| Knowledge lost to death | Colony mood: "We lost so much: -8" scaling with level lost |

### Individual effects from knowledge transitions

| Event | Effect on individual |
|-------|---------------------|
| Becoming Aware of a hopeful capability | +2 mood for 2 days |
| Becoming Aware of a threat | +3–8 stress (scales with threat severity) |
| Reaching Competent through practice | +5 mood for 3 days ("I can do this now") |
| Reaching Expert | +8 mood for 5 days, permanent +1 baseline mood in that domain's workspace |
| Failed experiment (Familiar attempting) | -2 mood, but +experience toward Competent |
| Teaching someone successfully | +3 mood for teacher, +2 for student |
| Discovering someone lied to you | -5 mood, permanent trust reduction toward that person |
| Having your expertise questioned by a contrarian | -2 mood (or +1 if the contrarian was right — humbling but educational) |

---

## Connection to Existing Systems

| System | Integration |
|--------|------------|
| **Sound sim** | Conversations are sound sources. Louder topics propagate further. Whispers are low-amplitude. Eavesdropping is physical. |
| **Saloon** (the-human-layer.md) | Primary social mixing space. Layout affects conversation frequency and audience size. |
| **Campfire** (philosophy.md) | Evening teaching sessions. Expert addresses all within sound range. |
| **Combat** (DN-011, combat.md) | Prisoner interrogation. Enemy deception. Combat experience spreads fear/respect awareness. |
| **Creatures** (alien-fauna.md) | Creature behavior knowledge spreads through observation reports and social retelling. |
| **Food** (food-and-survival.md) | Cooking knowledge with real failure consequences. Recipe accuracy matters. |
| **Equipment** (DN-018) | Tool proficiency tied to knowledge level. Can't effectively use what you don't understand. |
| **Lore/Research** (lore-and-research.md) | Library stores written knowledge. Books bridge tacit→shared. This doc adds the social transfer mechanism. |
| **Radio** (the-human-layer.md) | Information source with unknown veracity. Disinformation vector. |
| **Trade** (gameplay-systems.md) | Traders bring knowledge AND potential lies. Trust earned over multiple visits. |

---

## Future Documentation: The Lore-Craft Integration

This doc, lore-and-research.md, and the crafting chain need to converge into a single authoritative reference. That document doesn't exist yet and shouldn't be written until the systems described here are more settled. When it's time, it should be a **design note** (DN-format), not an ideas doc, because it defines concrete data structures and recipes. Proposed scope:

**DN-0XX: Knowledge-Gated Crafting** should contain:

1. **The `knowledge` field on recipes** — how recipes.toml specifies which knowledge domain and minimum level is required. How the check works (library shelf OR colonist tacit knowledge).

2. **Full knowledge domain registry** — every domain, its prerequisites (as minimum knowledge levels in other domains), its discovery methods, and the specific recipes it gates. This is the "landscape map" — not a tree, but a web showing all paths between domains.

3. **Material chains per domain** — where rare materials come from (map features, creatures, ruins, trade) and which domain they feed. Iron ore → metallurgy. Alien fragments → alien tech. Ridgeback hide → leatherwork. The material source IS the progression gate alongside knowledge.

4. **Infrastructure chain** — which crafting stations require knowledge to BUILD (not just to USE). You can't build a smelter if nobody knows smelting. The station itself is a knowledge-gated recipe. Circular dependency resolved by: the first smelter is built from a blueprint card found in ruins, OR by an Expert fire-maker who figures it out through experimentation.

5. **The calibration tables** — per-map variation ranges for key physical parameters. Bitterbulb denaturation: 75–90°C. Iron smelting temperature: 800–1100°C. Duskweaver flee radius: 4–8 tiles. These are the numbers that make each playthrough require local adaptation of general knowledge.

6. **Recipe veracity** — how traded/found recipes might be inaccurate for local conditions, and how colonists calibrate through practice. A recipe from another settlement is Approximately True — it works, but local optimization improves yield/quality.

This DN should be written after the knowledge gradient (this doc), the social spreading mechanism (this doc), and the existing crafting chain (crafting.md, crafting-tree.md) are all stable. It's the integration layer that ties them together into one coherent progression system. Until then, the three docs serve as the design intent.

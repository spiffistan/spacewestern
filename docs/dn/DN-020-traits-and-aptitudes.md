# DN-020: Traits, Aptitudes, and Hidden Potential

**Status:** Draft (sparring doc)
**Depends on:** DN-019 (knowledge system), DN-018 (equipment)
**Related:** ideas/chargen.md (Manifest system)

## The Problem with Rimworld/ONI Skills

In Rimworld, skills are numbers that go up. Construction 12 is strictly better than Construction 8. Every pawn is a spreadsheet of skill bars, and the optimal play is always "assign the highest skill to the matching job." It's solved at a glance. ONI adds learning speed but it's the same: higher number = better.

This is fine for those games. But it makes people feel like machines with stat cards. We want people who feel like *people* — surprising, contradictory, occasionally brilliant, occasionally terrible.

## Design: Three Layers of Character

### Layer 1: Traits (3-5 per person, visible)

**Data-driven** (traits.toml), declarative modifiers. What you see on the character sheet.

Each person has 3-5 traits from categories:
- 1-2 **physical** (body, health, stamina)
- 1 **mental** (learning, focus, creativity)
- 1 **temperament** (social, emotional, stress)
- 0-1 **quirk** (unique, sometimes negative, sometimes double-edged)

Traits are **fixed at creation** — they don't change. They're who you ARE.

**Mutual exclusion:** within each category, some pairs conflict:
- Weathered ↔ Frail
- Quick Learner ↔ Slow
- Steady Nerve ↔ Volatile
- Iron Gut ↔ Gourmand

**Positive, negative, double-edged:**

| Type | Examples | Design purpose |
|------|---------|----------------|
| Positive | Weathered, Deadeye, Green Thumb | Make the person good at something specific |
| Negative | Lazy, Clumsy, Volatile, Pyromaniac | Create interesting problems, not just "worse" |
| Double-edged | Perfectionist (-30% speed, +quality), Neurotic (+25% speed, +50% stress), Loner (faster alone, stressed in groups) | The most interesting: every double-edged trait is a playstyle choice |

**Key difference from Rimworld:** Negative traits aren't just penalties. A Pyromaniac who starts fires during mental breaks is a STORY. A Volatile colonist who explodes under pressure but is brilliant in calm moments is a CHARACTER. Traits create dramatic situations, not just stat modifiers.

### Layer 2: Aptitudes (Hidden, per knowledge domain)

This is the divergence from Rimworld. Instead of visible skill levels that just go up, each person has **hidden aptitudes** per DN-019 knowledge domain:

- **Aptitude: -2 to +3** (hidden from player)
- Determines learning speed AND skill ceiling
- A person with Metallurgy aptitude +3 learns smelting 3x faster than aptitude 0
- A person with Metallurgy aptitude -1 learns slowly and caps at Competent (never reaches Expert)
- **Not visible until tested.** You don't know if Cole is a natural blacksmith until he tries smelting

**Discovery moments:** When a colonist first attempts a new domain, there's a chance of an aptitude reveal:
- First successful craft: "Cole seems to have a natural feel for metalwork" (+2 aptitude revealed)
- First major failure: "Mara struggled badly with the forge. This might not be her strength." (-1 revealed)
- Repeated practice: gradual reveal over time — learning speed tells you aptitude indirectly

**Why this matters:** In Rimworld, you look at the skills tab and assign jobs. Done. Here, you don't KNOW who your best metalworker is until someone tries. The Scout might have hidden genius-level metallurgy aptitude. The Engineer might be terrible at it despite their backstory suggesting otherwise. **You discover your people by working with them.**

### Layer 3: The Hidden Trait (1 per person, revealed by events)

Each person has **one hidden trait** that's unknown at game start. It activates when a specific triggering condition occurs:

| Hidden Trait | Trigger | Effect | Discovery Text |
|-------------|---------|--------|----------------|
| **Coward** | First combat | Flee radius 2x, won't stand ground | "Jeff froze under fire. He's not who we thought." |
| **Born Leader** | 3+ people in crisis | Rally aura activates automatically | "Something changed in Mara. She started giving orders and everyone listened." |
| **Deft Hands** | First successful craft | Crafting speed permanent +30% | "Cole's hands moved like they knew the work already." |
| **Soft Heart** | Ally wounded nearby | Mood crash, but +50% medical speed | "Jeff couldn't stop staring at the blood. Then he started stitching." |
| **Night Eyes** | First night outdoors | No darkness penalty, +perception at night | "Mara noticed the duskweaver before anyone else." |
| **Green Sense** | First crop planted | Crops in 5-tile radius grow 20% faster | "The soil around Cole's garden just... cooperated." |
| **Iron Will** | Stress > 90% survived | Permanent stress cap at 80 (never breaks) | "Something hardened in Jeff. He won't break again." |
| **Grudge Keeper** | Ally killed | +50% damage to creature that killed them, permanent | "Mara never forgot. Never forgave." |
| **Lucky** | Random (5% per day) | Occasional lucky events: find extra resources, dodge bullets | "Cole just... found it. Again. He always finds things." |
| **Unlucky** | Random (5% per day) | Occasional bad events: trip, drop items, attract duskweavers | "If it can go wrong near Jeff, it will." |

The hidden trait creates a **narrative moment.** The player remembers "the time Cole revealed he was a natural craftsman" or "when Mara became the leader during the crisis." These moments don't happen in Rimworld because everything is visible from day one.

**UI representation:**
- Before reveal: the trait slot shows "?" with a faint shimmer
- After reveal: dramatic popup, the trait fills in, pleb gets a thought bubble about it
- Permanently visible from then on

## How This Diverges from Rimworld

| Rimworld | Rayworld |
|----------|----------|
| All skills visible at recruitment | Aptitudes hidden until tested |
| Skills are numbers 0-20 | Knowledge is a 6-level gradient with texture |
| Traits are random stat modifiers | Traits create dramatic situations |
| All information upfront | Hidden trait creates discovery moments |
| Optimal assignment is obvious | You have to experiment with your people |
| People feel like stat blocks | People feel like people with secrets |

## Skills vs Knowledge (the rename)

The current "skills" array (shooting, melee, crafting, farming, medical, construction) should become the **starting knowledge levels** from DN-019. Instead of `skills: [u8; 6]` that just multiply speed, each index maps to a knowledge domain with the full 6-level gradient:

- skills[0] (shooting) → Combat domain knowledge
- skills[1] (melee) → Combat domain knowledge (melee sub-specialization)
- skills[2] (crafting) → Woodworking/Clay working knowledge
- skills[3] (farming) → Agriculture knowledge
- skills[4] (medical) → Medicine knowledge
- skills[5] (construction) → Basic construction knowledge

This means a "Crafting 8" pleb isn't just 30% faster — they're an Expert who can experiment, teach others, and attempt advanced recipes that Familiar-level plebs would fail at.

## Implementation Phases

### Phase 1 (Now): Wire existing traits + add data-driven loading
- traits.toml with modifier declarations
- Generic `pleb.stat("stat_name")` resolution
- Mutual exclusion in chargen
- Hidden trait slot on Pleb (revealed by triggers)

### Phase 2: Aptitude system
- Hidden aptitude values per domain (-2 to +3)
- Learning speed modifiers
- Aptitude reveal events
- "?" display in character sheet for untested domains

### Phase 3: Full DN-019 knowledge integration
- Replace skills array with knowledge domain levels
- Social knowledge transfer
- Knowledge decay
- Three-lock crafting

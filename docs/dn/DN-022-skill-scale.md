# DN-022: Skill Scale and Progression Curve

**Status:** Draft
**Depends on:** DN-019 (knowledge domains), DN-020 (traits and aptitudes)
**Related:** ideas/chargen.md

## Problem

Rimworld's 0-20 skill scale trivializes progression. By year 2, colonists are at 15+ in their assigned job. The number goes up, the player optimizes once, done. Skills become noise. ONI adds learning speed but it's the same: higher number = better, everyone eventually maxes.

We want skills that feel meaningful at EVERY level, where mastery is rare and earned, and where a colony of "pretty good" workers is the norm — not a stepping stone to god-tier.

## The Scale: 0.0 – 10.0

A continuous float displayed to one decimal place. Everyone intuitively understands a 10-point scale. "She's a 7.3 in metalwork" communicates instantly.

### Level Descriptors

| Range | Descriptor | What it means |
|-------|-----------|---------------|
| 0.0–1.0 | Novice | Can attempt. ~60% failure rate. Slow. Wastes materials. |
| 1.0–2.5 | Beginner | Learning. ~40% failure. Understands basics. |
| 2.5–4.0 | Apprentice | Improving. ~20% failure. Needs supervision. |
| 4.0–5.5 | Journeyman | Reliable. ~5% failure. Normal speed. The working baseline. |
| 5.5–7.0 | Skilled | Good. Rarely fails. +15% speed. Can work unsupervised. |
| 7.0–8.0 | Proficient | Very good. +25% speed. Can teach others. |
| 8.0–9.0 | Expert | Exceptional. +40% speed. Can experiment and innovate. |
| 9.0–9.5 | Master | Rare. +60% speed. Masterwork quality. The realistic peak. |
| 9.5–9.8 | Grandmaster | Extraordinary. Maybe once per long game with perfect aptitude. |
| 9.8–10.0 | Legendary | The rarest achievement in the game. Not impossible — just nearly so. |

**10.0 is reachable, but barely.** It requires a +3 aptitude genius, years of dedicated practice, and a bit of luck. In practical terms: maybe 1 in 300 games produces a 10.0. When it happens, it's an event — the game should acknowledge it. A colonist reaching 10.0 is the equivalent of Shawshank hitting 9.3 on IMDB — proof the scale actually goes there, a moment the player remembers forever.

### Mapping to DN-019 Knowledge Gradient

| Knowledge Level | Skill Range | Transition |
|----------------|------------|------------|
| Unaware | 0.0–0.5 | Can't see related resources or tasks |
| Aware | 0.5–2.0 | Recognizes concepts, can mark for experts |
| Familiar | 2.0–4.0 | Can attempt with high failure rate |
| Competent | 4.0–6.5 | Reliable execution, normal speed |
| Expert | 6.5–8.5 | Teaching, experimentation, +speed |
| Master | 8.5–10.0 | Masterwork, invention, signature style |

## XP Curve: Exponential Cost

Each level costs double the previous one:

```
xp_to_next_level = BASE_XP * 2^floor(level)
```

| Level | XP for this level | Cumulative | ~Game time | % of total |
|-------|-------------------|-----------|------------|------------|
| 0→1 | 100 | 100 | Hours | 0.1% |
| 1→2 | 200 | 300 | Hours | 0.3% |
| 2→3 | 400 | 700 | ~1 day | 0.7% |
| 3→4 | 800 | 1,500 | Days | 1.5% |
| 4→5 | 1,600 | 3,100 | ~1 week | 3.0% |
| 5→6 | 3,200 | 6,300 | Weeks | 6.2% |
| 6→7 | 6,400 | 12,700 | ~1 month | 12.4% |
| 7→8 | 12,800 | 25,500 | Months | 24.9% |
| 8→9 | 25,600 | 51,100 | A season | — |
| 9→9.5 | ~40,000 | ~91,000 | Months more | — |
| 9.5→9.8 | ~150,000 | ~241,000 | Effectively forever | — |

Beyond 9.0, an additional **asymptotic multiplier** kicks in:

```
asymptotic_cost = 1.0 + (level / 10.0)^8 * 500.0
```

This is steep but finite. At 10.0 the cost is 501x, not infinity. A +3 aptitude genius doing nothing but one domain for 2+ in-game years CAN reach 10.0 — it's just so unlikely that when it happens, the game should pop confetti.

| Level | Cost Multiplier | What this means |
|-------|----------------|-----------------|
| 7.0 | 1.1x | Barely noticeable |
| 8.0 | 1.4x | Slightly harder |
| 9.0 | 1.7x | Noticeably slowing |
| 9.5 | 3.6x | Extraordinary effort |
| 9.8 | 16x | A lifetime of dedication |
| 9.9 | 54x | Once in a hundred games |
| 9.95 | 130x | Once in three hundred games |
| 10.0 | 500x | The rarest thing in the game. It can happen. |

**Key insight:** Reaching 5.0 (functional worker) takes hours. Reaching 7.0 (genuinely good) takes weeks. Reaching 9.0 (Master) takes a season. Reaching 9.5 takes months more on top. Reaching 9.8 is effectively the lifetime of a game.

**The vast majority of gameplay happens in the 4-7 range, where every decimal point feels earned.** A colony full of 5s and 6s is a good colony. A 7 is your best worker. An 8 is remarkable. A 9 is the talk of the colony. And 10.0 is like IMDB 10.0 — the scale allows it, but nothing ever gets there.

## Aptitude: The Hidden Ceiling

Each colonist has a hidden aptitude per domain: -2 to +3 (integer, set at creation, never changes).

Aptitude creates a **soft cap** — an asymptotic slowdown, not a hard wall:

```
soft_cap = 7.0 + aptitude * 1.0
xp_gain_multiplier = max(0.0, 1.0 - (level / soft_cap)^4)
```

| Aptitude | Soft Cap | Practical Max | Who this person is |
|----------|----------|---------------|-------------------|
| -2 | 5.0 | ~4.5 | Can do the work, never great at it |
| -1 | 6.0 | ~5.5 | Solid contributor, not a specialist |
| 0 | 7.0 | ~6.5 | Good worker, reliable, hits a natural ceiling |
| +1 | 8.0 | ~7.5 | Talented, can become genuinely expert |
| +2 | 9.0 | ~8.5 | Gifted, master-level achievable with dedication |
| +3 | 10.0 | ~9.0-9.3 | Genius, Master achievable with years of dedication |

### How the ceiling feels

At aptitude 0, approaching 7.0:
- Level 5.0→5.5: feels normal, steady progress
- Level 5.5→6.0: noticeably slower, but still moving
- Level 6.0→6.5: slow. A month of dedicated work for 0.5 gain.
- Level 6.5→6.8: very slow. Diminishing returns obvious.
- Level 6.8→7.0: glacial. Possible but the player naturally stops pushing.

This feels like "this person has reached their potential" rather than "the game won't let me level up." The player doesn't see a wall — they see a person who's maxed out what they can do. Realistic. Humane.

### Aptitude discovery

Aptitude is hidden. The player discovers it through:

1. **Learning speed** — a +3 aptitude person visibly improves faster. "Cole is picking this up fast."
2. **Reveal events** — first success/failure in a new domain can trigger: "Mara seems to have a natural feel for stonework" (+2 revealed)
3. **Plateau observation** — when progress slows, the player infers the ceiling. No explicit number shown unless the aptitude has been revealed.

**UI representation:**
- Untested domain: skill shows as "—" with "?" aptitude
- Tested, aptitude unknown: skill shows as number, aptitude still "?"
- Aptitude revealed: small star icons (1-5 mapping to -2 through +3), or a descriptor ("Natural", "Limited", "Gifted")

## XP Sources

XP is earned by DOING. There's no generic "learning" activity.

| Action | XP Amount | Scaling |
|--------|----------|---------|
| Complete a craft | Base recipe XP | ×1.5 on failure (learn from mistakes) |
| Harvest a crop | 10-20 | Based on crop difficulty |
| Build a wall | 15-30 | Based on material |
| Treat a wound | 20-40 | Based on severity |
| Chop a tree | 10 | Flat |
| Butcher an animal | 15 | Flat |
| Cook a meal | 10-25 | Based on recipe complexity |
| Mine ore | 15 | Flat |
| Teach another (1 hour) | 5 for teacher | Teaching deepens understanding |
| Experiment (Expert+) | 30-50 | Risky: may waste materials |

### Failure XP Bonus

Failed attempts give 50% MORE XP than successes. This is counterintuitive but critical:
- It prevents the player from only assigning "safe" tasks to high-skill colonists
- It rewards letting beginners try
- It models real learning: you learn more from mistakes
- It prevents the Rimworld pattern of "only let the best person craft"

### Teaching Multiplier

Working alongside someone 2+ levels higher gives +30% XP gain. The teacher also gains +5 XP per hour taught (teaching deepens understanding). This creates natural apprenticeship without forcing a "teaching" activity.

## Skill Decay

Unused skills decay toward a floor:

```
decay_per_month = 0.1 * (level - floor)
floor = min(level * 0.5, 3.0)  // never forget fundamentals below 3.0
```

A level 8.0 Expert who stops practicing decays:
- Month 1: 8.0 → 7.9 (barely noticeable)
- Month 3: 7.7 (starting to rust)
- Month 6: 7.1 (noticeably degraded)
- Floor: ~4.0 (never loses fundamental competence)

A level 4.0 Journeyman barely decays (floor is ~2.0, decay is 0.2/month).

This means:
- Casual skills naturally settle at the floor
- Maintaining expertise requires ongoing practice
- A returning expert is rusty but not starting over
- The colony needs to actually USE capabilities to keep them

## Colony Strategy Implications

This system changes how the player thinks about their colony:

1. **Breadth vs depth**: Getting 5 people to skill 5.0 is much cheaper than getting 1 person to 8.0. But only the 8.0 can experiment with new recipes. Both strategies are valid.

2. **Specialist vs generalist**: A person with high aptitude in ONE domain is more valuable as a specialist. A person with moderate aptitude across many domains is a flexible generalist. You need both.

3. **Knowledge insurance**: If your only Expert metallurgist dies, the skill is practically lost. Train a backup. But training a backup takes months of someone NOT doing their primary job.

4. **The aptitude lottery**: You won't know who should specialize in what until you test them. The starting chargen gives backstory-based starting skills, but aptitudes are hidden. A "Mechanic" backstory doesn't guarantee metalwork aptitude.

5. **Teaching is investment**: An Expert spending time teaching apprentices isn't producing — they're investing. The player must decide: produce now, or build capability for later.

## Speed Multiplier Formula

For activities gated by a skill:

```
speed_mult = 0.4 + level * 0.06 + max(0, level - 5.0) * 0.04 + max(0, level - 8.0) * 0.06
```

| Level | Speed | Description |
|-------|-------|-------------|
| 0.0 | 0.40x | Novice: very slow |
| 2.0 | 0.52x | Beginner: slow |
| 4.0 | 0.64x | Journeyman: adequate |
| 5.0 | 0.70x | Competent: baseline |
| 6.0 | 0.80x | Skilled: noticeably faster |
| 7.0 | 0.90x | Proficient: quick |
| 8.0 | 1.00x | Expert: full speed (the "reference" speed) |
| 9.0 | 1.16x | Master: impressively fast |
| 10.0 | 1.32x | Legendary: awe-inspiring |

Note: 8.0 (Expert) is the "reference" speed — the speed recipes are balanced around. Most colonists work slower. A Master works faster. This means recipe times in the data files represent expert-speed, and beginners take 2-3x longer. This naturally gates complex recipes behind skill without hard locks.

## Failure Rate Formula

```
failure_chance = max(0.0, 0.6 - level * 0.08 - max(0, level - 4.0) * 0.04)
```

| Level | Failure Rate |
|-------|-------------|
| 0.0 | 60% |
| 2.0 | 44% |
| 4.0 | 28% |
| 5.0 | 20% |
| 6.0 | 12% |
| 7.0 | 4% |
| 8.0+ | 0% |

Below 4.0, attempts often fail and waste materials. This creates real risk in letting beginners try expensive crafts — but they learn faster from failures.

## Implementation Notes

### Data representation

```rust
pub struct SkillLevel {
    pub value: f32,        // 0.0-10.0
    pub xp: f32,           // accumulated XP toward next 0.1 increment
    pub aptitude: i8,      // -2 to +3, hidden
    pub aptitude_known: bool, // has the player seen a reveal event?
}
```

### Migration from current [u8; 6]

Current `skills: [u8; 6]` with values 1-10 maps to starting levels:
- Old skill 1 → new 1.0-2.0
- Old skill 5 → new 4.0-5.0
- Old skill 10 → new 6.0-7.0

Note: the old max (10) maps to only 7.0 in the new system. This is intentional — backstories give a strong starting position but NOT expertise. You earn expertise through play.

### Display

One decimal place: "7.3". Color-coded by descriptor range. Progress pip showing XP toward next 0.1. Aptitude shown as descriptor text when known ("Natural", "Gifted", "Limited").

---

## Emergent Skill Dynamics

### Eureka Moments

Skill progression is usually smooth. What if it isn't? When a colonist has been "stuck" in a domain (less than 0.2 gain in the last in-game week), there's a tiny per-action chance of a **breakthrough** — a sudden 0.3–0.8 level jump.

Eureka triggers (increase chance):
- Working with a material they haven't used before
- Conversation with someone from a different backstory
- Surviving a crisis involving the skill (field surgery under fire, emergency repair)
- Working alongside someone 3+ levels higher (mentorship spark)
- High mood (happy people have more creative insight)

*"Cole was struggling with the forge for weeks. Then last night, something clicked. He looked at the metal differently." Metalwork: 5.4 → 6.1*

This makes stagnation feel like *building pressure toward a breakthrough* rather than hitting a wall. The player watches a struggling colonist with anticipation rather than frustration.

### The Concept Wall

Each skill domain has a hidden **wall** — a specific level where understanding requires a conceptual leap, not just more practice. The wall level varies by domain and individual (influenced by aptitude and mental traits).

- Metalwork wall at ~5.5 (understanding heat treatment)
- Agriculture wall at ~4.0 (understanding soil chemistry)
- Medicine wall at ~6.0 (understanding infection)

Below the wall: smooth progression. At the wall: progress stalls for days/weeks. Past the wall: smooth again until the next wall (if any). Mirrors real skill acquisition — the plateau-then-breakthrough pattern.

Some colonists break through walls faster (aptitude + "Quick Learner" trait). The wall itself is invisible — the player sees slowdown and infers it. Eureka moments can bypass walls entirely — a sudden insight that leaps past the conceptual barrier.

### Emotional State Affects Learning

| Mood State | XP Modifier | Why |
|-----------|------------|-----|
| Happy (mood > 30) | +20% XP | Engaged, curious, open |
| Content (0 to 30) | Normal | Baseline |
| Stressed (mood < -20) | -30% XP, but +50% failure XP | Distracted, but failures burn deeper |
| Breaking (stress > 85) | 0 XP | Can't learn, just surviving |
| Post-break recovery | +40% XP for 24h | Clarity after crisis |

A well-fed, well-rested colony LEARNS faster. The campfire isn't just warmth — it's where people grow. This ties survival systems to progression without artificial "training" activities.

### Signature Styles (8.0+)

At Expert level, items carry the crafter's name: "Mara's Stone Axe". Slight stat variation (+5% damage, +10% durability) based on the maker's skill level.

At Master (9.0+), items get a visual distinction — a subtle tint or mark in the shader. The player starts recognizing WHO made what. Items become sentimental. Losing a Master craftsman doesn't just lose a fast worker — it loses an aesthetic. The colony remembers.

This transforms skill from a speed number into a *personality*. It's the difference between "our cook is fast" and "our cook makes food that tastes like home."

### Dreams Reveal Aptitude

During sleep, colonists occasionally dream about domains they've been exposed to but haven't seriously practiced. Dreams hint at hidden aptitude:

- *"Mara dreamed she was forging metal. Her hands moved with certainty."* → Metallurgy aptitude +2 revealed
- *"Jeff dreamed he was lost in a cave. He kept going the wrong way."* → Mining aptitude -1 revealed

Dream frequency scales with **bed quality**. Good bed → more dreams → faster aptitude discovery. Sleeping rough → fewer dreams → aptitudes stay hidden. This gives beds real gameplay weight beyond rest recovery.

### Teaching Degrades Through Retelling

When an Expert teaches, the student learns accurately. When a Familiar teaches, knowledge degrades — the student's **accuracy** is lower. They don't know it's degraded. Their skill number goes up. But they make "mysterious" errors.

Only when a higher-level practitioner observes: *"That's not how you temper steel. Who taught you that?"* → accuracy corrected, slight XP loss as they unlearn.

The colony's knowledge has **fidelity**. A colony that lost its Expert and trained replacements from Familiar-level teachers will be subtly worse — and not realize it until someone better comes along. This rewards preserving expertise.

---

## Information Presentation

How does the player learn about skill events? Three tiers, drawing from DN-021's philosophy of world-first, chrome-second:

### Tier 1: World Shows It (ambient, always)

Skill is visible in HOW colonists work (DN-021 Part 2):
- Familiar: pauses, hesitates, looks uncertain
- Competent: smooth, steady, confident
- Expert: casual, doesn't look at what they're doing
- Master: distinctive flourishes, signature gestures

The player who watches closely reads skill from behavior. No panel needed.

Eureka moments have a visual: the colonist pauses, a brief "lightbulb" thought bubble, then they resume working noticeably faster. The world shows the breakthrough.

### Tier 2: Banner Bar (important, contextual — EU4-style)

A horizontal strip below the colonist bar. Small, persistent banners for important ongoing states that need attention but aren't emergencies:

| Banner | Trigger | Look |
|--------|---------|------|
| 🔔 "Night approaching" | Day fraction > 0.75 | Amber, auto-dismiss at dawn |
| 🍖 "Food low" | Colony food < 2 days supply | Red, persists until stocked |
| 📈 "Cole: Metalwork breakthrough!" | Eureka moment | Gold, dismiss on click |
| ⚠️ "Mara can't butcher (no knife)" | Hard tool gate failure | Orange, dismiss when resolved |
| 🎓 "Jeff reached Expert Construction" | Level milestone (whole numbers) | Blue, dismiss on click |
| 💀 "Duskweavers spotted" | Night threat arrives | Red pulse, dismiss at dawn |
| 🛠️ "Snare caught a dusthare!" | Trap event | Green, dismiss on click |

Banners are small (icon + short text), stack horizontally, auto-expire or dismiss on click. Maximum ~6 visible at once. They don't interrupt — they inform. The player processes them at their own pace.

**Key difference from Transport Tycoon popups:** Banners don't pause the game or demand attention. They sit there. If the player is busy building a wall, the "Cole had a breakthrough" banner waits patiently. But it's there, visible, and the player won't miss it.

**Key difference from Rimworld's letter system:** Rimworld stacks letters on the right edge that accumulate and require individual clicking. Banners are more like EU4/CK3's alert icons — contextual, self-resolving, and positioned where the eye naturally falls.

### Tier 3: Modal Popup (rare, unmissable)

Reserved for events so significant they SHOULD interrupt the player:

- First night warning (once per game): "Duskweavers hunt in darkness." Pauses game. Teaches.
- Colony-first achievements: "First colonist reached Master level." Dramatic popup. Maybe confetti.
- 10.0 legendary (if it ever happens): Full-screen moment. This deserves everything.
- Death of a named colonist: Brief pause, portrait, epitaph.
- Colony milestone: First full year survived. First building completed. First child born (future).

These are Transport Tycoon "new airplane" moments — rare enough that interruption is welcome. The rarity preserves their impact. If everything is a modal popup, nothing is.

### Tier 4: Colony Log (persistent, searchable)

Everything goes in the log. Every eureka, every level-up, every skill decay warning, every aptitude reveal. The log is the memory of the colony. Accessed through the writing desk (diegetic, DN-021) or a UI button.

The log groups entries by colonist. Clicking a colonist's tab in the character sheet shows THEIR personal log — every event, every breakthrough, every failure. This is where the player reads the story of a person's growth.

---

## Inherent Variance

Every modifier in the game should have a probability distribution, not a fixed value. The numbers in this document ("+40% speed", "24h recovery", "60% failure rate") are **means**, not constants.

### The Principle

Nothing in the frontier is precise. A skilled craftsman doesn't produce identical work every time. A wound doesn't heal on a predictable schedule. The weather doesn't care about your plans.

When the game computes any modifier, it samples from a distribution centered on the nominal value:

```
actual_value = nominal * (1.0 + normal_random(0.0, variance))
```

Where `variance` depends on the system:

| System | Variance (σ) | What this means |
|--------|-------------|-----------------|
| Skill speed modifier | 0.15 | +40% trait → actual range ~25%–55% per action |
| Craft quality | 0.20 | Same crafter, same recipe → slightly different results each time |
| Failure chance | 0.10 | 20% failure rate → sometimes 15%, sometimes 25% |
| XP gain | 0.20 | Some sessions teach more than others |
| Recovery time | 0.25 | "~24h recovery" → actual 18h–30h |
| Crop growth | 0.15 | Same field, same conditions → some plants mature first |
| Spoilage time | 0.20 | Same meat, same storage → one piece lasts longer |
| Bleed rate | 0.15 | Wounds are unpredictable |
| Mood effects | 0.20 | Eating the same meal doesn't always feel the same |
| Teaching effectiveness | 0.25 | Some lessons land better than others |
| Eureka chance | 0.30 | High variance — breakthroughs are inherently unpredictable |

### Why This Matters

1. **Prevents optimization lock-in.** If "+40% crafting" always means exactly 1.4x, the player solves the system once. With variance, the system is *legible* (you know the trait is good) but not *solved* (you can't predict the exact output).

2. **Creates stories.** "Mara's recovery took longer than expected" is a story. "Mara recovered in exactly 24 hours" is not. The variance creates surprise within a predictable envelope.

3. **Rewards attention over calculation.** The player learns to read trends, not compute formulas. "Cole is usually fast at the forge" is more interesting than "Cole is exactly 1.35x speed."

4. **Makes repeated actions feel alive.** Crafting the same item 10 times should feel like 10 slightly different experiences, not one experience repeated. Some come out better, some worse. The crafter's skill determines the CENTER of the distribution, not the exact output.

### Variance Sources

The variance itself can be modified:

- **High skill reduces variance.** An Expert (8.0) produces consistently good work (σ × 0.6). A Beginner (1.0) is wildly unpredictable (σ × 1.4). Mastery is partly about consistency.
- **Stress increases variance.** A stressed colonist is erratic — sometimes brilliant, usually worse. Calm colonists are steady.
- **Perfectionist trait** reduces variance (more consistent) but also reduces mean speed.
- **Neurotic trait** increases variance in everything — the double-edged nature becomes: sometimes great, sometimes terrible, always uncertain.

### The "Lucky" and "Unlucky" Hidden Traits

These traits shift the ENTIRE distribution, not just a single stat:

- **Lucky:** All variance rolls are shifted +0.5σ toward favorable outcomes. Not huge, but over hundreds of actions, Lucky colonists measurably outperform their nominal stats. The effect is subtle enough that the player suspects rather than knows.
- **Unlucky:** Shifted -0.5σ. Things go wrong slightly more often. Equipment breaks sooner. Food spoils faster. Wounds heal slower. Again, subtle — the player wonders "is Jeff just having a bad week, or...?"

### Implementation

A simple per-action hash gives deterministic-but-unpredictable variance:

```rust
fn varied(nominal: f32, variance: f32, seed: u32) -> f32 {
    // Box-Muller approximation from hash
    let u1 = (seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
    let u2 = ((seed >> 16).wrapping_mul(1013904223) & 0xFFFF) as f32 / 65535.0;
    let z = (-2.0 * u1.max(0.001).ln()).sqrt() * (u2 * TAU).cos();
    nominal * (1.0 + z * variance).clamp(0.3, 2.0)  // clamp to prevent absurd outliers
}
```

The seed is derived from (pleb_id, action_type, game_time) — same inputs produce same output (deterministic replay), but every action/moment is unique.

---

## Implementation Priority

1. **0.0-10.0 scale with exponential XP** — replaces current [u8; 6]
2. **Aptitude as hidden ceiling** — per-domain, revealed through play
3. **Eureka moments** — rare breakthrough events
4. **Emotional XP modifier** — tie mood to learning
5. **Banner bar** — EU4-style persistent notification strip
6. **Signature styles** — named items from Expert+ crafters
7. **Teaching degradation** — knowledge fidelity (requires DN-019)
8. **Concept walls** — domain-specific plateaus (tuning pass)
9. **Dreams** — aptitude reveal during sleep (requires bed quality system)
10. **10.0 legendary popup** — the rarest event in the game

# "The Manifest" — Frontier Crew Recruitment

## Concept

You're a wagon master filling a ship manifest before landing on the frontier. At a dusty orbital station, you review dossiers of applicants looking for passage. Each one has a past, skills, baggage — and gear they bring along.

## Core Mechanic

- **3 applicant cards** are dealt face-up (randomly generated colonists)
- **Recruit** one → they join your crew panel on the right
- **Pass** → draw 3 new ones (the old ones walk away forever)
- Crew capacity: 3 colonists (hard mode: 2, easy: 5)
- When full, **"Land"** button takes you to the map

The tension: every draw is a gamble. That perfect sharpshooter might be in the next batch — or you might get three drifters with drinking problems.

## Backstories

Each colonist has one backstory that determines base skills and starting gear.

| Backstory | Strong In | Brings | Flavor |
|-----------|-----------|--------|--------|
| **Sheriff** | Combat, Social | Revolver, badge | *"Kept the peace in Dry Gulch. Until the peace kept itself."* |
| **Prospector** | Mining, Hauling | Pickaxe, lantern | *"Spent 20 years looking for gold. Found mostly dirt."* |
| **Ranch Hand** | Farming, Hauling | Rope, seeds | *"Could rope a steer at 50 paces. Cows, less so."* |
| **Mechanic** | Crafting, Building | Wrench, spare parts | *"If it's broke, she'll fix it. If it ain't, she'll improve it."* |
| **Frontier Doc** | Crafting, Social | Medical kit, scalpel | *"Lost his license. Kept his scalpel."* |
| **Outlaw** | Combat, Stealth | Pistol, lockpick | *"Three counties want him. Fourth one got him."* |
| **Preacher** | Social, Farming | Bible, seeds | *"Came to save souls. Staying to save lives."* |
| **Saloon Keep** | Social, Crafting | Cooking pot, whiskey | *"Good listener. Better bartender."* |
| **Drifter** | Hauling, Combat | Bedroll, knife | *"No past worth mentioning. No future worth planning."* |
| **Engineer** | Building, Crafting | Blueprints, wrench | *"Built bridges on three worlds. Burned one."* |
| **Convict** | Hauling, Mining | Shovel, grit | *"Served time. Now serves a purpose."* |
| **Scout** | Combat, Farming | Binoculars, canteen | *"Knows the land better than most know themselves."* |

## Traits

Each colonist rolls 1-2 traits that modify needs, stress, and work behavior.

### Positive
- **Deadeye** — shooting accuracy +50%
- **Green Thumb** — crops in their zone grow 20% faster
- **Iron Gut** — hunger drains half as fast
- **Camel** — thirst drains half as fast
- **Tinker** — crafting speed +30%
- **Night Owl** — no night vision penalty, +20% work speed after dark
- **Steady Nerve** — stress gain halved
- **Ox** — haul speed +40%

### Negative
- **Pyromaniac** — may start fires during mental breaks
- **Lazy** — work speed -20%
- **Gourmand** — eats twice as much
- **Volatile** — stress builds 2x faster
- **Wanted** — bounty hunters show up periodically
- **Clumsy** — occasionally drops/breaks items

### Double-Edged
- **Perfectionist** — 30% slower, but output quality higher
- **Loner** — faster alone, stressed when near others
- **Adrenaline Junkie** — faster during crises, stressed during peace
- **Neurotic** — +25% work speed, +50% stress gain

## Skills

Five skill bars (1-5 pips), set by backstory + random variance:

- **Farm** — planting, harvesting, tending
- **Build** — construction speed + quality
- **Craft** — workbench/kiln speed
- **Haul** — carry speed, pathing priority
- **Fight** — accuracy, damage, dodge

These map directly to `work_priorities` — a colonist with Farm:5 gets auto-assigned priority 1 in farming.

## Special Stamps

Some applicant cards have stamps that modify the deal:

- **WANTED** (red stamp) — skilled but brings trouble. Bounty hunter raids target them specifically. Higher stats as compensation.
- **DESPERATE** (yellow stamp) — willing to work for nothing. Doesn't cost a crew slot... but has 2 negative traits instead of the usual 0-1.

## Name Generator

Western-flavored procedural names from component lists:

- **First**: Jeb, Mae, Silas, Clara, Dutch, Rose, Hank, Nettie, Colt, Ada...
- **Nickname** (optional, 40% chance): "Dusty", "Two-Shot", "Patches", "Lucky", "Slim", "Doc"...
- **Last**: McCrae, Dalton, Bridger, Holloway, Vance, Thorn, Cassidy, Bonney...

## UI Layout

```
┌─────────────────────────┐
│  ELEANOR "DUSTY" MCCRAE │
│  ─── Frontier Doc ───   │
│                         │
│  Farm ██░░░  Build █░░░░│
│  Craft████░  Haul  ██░░░│
│  Fight██░░░             │
│                         │
│  ✦ Steady Nerve         │
│  ✧ Perfectionist        │
│                         │
│  Gear: Medical kit,     │
│        Scalpel          │
│                         │
│  "Lost his license.     │
│   Kept his scalpel."    │
│           [RECRUIT]     │
└─────────────────────────┘
```

Three applicant cards side by side on the left. Recruited crew in a smaller panel on the right. "DRAW NEW APPLICANTS" button at the bottom.

## Integration with Existing Systems

| Chargen Data | Maps To |
|-------------|---------|
| Skill values | `work_priorities` in Pleb |
| Trait modifiers | `PlebNeeds` rate multipliers (hunger_rate, thirst_rate, stress_rate) |
| Starting gear | `PlebInventory` items |
| Pyromaniac trait | New `MentalBreakKind::Arson` |
| Stress traits | Stress system multipliers in needs.rs |
| Fight skill | Accuracy modifier in physics.rs bullet trace |

## Implementation Steps

1. **Backstory + traits + skills data** — structs, enums, random generation
2. **UI card layout** — egui applicant cards, recruit/pass flow
3. **Gear integration** — starting items feed into pleb inventory
4. **GameState integration** — MainMenu → MapGen → Chargen → Playing
5. **WANTED/DESPERATE stamps** — special applicant modifiers
6. **Name generator** — procedural western names

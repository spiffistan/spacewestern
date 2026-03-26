# Cards as the Narrative Meta-Layer

The colony sim is the core. Cards govern what *happens to you* — events, discoveries, crises, opportunities. Every few days the frontier deals you a hand, and you play what you've got.

## 1. The Frontier Deck (Event Engine)

A literal deck of cards. One drawn every 2-3 days. The deck isn't static — it's composed based on colony state, so it's curated randomness, not pure chaos.

### Deck Composition Shifts Dynamically

- Colony is wealthy? → more Raid and Trader cards shuffle in
- Colony is struggling? → more Discovery and Opportunity cards (rubber banding)
- Built a saloon? → Visitor cards become more frequent
- Near ruins? → Mystery cards enter the deck
- Winter approaching? → Weather cards stack up

### Card Categories

| Suit | Symbol | Examples |
|------|--------|----------|
| **Storms** | Spades | Dust storm, flash flood, cold snap, heat wave, wildfire |
| **Strangers** | Hearts | Wanderer seeks refuge, trader caravan, bounty hunter, con artist |
| **Bounty** | Diamonds | Ore vein discovered, water spring, abandoned cache, fertile ground |
| **Trouble** | Clubs | Disease outbreak, wolf pack, bandits spotted, structural collapse |
| **Fortune** | Stars ★ | Gold rush rumor, mysterious map, alliance offer, lost herd found |

### Cards Present Choices

Each card presents a choice, not just an outcome:

> **"A Stranger at the Gate"**
> *A wounded drifter stumbles to your perimeter. Says he's running from Redskulls.*
>
> - **Take him in** → new colonist (Drifter backstory, possibly Wanted trait)
> - **Patch him up and send him on** → +morale, he might return later with a gift
> - **Turn him away** → nothing happens... or the Redskulls come looking anyway

This turns passive events into player decisions with consequences.

## 2. Blueprint Cards (Progression)

No tech tree. Instead, you **find** blueprint cards in the world:

- Loot an abandoned camp → "Windmill" blueprint card
- Trade with a merchant → "Rifle" blueprint card
- Explore ruins → "Electric Lamp" blueprint card
- Complete an event chain → "Steel Smelting" blueprint card

Progression feels like **discovery**, not research. Two playthroughs unlock things in different orders. You might get gunpowder before you get glass, or vice versa.

Blueprints are shown as actual cards in a "Journal" UI — a collection book that fills up. Satisfying to collect, gives direction to exploration.

## 3. Colonist Ability Cards

Each colonist gets 1 ability card from their backstory. Active ability, long cooldown (once per day or once per crisis). Click the card to play it.

| Backstory | Card | Effect |
|-----------|------|--------|
| Sheriff | **"Law & Order"** | All colonists in radius: +50% combat, 60 seconds |
| Doc | **"Field Surgery"** | Instantly stabilize a dying colonist |
| Mechanic | **"Jury Rig"** | Instantly repair a broken machine/pipe |
| Scout | **"Eagle Eye"** | Reveal fog of war in huge radius for 30 seconds |
| Preacher | **"Sermon"** | All colonists: -20 stress |
| Outlaw | **"Ambush"** | Next shot deals 3x damage, guaranteed hit |
| Saloon Keep | **"Liquid Courage"** | All colonists: +30% work speed, +10 stress |
| Ranch Hand | **"Roundup"** | Instantly haul all loose items to nearest crate |
| Engineer | **"Overcharge"** | Double power output from all generators for 60s |
| Convict | **"Hard Time"** | This colonist ignores needs for 2 minutes |

These give each colonist a distinct identity beyond their stats. The Sheriff isn't just "guy with high combat" — he's the one who can rally everyone when the Redskulls attack.

## 4. Crisis Response Hands

When a crisis triggers (raid, fire, plague), you're dealt a **response hand** based on what your colony actually has. You pick one card to play.

### During a Fire
- Have a well? → **"Bucket Brigade"** — colonists prioritize firefighting
- Have stone buildings? → **"Firebreak"** — mark tiles to demolish, stopping spread
- Have nothing? → **"Flee"** — colonists evacuate to safety

### During a Raid
- Have a sheriff? → **"Fortify"** — colonists take cover, accuracy bonus
- Have an outlaw? → **"Counterambush"** — surprise attack, enemies rout
- Have walls? → **"Lockdown"** — close all doors, enemies must breach

### During a Plague
- Have a doc? → **"Quarantine"** — isolate sick colonists, slows spread
- Have a preacher? → **"Comfort"** — sick colonists gain less stress
- Have whiskey? → **"Anesthetic"** — pain reduction, faster recovery

This replaces frantic micromanagement with a strategic decision point. The real-time action continues but your card choice shapes the outcome.

## 5. The Saloon Table

Build a saloon → stress relief building AND a card game venue.

Occasionally an event fires: *"A stranger challenges you to a hand of cards."* A simple poker mini-game (or choice-based bluff sequence) with stakes:

- **Win** → information about nearby ruins, a blueprint card, an item, or a recruit
- **Lose** → lose some supplies, or the stranger causes trouble
- **Cheat** (requires Outlaw colonist) → guaranteed win but risk getting caught

Pure flavor, deeply western.

## Visual Language

Cards rendered as slightly worn poker cards with custom western art. Each category has its suit symbol.

When an event fires, the card deals onto screen — slides in, flips over. Lerped position + opacity animation.

## What This Does for the Game

- **Replayability** — deck composition means no two games play the same
- **Meaningful choices** — events aren't just "thing happened", they're decisions
- **Progression feels earned** — blueprints are found, not researched
- **Colonists feel unique** — ability cards give them signature moments
- **Crises feel manageable** — response cards give you agency without pausing
- **Thematic cohesion** — cards + western = poker + fate, deeply natural

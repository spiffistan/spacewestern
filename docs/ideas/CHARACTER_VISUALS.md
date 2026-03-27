# Character Visual Ideas

Ideas that build on the layered sprite system (DN-009). These explore how character visuals can carry gameplay meaning, personality, and history.

## Clothing as Equipment

Clothing isn't cosmetic — it's functional equipment that happens to be visible. A leather jacket changes the torso sprite AND gives cold resistance. A heavy coat makes the silhouette bigger AND slows movement AND keeps you warm.

This unifies the visual and mechanical systems. You see what a colonist is wearing and know what it does. Looting a dead enemy's armor and equipping it changes how your colonist looks, moves, and survives.

Clothing tiers could follow the crafting progression:
- Scrap cloth (starting) → basic fiber clothing → leather → heavy leather → armored
- Each tier has a distinct torso sprite variant + stat modifiers
- Crafted at the workbench, requires increasingly rare materials

## Aging and Wear

Sprites accumulate visual history over time. The shader applies per-pleb modifiers:

- **Battle-worn:** a colonist who's been in firefights gets a slightly singed/tattered tint
- **Mine dust:** mining work gradually darkens clothing toward grey-brown
- **Sun-bleached:** outdoor workers' clothing fades lighter over time
- **Fresh arrival:** crisp, bright, saturated colors
- **Veteran:** faded, muted, worn

A single `wear` float per pleb (0.0 = fresh, 1.0 = veteran) drives a shader tint: slightly desaturate, darken edges, add noise to the fill color. No extra sprites — just a per-pleb parameter that shifts over time based on activity.

Long-serving colonists look different from new arrivals without any explicit visual change. The world wears them down.

## Social Reading at a Glance

The goal: you recognize colonists by sight, never by name label. "The big one with the wide-brim is the sheriff. The hooded thin one is the drifter."

This requires:
- **Body type** is distinct enough to spot (thin vs stocky vs large)
- **Hat** is the primary identifier (most visible at distance)
- **Shirt color** is the secondary identifier
- Combined, these create unique silhouettes per colonist

Test: turn off name labels. Can you still tell who's who? If yes, the visual identity system works. If not, more silhouette variation is needed.

## Emotes and Reactions

Not face expressions (too small) but full-body gestures. Triggered rarely, for high-impact moments:

| Emotion | Gesture | Trigger |
|---------|---------|---------|
| Stressed | Throws hands up | Stress > 70 |
| Happy | Small hop/bounce | Mood boost event |
| Scared | Cowers/crouches | Nearby explosion, raid alarm |
| Angry | Stomps/kicks ground | Mental break onset |
| Grief | Slumps, head down | Ally death |
| Triumph | Fist pump | Raid repelled, construction complete |

Each is 2-3 extra body frames, played once then returning to normal. These are RARE — you might see 1-2 emotes per game-day. That rarity makes them meaningful. "Oh no, the sheriff threw his hands up. What's wrong?"

## Relationship Body Language

Relationships between colonists manifest physically:

- **Friends:** stand closer during idle time, heads turn toward each other
- **Rivals:** face away from each other, maintain distance, never sit adjacent
- **Couple:** walk side by side (not single-file), occasional head turns toward partner
- **Leader (sheriff):** other colonists face toward them during idle gathering
- **Outcast:** alone at the edge of groups, head down

The head independence system (DN-009) enables most of this — heads turn toward friends, away from enemies. Idle positioning is a CPU behavior change, not a sprite change. No extra art needed.

## Weather-Reactive Appearance

The world affects how characters look:

- **Rain:** clothing darkens (wet tint multiply), colonists hunch slightly (rain idle frame), optional: water drip particles
- **Cold:** arms cross (cold idle frame), breath puffs as tiny white particles on exhale cycle
- **Hot:** (if clothing system exists) sleeves rolled / jacket open variant. Otherwise: slower walk animation speed
- **Night:** torch-carriers glow warmly. Others are dimmer, silhouettes against darkness.

Most of these are shader tint changes or idle frame swaps — 1-2 extra body frames per weather state, reused across all body types.

## Silhouette Language for Enemies

Redskulls (and future factions) should be identifiable by shape alone, without color or labels:

- **Different posture:** enemies stand wider, more aggressive stance
- **Different proportions:** slightly larger head-to-body ratio? Bulkier?
- **Distinct headgear:** Redskulls wear skull-marked bandanas or helmets — unique hat sprites
- **Movement style:** enemies move differently (more direct, less idle wandering)

Test: render all characters as black silhouettes. Can you tell colonists from enemies? If yes, the design works.

## Character Shadows

The raycaster already handles shadows for blocks and trees. Character sprites could cast small shadows:

- Dark ellipse on the ground, offset in the sun direction
- Size scales with character (stocky casts wider shadow)
- Disappears at night, stretches at dawn/dusk
- Very subtle but grounds characters in the world
- Could use the same heightmap approach as trees — sprite alpha channel = height for shadow casting

## Procedural Scars and Markings

After taking damage, a small visual mark appears on the body sprite:

- A 1-2 pixel darker patch at a random position on the body
- Accumulated over the colonist's lifetime
- Stored as a list of (position, type) per pleb — rendered as small overlays
- A veteran who's been through ten fights looks visibly marked
- A fresh colonist is clean

Types: scar (dark line), burn (reddish patch), bruise (blue-purple, fades over time). Permanent scars stay. Bruises heal.

This is "visual history" — you look at a colonist and see what they've been through. No text needed.

## Paper Doll Inventory

The BG2-inspired inventory window (already designed) shows the colonist at 4× sprite size with equipment slots:

```
┌──────────────────────────┐
│     [Hat slot]           │
│    ┌──────────┐          │
│    │          │  [Weapon] │
│    │  4× pleb │          │
│    │  sprite  │  [Tool]  │
│    │          │          │
│    └──────────┘  [Pack]  │
│  [Shirt slot] [Pants]    │
│                          │
│  Inventory grid below    │
└──────────────────────────┘
```

The same layered compositing from DN-009 runs at 4× scale (96×96 instead of 24×24). Dragging equipment onto slots updates the pleb's sprite layers in real-time — you see the hat appear, the tool change, the armor swap. Same shader, same atlas, just bigger render.

## Faction Uniforms

If multiple factions exist:
- Each faction has a signature color (colonists = blue, Redskulls = red, traders = green)
- The outline shader tints to faction color for enemies
- Friendly factions' clothing loosely matches their color palette
- Captured enemies wearing your faction's clothes = visible conversion

## Death Poses

When a colonist dies, instead of the current red X overlay:
- Body sprite renders rotated 90° (lying on ground)
- Desaturated and darkened
- Hat falls off (rendered as separate small sprite nearby)
- Held item drops (already handled by ground items system)
- Body persists until buried/hauled (not instant disappear)

A dead colonist on the ground with their hat beside them tells a story. Much more evocative than a circle with an X.

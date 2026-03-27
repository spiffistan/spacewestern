# DN-009: Pleb Sprite System

## Status: Proposed

## Reference Analysis

| Game | Layers | Directions | Walk Frames | Sprite Size | Key Trick |
|------|--------|-----------|-------------|-------------|-----------|
| Rimworld | 3 (body, head, equip) | 4 | None (slide) | ~64×64 | Head turns independently from body |
| Prison Architect | 2 (head circle, body rect) | Smooth | Limb rotation | Simple geometry | Color = identity (orange/blue) |
| Baldur's Gate 2 | 1 (pre-baked) | 8-16 | Full cycle (8) | ~100×120 | Armor replaces entire sprite |
| Oxygen Not Included | 4-5 | 2 (side view) | Full cycle | ~40×60 | Body proportions = personality |
| Stardew Valley | 3 (body, hair, hat) | 4 | 4 frames | 16×24 | Hair + shirt + hat = identity |
| Dwarf Fortress | 1 | 1 | None | Tile-sized | Text is the identity, not sprite |

### Lessons

1. **Fewer layers, bigger impact.** Rimworld uses 3, not 8. Each layer is visually distinct. More layers = more work for marginal improvement.
2. **Head independence matters more than layer count.** A head that turns separately adds more life than 5 extra clothing layers.
3. **Walk animation is optional.** Rimworld ships without it. If walk feels good, it comes from smooth position interpolation, not sprite frames.
4. **Color IS identity at small scales.** Hat shape + shirt color + hair color = recognizable colonist. You don't need face expressions at pixel scale.
5. **Mood through icons, not faces.** A face at 16-24px is 3-4 pixels. Use overhead icons (💤 😡 !) instead.

## Revised Design: 3+2 Layers

### Core Layers (always rendered)

**Layer 1 — Body (24×24)**

Full body in one sprite: legs, torso, arms at rest. The single most important sprite.

- Tinted in TWO zones: lower half by `pants_color`, upper half by `shirt_color`. Skin visible at hands/neck, tinted by `skin_color`.
- 3-4 body types: thin, medium, stocky, large. Determined by backstory/random at chargen.
- 8 directions (N, NE, E, SE, S, SW, W, NW)
- 3 walk frames: left step, neutral, right step. Frame 1 mirrored as frame 3 gives a 4-step cycle from 3 unique sprites.
- Sprites per body type: 8 dir × 3 frames = 24
- Total body sprites: 4 types × 24 = **96 sprites**

**Layer 2 — Head (12×12, positioned on body)**

Head + hair as one piece. Positioned on top of body at a direction-dependent offset.

- Tinted: face area by `skin_color`, hair area by `hair_color`
- 6-8 hair styles (short, long, bun, mohawk, bald, braided, ponytail, curly)
- Head can face a **different direction** than body (the Rimworld trick). Adds enormous life. Walking east, glancing south at a fire. Only costs an independent direction index, no extra sprites.
- 8 directions per style
- Sprites per style: 8 directions × 1 frame = 8
- Total head sprites: 8 styles × 8 = **64 sprites**

**Layer 3 — Hat (12×12, positioned above head)**

Headgear overlaid on head. Optional (index 0 = no hat).

- Not tinted (hat color is baked in — a brown cowboy hat is brown)
- 6-8 types: none, wide-brim (sheriff/scout), bandana (outlaw), flat-top (preacher), cap (mechanic), hood (drifter), hard hat (engineer), straw hat (ranch hand)
- 8 directions
- Total hat sprites: 7 types × 8 dir = **56 sprites**

### Overlay Layers (contextual)

**Layer 4 — Held Item (10×10, at hand position)**

Tool or weapon in the pleb's hand. Position varies by body direction (hand is at different screen position for each facing). Only shown during relevant activities or when equipped.

- 6 types: empty, axe, hammer/pick, shovel, bucket, gun
- 4 key directions (N, E, S, W — diagonals interpolate or pick nearest)
- Total: 6 × 4 = **24 sprites**

**Layer 5 — Carried Object (10×10, above head)**

Item being hauled. Shown above/on the pleb's back.

- 4 types: log/wood bundle, rock, plank stack, crate/sack
- Direction-independent (always shown from above)
- Total: **4 sprites**

### Status Icons (not sprites — UI overlay)

Mood and activity indicators rendered as small icons above the pleb, not as sprite layers:

- 💤 Sleeping
- 🔨 Working
- ❗ Alert/danger
- 😠 Stressed (high stress)
- 🤕 Injured
- 🥶 Freezing
- 🫁 Suffocating

These render in the egui label layer (existing `Order::Background` world labels), not in the raytrace shader. Clearer at any zoom level than trying to encode mood into a 3-pixel face.

## Sprite Totals

| Layer | Sprites | Size | Memory |
|-------|---------|------|--------|
| Body | 96 | 24×24 | 55 KB |
| Head | 64 | 12×12 | 9 KB |
| Hat | 56 | 12×12 | 8 KB |
| Held item | 24 | 10×10 | 2 KB |
| Carried | 4 | 10×10 | 0.4 KB |
| **Total** | **244** | | **~75 KB** |

Fits easily in a single 512×256 texture atlas.

## Sprite Size: Why 24×24

| Size | Pixels | At typical zoom | Character feel |
|------|--------|-----------------|----------------|
| 16×16 | 256 | 32-64 screen px | Colored blob. Identity from color only. |
| 24×24 | 576 | 48-96 screen px | Clear body/head split. Hat shape readable. |
| 32×32 | 1024 | 64-128 screen px | Detailed. Risk of looking too large vs tiles. |

24×24 is the sweet spot:
- 1.5× the tile grid — plebs are slightly larger than a tile (people are bigger than floor squares)
- Enough pixels for clear hat shape, body type, held tool
- Still retro pixel-art friendly
- Head (12×12) has enough room for hair style distinction

## The Head Independence Trick

This is the single highest-value feature for pleb liveliness:

```
GpuPleb fields:
  angle:      f32   // body facing direction (movement)
  head_angle: f32   // head facing direction (attention)
```

The body faces the movement direction. The head faces the point of interest:
- Walking east, head turns south to watch a fire
- Idle, head occasionally glances at nearby plebs
- Crafting, head faces the workbench
- Conversation, heads face each other

The shader renders the body at `body_dir` and the head at `head_dir`. When both match, the pleb looks normal. When they differ, the pleb looks alive.

CPU logic for head direction:
- Default: same as body
- Within 2 tiles of fire/crisis: head turns toward it
- Talking to another pleb: head faces them
- Idle: random slow head turn (every 3-5 seconds)
- Working: head faces work target

## Color Tinting

Sprites are drawn in neutral gray tones. Color is applied per-pleb at render time:

```wgsl
// Body sprite has zones marked by the sprite's green channel:
// G < 0.3 = pants zone (lower body)
// G 0.3-0.7 = shirt zone (upper body)
// G > 0.7 = skin zone (hands, neck)
let zone = sprite_sample.g;
var tint = vec3(1.0);
if zone < 0.3 {
    tint = vec3(p.pants_r, p.pants_g, p.pants_b);
} else if zone < 0.7 {
    tint = vec3(p.shirt_r, p.shirt_g, p.shirt_b);
} else {
    tint = vec3(p.skin_r, p.skin_g, p.skin_b);
}
color = sprite_sample.r * tint; // R channel = luminance
```

The sprite's red channel encodes luminance (light/dark). The green channel encodes which color zone. Blue is free for future use (metallic? roughness?). Alpha is transparency.

This means one body sprite works for every color combination. A dark-skinned pawn in a red shirt and blue pants uses the same sprite as a light-skinned pawn in green and brown.

## GpuPleb Struct Changes

Repurpose existing padding fields (no size change):

```rust
pub struct GpuPleb {
    // Position (existing)
    pub x: f32, pub y: f32, pub angle: f32, pub selected: f32,
    pub torch: f32, pub headlight: f32, pub carrying: f32, pub health: f32,
    // Colors (existing)
    pub skin_r: f32, pub skin_g: f32, pub skin_b: f32, pub hair_style: f32,
    pub hair_r: f32, pub hair_g: f32, pub hair_b: f32, pub head_angle: f32, // was _pad2
    pub shirt_r: f32, pub shirt_g: f32, pub shirt_b: f32, pub hat_type: f32, // was _pad3
    pub pants_r: f32, pub pants_g: f32, pub pants_b: f32, pub held_item: f32, // was _pad4
}
```

Fields repurposed:
- `hair_style` (already exists): head sprite variant (0-7)
- `head_angle` (was _pad2): independent head direction
- `hat_type` (was _pad3): headgear variant (0=none, 1-7=types)
- `held_item` (was _pad4): tool/weapon (0=empty, 1-6=types)
- `carrying` (already exists): carried object type (0=none, 1-4=types)

## Shader Rendering

```wgsl
// For each pleb within render distance:
let pdx = world_x - p.x;
let pdy = world_y - p.y;
if abs(pdx) < 0.75 && abs(pdy) < 0.75 {
    // Sprite-local coordinates (24×24 sprite = 0.75 tile radius)
    let local_x = (pdx + 0.75) / 1.5;  // 0..1
    let local_y = (pdy + 0.75) / 1.5;

    // Body direction: 8-way from angle
    let body_dir = u32((p.angle / 0.7854 + 0.5) % 8.0);
    // Walk frame from position interpolation
    let walk_phase = fract((p.x + p.y) * 2.0 + camera.time * 3.0);
    let walk_frame = u32(walk_phase * 3.0); // 0, 1, 2

    // Head direction: independent
    let head_dir = u32((p.head_angle / 0.7854 + 0.5) % 8.0);

    // Sample body (24×24 in atlas)
    let body = sample_body(body_type, body_dir, walk_frame, local_x, local_y);
    if body.a > 0.5 {
        color = tint_body(body, p);
    }

    // Sample head (12×12, offset by direction)
    let head_offset = head_position(body_dir); // where head sits on body
    let head_uv = (vec2(local_x, local_y) - head_offset) * 2.0; // scale to head space
    if head_uv.x > 0.0 && head_uv.x < 1.0 && head_uv.y > 0.0 && head_uv.y < 1.0 {
        let head = sample_head(hair_style, head_dir, head_uv.x, head_uv.y);
        if head.a > 0.5 {
            color = tint_head(head, p);
        }
    }

    // Hat (on top of head, same offset)
    // Held item (at hand position, varies by body_dir)
    // Carried object (above everything)
    // ... similar pattern ...
}
```

## Outline Rendering

After compositing all layers, outline check on the composited alpha:

```wgsl
if final_alpha < 0.1 {
    // Transparent pixel — check neighbors for outline
    let texel = 1.0 / 24.0;  // sprite texel size
    // Check 4 cardinal neighbors
    if any_neighbor_opaque(local_x, local_y, texel) {
        color = select(vec3(1.0), vec3(0.3, 0.9, 0.3), p.selected > 0.5);
        // White outline by default, green when selected
    }
}
```

Replaces the current pulsing selection ring. Works for any sprite shape automatically.

## Corpse Rendering

Dead plebs:
- Body sprite rendered at `angle` (lying in the direction they fell)
- Desaturated (grayscale × 0.5)
- No head independence (head locked to body angle)
- No walk animation (frame 0 always)
- Optional: blood overlay sprite beneath body
- Health bar hidden

## Backstory Visual Identity

From chargen (CHARGEN.md), each backstory gets a default hat and body type:

| Backstory | Body Type | Hat | Idle Held Item |
|-----------|-----------|-----|----------------|
| Sheriff | Medium | Wide-brim | — |
| Prospector | Stocky | Battered hat | Pick |
| Ranch Hand | Medium | Cowboy hat | — |
| Mechanic | Thin | Cap | Wrench |
| Frontier Doc | Thin | — | — |
| Outlaw | Medium | Bandana | — |
| Preacher | Thin | Flat-top | — |
| Saloon Keep | Stocky | — | — |
| Drifter | Thin | Hood | Knife |
| Engineer | Medium | Hard hat | — |
| Convict | Stocky | — | — |
| Scout | Medium | Wide-brim | — |

The hat is the strongest visual identifier at distance. "The one with the wide-brim hat" = the sheriff. "The hooded one" = the drifter.

## Implementation Phases

### Phase 1: Body + Head (Minimum Viable Sprites)
- 24×24 body atlas (procedural placeholder: colored rectangles with head circle)
- 12×12 head atlas (procedural: skin circle + hair shapes)
- Replace circle rendering with sprite sampling
- Direction from angle (8-way)
- Color tinting from existing pleb palette
- Head at same direction as body (no independence yet)

### Phase 2: Head Independence + Hats
- Independent head_angle field
- CPU logic: head turns toward points of interest
- Hat overlay layer
- Backstory default hats from chargen

### Phase 3: Walk Animation + Items
- 3 walk frames for body
- Held item overlay at hand position
- Carried object above head
- Tool swaps based on current activity

### Phase 4: Art Replacement + Outline
- Replace procedural placeholders with hand-drawn pixel art
- Dynamic outline rendering
- Selection outline replaces pulsing ring
- Corpse rendering

## Art Direction: The Rimworld Look

Rimworld's characters read as vector graphics but are actually raster sprites drawn with a specific formula: **thick black outlines + flat color fills + simple rounded shapes**. No gradients, no internal shading, no texture. The outline does all the work — it defines the silhouette. The fill just says "red shirt" or "dark skin."

This is why Rimworld characters are readable at any zoom. The outline is high-contrast, resolution-independent in feel, and carries all the shape information.

### How to Achieve This in Rayworld

**Draw sprites WITHOUT outlines.** Just flat-colored fills in the right shapes — solid blobs of color, no careful pixel-perfect outline work. The shapes should be simple and rounded (ellipse body, circle head, rounded hat shapes).

**Shader adds outlines dynamically.** The neighbor-check outline (transparent pixel adjacent to opaque pixel = draw outline) produces clean 1px outlines around any sprite shape automatically. This gives the Rimworld clean look PLUS runtime control:

| State | Outline Color | Thickness |
|-------|--------------|-----------|
| Normal | Black | 1px |
| Selected | Green | 1-2px |
| Injured | Red pulse | 1px |
| Enemy | Red | 1px |
| Stressed | Orange | 1px |
| Zoomed far out | None (skip for clarity) | 0 |
| Zoomed in | Black | 2px |

Outlines react to game state without redrawing any sprites.

### The Three Ingredients

1. **Strong outlines** — shader handles this. Free, dynamic, state-reactive.
2. **Flat fills, no internal shading** — draw sprites as solid colors per zone. The color tinting system (R=luminance, G=zone) naturally produces this: each zone is a single tinted color.
3. **Simple rounded shapes** — the art direction part. Body is an ellipse. Head is a circle. Hat is a rounded shape on top. Keep geometry simple and silhouettes distinct.

### Sprite Production Implication

This makes sprites MUCH easier to produce than traditional pixel art:

- No anti-aliasing (shader outline is crisp)
- No shading (flat fills only)
- No outline pixel work (shader does it)
- Just: draw the shape, fill the zones with the right gray values for the tinting system

A competent pixel artist could produce all ~244 sprites in a day. Procedural generation could produce acceptable placeholders in code (filled ellipses and circles with zone-encoded gray values).

### Consistency with Block Rendering

The existing block rendering is procedural with noise-based textures (wood grain, stone cracks). Plebs with flat-fill sprites against noisy textured terrain creates a natural visual hierarchy: characters POP against the environment because they're cleaner and simpler. This is the Rimworld effect — characters are the clearest, most readable thing on screen.

## Open Questions

1. **Do enemies use the same sprite system?** Probably simpler — 1 body type, 1 color, no hats. Redskulls just need to be visually distinct (red tint, different body silhouette).
2. **Animal sprites?** If animals are added, they'd need their own atlas. Same compositing pattern but different layer structure.
3. **Zoom-dependent LOD?** At far zoom, skip head/hat layers and just render tinted body? Saves GPU work when many plebs are visible.
4. **Mirroring for E/W symmetry?** E-facing sprites could be W-facing sprites mirrored. Cuts direction sprites roughly in half. Slight asymmetry (left-hand vs right-hand) might look wrong though.

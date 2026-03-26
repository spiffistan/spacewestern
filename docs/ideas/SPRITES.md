# Practical Sprite Design

How sprites actually work in Rayworld, what's needed, and how to produce them efficiently.

## Current Rendering Architecture

Rayworld uses **three distinct rendering approaches** — understanding which category each visual falls into determines the sprite strategy.

### 1. Procedural Shader Shapes (blocks, furniture, equipment)

Most blocks are drawn directly in `raytrace.wgsl` using math — circles, rectangles, gradients, noise. Examples: walls are flat colored with grain noise, benches use `render_bench()` which draws planks with wood grain, beds use `render_bed()` with pillow/blanket shapes.

**No sprites needed.** The shader *is* the art. Changing a block's look means editing WGSL functions. This is fast to iterate (change code → see result) but requires shader programming skill.

### 2. Heightmap Sprites (trees, bushes)

Trees use 16x16 RGBA sprites where RGB = color and A = height. Eight procedural variants are generated in `sprites.rs` and uploaded to a GPU storage buffer. The shader samples the sprite per-pixel, giving each tree a unique silhouette with proper height for shadow casting.

**This is the system to expand.** Any organic/complex shape that doesn't reduce to simple geometry benefits from sprites: rocks, ruins, vehicles, large props.

### 3. Pleb Rendering (characters)

Plebs are drawn as layered circles in the shader — pants ring, shirt disc, skin head, hair patch, direction dot. Each pleb has GPU-side color values (shirt_r/g/b, skin_r/g/b, etc.) so individuals look different. Simple but readable at the game's zoom level.

**Could be upgraded to sprites** for more character, but the current system works well at typical zoom.

---

## What Actually Needs Sprites

Given the card system, chargen, and general art direction, here's what needs visual design:

### Tier 1: Needed Now

| Asset | Count | Size | Purpose |
|-------|-------|------|---------|
| **Card frame** | 5 suit variants | 200x300px (UI) | egui-rendered event/blueprint/ability cards |
| **Card icons** | ~30 | 32x32px (UI) | Trait/skill/backstory symbols for chargen cards |
| **Item icons** | ~25 | 16x16 or 24x24px (UI) | Inventory, crafting, resource bar |
| **Backstory portraits** | 12 | 48x64px (UI) | Chargen dossier cards |

### Tier 2: Would Improve the Game

| Asset | Count | Size | Purpose |
|-------|-------|------|---------|
| **Tree variants** | 8 (exists) | 16x16 heightmap | Already procedural, could be Blender-rendered |
| **Rock formations** | 4-6 | 16x16 heightmap | Replace flat circles with shaped boulders |
| **Ruin pieces** | 6-8 | 16x16 heightmap | Explorable ruins for blueprint discovery |
| **Pleb sprites** | per-backstory | 16x16 heightmap | Replace circle-plebs with tiny figures |

### Tier 3: Polish

| Asset | Count | Size | Purpose |
|-------|-------|------|---------|
| **Crop growth stages** | 4 per crop | 16x16 heightmap | Visual farming progression |
| **Tool sprites** | 6-8 | 8x8 carried | Visible tools on pleb sprites |
| **Weather particles** | 3-4 | 4x4 | Rain, dust, snow, embers |
| **Status icons** | 8-10 | 12x12 (UI) | Stress, hunger, cold indicators above plebs |

---

## Production Approaches

### A. Procedural (Current Approach)

Write Rust code that generates pixel data mathematically.

**Pros:** Zero external dependencies, perfect consistency, infinite variants via parameter tweaking, no asset loading.

**Cons:** Hard to make things look "designed" rather than algorithmic. Organic shapes (faces, animals, detailed props) are painful. Every art change requires recompiling.

**Best for:** Trees (already done), terrain textures, particle effects, geometric objects.

### B. Blender Pipeline (Documented in DN-001)

Model in 3D → render orthographic top-down → export RGB + depth as RGBA sprite.

**Pros:** Full artistic control, natural-looking shapes, proper depth/height from real 3D geometry, fast iteration in Blender. Ambient occlusion baked in for free.

**Cons:** Requires Blender skill, the pipeline script needs finishing, asset management overhead, harder to do random variants (need multiple models).

**Best for:** Complex props (ruins, vehicles, large furniture), rock formations, anything where shape matters more than color.

### C. Hand-Drawn Pixel Art

Draw sprites directly in Aseprite, Pixeloracle, or any pixel editor. For heightmap sprites, paint the height channel manually (or derive from luminance).

**Pros:** Maximum artistic expression, fast for small sprites (icons, UI), personality and charm. A skilled pixel artist can make a 16x16 character more expressive than any procedural system.

**Cons:** Labor-intensive at volume, harder to maintain consistency across many assets, height channel is unintuitive to paint by hand.

**Best for:** UI icons, card art, portraits, item sprites, anything player-facing where personality matters.

### D. Hybrid: AI-Assisted + Hand-Cleaned

Use an image generator for initial drafts at higher resolution, then hand-clean and downscale to pixel art. Paint the height channel manually.

**Pros:** Fast concepting, generates variety quickly, human cleanup ensures quality.

**Cons:** Consistency across assets requires careful curation, style drift between generated batches.

**Best for:** Rapid prototyping of many variants, card art backgrounds, portrait drafts.

---

## Practical Recommendations

### Card Art (UI-Only, Not Heightmaps)

Cards are rendered in egui, not in the raytrace shader. They're standard 2D images — no height channel needed.

**Approach:** Design card frames as 9-slice images (corners + edges + fill) so they scale. Draw suit symbols and trait icons as small pixel art sprites. Portraits can be slightly higher resolution since they're UI-only.

**Format:** PNG loaded at startup. Embed with `include_bytes!` for WASM compatibility.

**Style note:** Card art should be the most polished visual in the game. If the overall aesthetic is muted/gritty, cards should be the exception — vibrant, detailed, collectible-feeling. They're the interface between player and narrative.

### Item Icons

Currently items show as text labels in the resource bar and inventory. Icons would replace text.

**Approach:** Hand-drawn pixel art, 16x16 or 24x24. Small palette per icon (4-6 colors). Slight outline for readability against any background.

**Batch strategy:** Draw all icons on a single sprite sheet (8 columns). Load as texture atlas. Reference by item ID → atlas position.

### Pleb Sprites (If Upgrading)

The current circle-based pleb rendering is charming and readable. But if upgrading:

**Option 1: Bigger circles with detail.** Keep the layered-disc approach but add 2-3 more layers — belt, boots, hat. Hats especially would add western character. A sheriff gets a wide-brim hat, outlaw gets a bandana, prospector gets a wide hat, etc. This stays fully procedural, no sprite loading needed.

**Option 2: Heightmap sprite plebs.** Create 16x16 top-down character sprites with height data. 4 rotation frames (N/E/S/W) × backstory variants. The shader already handles heightmap sprites for trees — same system. This gives proper silhouettes and shadows but requires 48+ sprite frames (12 backstories × 4 directions).

**Option 3: Minimal upgrade.** Keep circles but add a single distinguishing feature per backstory: hat shape rendered as a displaced circle above the head. Sheriff = wide circle, doc = no hat, outlaw = bandana (half-circle), preacher = flat-top hat. 2-3 lines of shader code per variant.

**Recommendation:** Option 3 first (hats via shader), then Option 2 if the game grows enough to justify the asset investment.

### Heightmap Sprites for World Objects

For trees, rocks, ruins — the existing heightmap sprite system works well. Key considerations:

**Resolution:** 16x16 is the sweet spot. At typical zoom level (camera height ~15-20 blocks), each sprite pixel maps to roughly 2-4 screen pixels. Going to 32x32 doubles the detail but quadruples memory and is barely noticeable.

**Variant count:** 4-8 variants per object type prevents pattern repetition. The shader selects variant by position hash, so adjacent trees always look different.

**Height values:** The alpha channel directly controls shadow casting and 3D appearance. A tree trunk at A=60 (low) with canopy peaking at A=220 (tall) creates a natural height profile. Getting these values right matters more than color detail.

**Palette discipline:** Use the same base palette across all sprites for visual coherence. Define 4-5 green shades for vegetation, 3-4 brown shades for wood/earth, 3 grey shades for stone. Reuse across all heightmap sprites.

---

## Asset Pipeline Summary

```
Procedural (sprites.rs)          →  GPU storage buffer  →  raytrace.wgsl
  Trees, particles, terrain            heightmap sprites      samples per-pixel

Blender (DN-001 pipeline)        →  PNG files           →  loaded at startup
  Ruins, rocks, complex props          RGBA heightmaps        same GPU buffer

Hand-drawn pixel art             →  PNG sprite sheet    →  egui / UI rendering
  Icons, cards, portraits              standard RGBA          texture atlas

Shader code (raytrace.wgsl)      →  (no assets)         →  direct rendering
  Walls, floors, furniture,            math only              per-block functions
  plebs, equipment, pipes
```

## Open Questions

1. **Card size in UI** — How large should cards render on screen? Full-screen takeover during events? Corner notification? Draggable hand along the bottom?
2. **Animation** — Do we want animated sprites (swaying trees, flickering torches)? The shader can fake this with UV offset math on static sprites, no extra frames needed.
3. **Seasonal variation** — Swap tree sprite palette by season? Same shapes, different greens/oranges/bare?
4. **Zoom levels** — Do we need LOD? At far zoom, sprites could simplify to single-color dots. At near zoom, show full detail. Currently one LOD fits all.

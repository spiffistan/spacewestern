# Sprite Pipeline: Blender to Game

## Overview

Sprites are rendered as **flat albedo only** (no baked lighting). The game's raytrace shader applies all dynamic lighting — sun direction, shadows, day/night cycle, proximity glow, ambient occlusion — identically to procedural blocks. This ensures sprites look natural at any time of day and respond to nearby light sources.

## Blender Setup

### Camera
- **Orthographic** projection
- **45° from vertical** (looking south-downward) — reveals the layered structure of 3D objects while keeping a top-down game feel
- Frame the object to fill ~70-80% of the sprite area
- Output resolution: 256x256 or 512x512 for detailed sprites (trees, furniture); 64x64 or 128x128 for small ground items

### Lighting
- **No directional light, no shadows**
- Use Blender's **Workbench** renderer with **Flat** shading — fastest, captures pure material color
- Alternatively: assign Emission shaders to all materials (color only, strength 1.0)
- Goal: capture ONLY the object's material colors as seen from above

### Materials
- Assign distinct flat colors per part (wood=brown, metal=gray, skin=beige, fabric=colored)
- No PBR (no roughness, metallic, subsurface) — invisible at sprite resolution
- **Use vertex colors** for within-object texture variation (noise-based painting)
- Workbench displays vertex colors with `color_type = 'VERTEX'`
- Displace outer vertices with noise for organic silhouette edges (avoid perfect geometric shapes)
- Character materials should match the game's color system: skin_rgb, hair_rgb, shirt_rgb, pants_rgb

### Render Settings
- **Transparent background** (RGBA output, Film > Transparent in Cycles, or Alpha in Workbench)
- PNG output with alpha channel
- Color management: Standard (not Filmic) to preserve flat material colors
- **Cavity** enabled: valley_factor=2.0, ridge_factor=0.3, type=BOTH. This darkens creases where canopy clusters overlap, giving depth separation without baking directional light.
- **UV spheres** over icospheres — icosphere triangles create visible artifacts at 256px; UV sphere quads are smoother
- **Per-face vertex colors** for texture variation — one noise sample per polygon center, not per-vertex (per-vertex loops crash Blender MCP at high vertex counts)

## What to Render

### Characters (Plebs)

Animated, 4 directions, multiple states:

```
pleb_N_idle.png
pleb_N_walk1.png, pleb_N_walk2.png, pleb_N_walk3.png
pleb_E_idle.png
pleb_E_walk1.png, pleb_E_walk2.png, pleb_E_walk3.png
pleb_S_idle.png
pleb_S_walk1.png, pleb_S_walk2.png, pleb_S_walk3.png
pleb_W_idle.png
pleb_W_walk1.png, pleb_W_walk2.png, pleb_W_walk3.png
```

**Modeling tips:**
- Simple low-poly humanoid (box body, cylinder limbs)
- Basic armature: spine, 2 arms, 2 legs (5 bones minimum)
- Walk cycle: 3-4 key poses, rendered as separate frames
- Scale: character should fill ~60-70% of the sprite frame (leave room for arms swinging)

**Color customization:**
- Render with placeholder colors (white skin, gray shirt, gray pants)
- The game shader tints sprite pixels at runtime using per-pleb color assignments
- OR: render multiple color variants (more storage, simpler shader)

### Furniture (Static)

1-4 rotations depending on symmetry:

```
bench_H.png, bench_V.png          (2 rotations, symmetric)
bed_head.png, bed_foot.png        (2-tile piece, N/S and E/W variants)
crate.png                         (1 rotation, symmetric)
cannon_N.png, cannon_E.png, ...   (4 rotations)
fireplace.png                     (1 rotation, symmetric)
```

### Ground Items

1 view each, small sprites (12x12 or 16x16):

```
berries.png
wood_log.png
rock.png
fiber.png
```

### Trees

Blender-rendered sprites (256x256, 8 species × 4 variants = 32 atlas entries). Each pixel packs albedo + height for shadow casting through canopy.

**Species** (atlas slots):
- **0: Conifer** (slots 0-3) — layered cones, dark→light green. Grass/rocky terrain.
- **1: Oak** (slots 4-19) — round dome canopy from 11 overlapping UV sphere lobes, gnarled trunk with bark texture. 16 variants with canopy size ranging young→mature. See `docs/OAK_BUILD.md` for full build documentation. Grass/loam terrain.
- **2: Scrub bush** (slots 8-11) — low wide lumps, no trunk. Chalky/gravel/rocky.
- **3: Dead tree** (slots 12-15) — bare branches, sparse leaf tips. Peat/clay.
- **4: Yucca** (slots 16-19) — forking arms, spiky clusters. Rocky/clay/gravel.
- **5: Willow** (slots 20-23) — drooping dome + skirt canopy. Marsh/peat.
- **6: Poplar** (slots 24-27) — tall narrow columnar. Loam/grass.
- **7: Birch** (slots 28-31) — white trunk, sparse leaf clusters. Grass/loam.

Atlas index formula: `sprites[(species * 4 + variant) * 256² + y * 256 + x]`

**How in-game lighting works on sprites** (from `raytrace.wgsl`):
1. `render_tree()` samples the sprite atlas → returns `vec4(albedo_rgb, height)`
2. The sprite albedo is treated as a **pure diffuse color** — no baked light
3. The raytrace shader applies dynamically: `color = albedo * (ambient + sun_color * shadow * 0.85)`
4. Shadow map uses the alpha/height channel to cast per-pixel shadows from canopy
5. Proximity glow from nearby torches/lamps adds warm light to trunk and lower canopy
6. `foliage_opacity` uniform controls canopy transparency (see-through mode)
7. Per-tree `id_hash` selects species+variant and tints color by ±15%

Output: `assets/sprites/tree_atlas_all.bin` (8 MB, 32 entries × 256² × u32).

## Sprite Atlas Packing

Individual PNGs are packed into a sprite atlas — a single large buffer uploaded to the GPU.

### Current Format (Trees)

```
sprites[variant * SPRITE_SIZE^2 + y * SPRITE_SIZE + x] = R | (G << 8) | (B << 16) | (HEIGHT << 24)
```

- SPRITE_SIZE: 256
- SPRITE_VARIANTS: 32 (8 species × 4 variants)
- R, G, B: albedo color (0-255 each)
- HEIGHT: normalized height above ground (0=transparent/ground, 1-255=height). Used for:
  - Determining if a pixel is part of the object (height > 0)
  - Shadow casting (taller parts block more light)
  - Trees: canopy vs trunk distinction

### Extended Format (New Sprites)

For animated/multi-directional sprites, the atlas uses a catalog:

```rust
struct SpriteEntry {
    atlas_offset: u32,   // index into sprite buffer
    width: u16,          // sprite width in pixels
    height: u16,         // sprite height in pixels
    origin_x: i8,        // render offset from tile center
    origin_y: i8,        // render offset from tile center
    object_height: u8,   // height for shadow casting (in block units)
    flags: u8,           // animation frame count, etc.
}
```

## Shader Integration

### Rendering

The raytrace shader samples the sprite and applies lighting:

```wgsl
let sprite = sample_sprite(sprite_id, local_x, local_y);
if sprite.a < 0.01 { /* transparent — show ground underneath */ }

// Albedo from sprite, lighting from game systems
let albedo = sprite.rgb;
let lit = albedo * (ambient + sun_color * shadow * 0.85);
let final = lit + proximity_glow * glow_mul;
color = final;
```

This is identical to how every other block is lit. The sprite is just a different source of `albedo`.

### Character Color Tinting

For plebs with customizable appearance:

```wgsl
// Sprite rendered with neutral gray body parts
// Tint at runtime based on pleb color data
let gray = (sprite.r + sprite.g + sprite.b) / 3.0;
if is_shirt_region { color = pleb.shirt_rgb * gray; }
if is_skin_region  { color = pleb.skin_rgb * gray; }
```

Region detection: use the sprite's green channel as a mask (e.g., G=255 for shirt, G=128 for skin, G=64 for pants). Or render separate mask sprites.

### Shadow Casting

Objects with height cast shadows in the shadow map:

```wgsl
// In shadow_map.wgsl, when sampling a sprite block:
let sprite_h = f32(sprite.a) / 255.0 * object_max_height;
if sprite_h > ray_height { /* shadow */ }
```

## Export Pipeline Script

A Python script automates Blender render → atlas:

```
1. blender --background scene.blend --python render_sprites.py
   → Renders all objects/animations to individual PNGs

2. python pack_atlas.py sprites/*.png --output atlas.bin
   → Packs PNGs into the u32 buffer format
   → Generates sprite catalog (offsets, sizes)

3. Copy atlas.bin → src/sprites/ (loaded at game startup)
```

## File Organization

```
assets/
  blender/
    conifer.blend           # conifer tree model (kept as template for future species .blend files)
    render_conifer.py       # single-species batch render script (template)
  sprites/
    raw/                    # individual rendered PNGs + preview sheets
    tree_atlas_all.bin      # packed sprite buffer (8 species × 4 variants × 256² × u32 = 8 MB)
    conifer_atlas_256.bin   # single-species atlas (legacy, 2 MB)
    catalog.json            # sprite index — names → offsets (future)
```

## Priority Order

1. **Trees** — ✅ conifer done (256/512, 8 variants, Blender-rendered). Next: oak, willow, birch, bush variants
2. **Plebs** — biggest visual impact, most visible, most animated
3. **Furniture** — bench, bed, crate, cannon, fireplace
4. **Ground items** — berries, wood, rocks
5. **Walls** — auto-tiled wall sprites (complex: 16-47 variants per material)

## Gotchas

- **Pixel alignment**: sprites must be pixel-aligned to the game grid to avoid sub-pixel shimmer. The shader should `floor()` the sprite UV, not interpolate.
- **Rotation**: for 4-direction sprites, render N/E/S/W separately in Blender rather than rotating a single render (rotation causes pixel artifacts at small sizes).
- **Animation speed**: walk cycle speed must match the pleb movement speed in the game (~3 tiles/sec = ~4-5 frames per tile at 60fps).
- **Color space**: render in sRGB, the game applies gamma in the blit shader. Don't double-gamma.
- **Transparent edges**: use premultiplied alpha to avoid dark halos around sprite edges.

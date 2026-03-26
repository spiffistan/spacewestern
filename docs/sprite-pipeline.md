# Sprite Pipeline: Blender to Game

## Overview

Sprites are rendered as **flat albedo only** (no baked lighting). The game's raytrace shader applies all dynamic lighting — sun direction, shadows, day/night cycle, proximity glow, ambient occlusion — identically to procedural blocks. This ensures sprites look natural at any time of day and respond to nearby light sources.

## Blender Setup

### Camera
- **Orthographic** projection
- Looking straight down (or ~10 degrees from south to match the game's oblique wall-face projection)
- Frame the object so 1 Blender unit maps to 1 output pixel
- Output resolution: 16x16, 24x24, or 32x32 depending on object size

### Lighting
- **No directional light, no shadows**
- Use Blender's **Workbench** renderer with **Flat** shading — fastest, captures pure material color
- Alternatively: assign Emission shaders to all materials (color only, strength 1.0)
- Goal: capture ONLY the object's material colors as seen from above

### Materials
- Assign distinct flat colors per part (wood=brown, metal=gray, skin=beige, fabric=colored)
- No PBR (no roughness, metallic, subsurface) — invisible at sprite resolution
- Character materials should match the game's color system: skin_rgb, hair_rgb, shirt_rgb, pants_rgb

### Render Settings
- **Transparent background** (RGBA output, Film > Transparent in Cycles, or Alpha in Workbench)
- PNG output with alpha channel
- No anti-aliasing (nearest-neighbor scaling) or minimal AA for clean pixel edges
- Color management: Standard (not Filmic) to preserve flat material colors

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

Already sprite-based (16x16, 8 variants). Could upgrade to 32x32 for more detail. Render with per-pixel height in alpha channel for shadow casting through canopy.

## Sprite Atlas Packing

Individual PNGs are packed into a sprite atlas — a single large buffer uploaded to the GPU.

### Current Format (Trees)

```
sprites[variant * SPRITE_SIZE^2 + y * SPRITE_SIZE + x] = R | (G << 8) | (B << 16) | (HEIGHT << 24)
```

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
    pleb.blend          # character model + rig
    furniture.blend     # bench, bed, crate, etc.
    items.blend         # ground items
    render_template.py  # batch render script
  sprites/
    raw/                # individual rendered PNGs
    atlas.bin           # packed sprite buffer
    catalog.json        # sprite index (names → offsets)
```

## Priority Order

1. **Plebs** — biggest visual impact, most visible, most animated
2. **Furniture** — bench, bed, crate, cannon, fireplace
3. **Ground items** — berries, wood, rocks
4. **Walls** — auto-tiled wall sprites (complex: 16-47 variants per material)
5. **Trees** — upgrade from 16x16 to 32x32

## Gotchas

- **Pixel alignment**: sprites must be pixel-aligned to the game grid to avoid sub-pixel shimmer. The shader should `floor()` the sprite UV, not interpolate.
- **Rotation**: for 4-direction sprites, render N/E/S/W separately in Blender rather than rotating a single render (rotation causes pixel artifacts at small sizes).
- **Animation speed**: walk cycle speed must match the pleb movement speed in the game (~3 tiles/sec = ~4-5 frames per tile at 60fps).
- **Color space**: render in sRGB, the game applies gamma in the blit shader. Don't double-gamma.
- **Transparent edges**: use premultiplied alpha to avoid dark halos around sprite edges.

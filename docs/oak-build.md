# Oak Tree Sprite — Build Documentation

## Overview

The oak tree sprite is rendered from a 3D Blender model at 256×256 pixels using the
Workbench renderer with flat shading and cavity for depth. The model produces 16 variants
with varying canopy size (young→mature) and trunk thickness/lean.

## Blender Scene Structure

The oak scene (`assets/blender/oak.blend`) contains:

### Trunk (`Trunk`)
- **Geometry**: 24-sided cylinder, subdivided 3×, giving 480 vertices
- **Shape**: Root flare at base (×1.3 radius), gradual taper to top (×0.35 radius)
- **Gnarled bends**: Noise-based XY offset along the Z axis creates organic curves
- **Bark ridges**: Radial vertex displacement from `noise.noise()` at frequency 10×10×4
- **Vertex colors**: Per-face bark texture painted with noise (freq 6×6×2.5)
  - Dark grooves: `(0.08, 0.05, 0.02)` — 15% of faces
  - Mid-dark bark: `(0.13, 0.08, 0.03)` — 15% of faces
  - Base bark: `(0.17, 0.10, 0.04)` — 35% of faces
  - Light ridges: `(0.22, 0.14, 0.06)` — 15% of faces
  - Exposed bark: `(0.28, 0.18, 0.08)` — 15% of faces (highest noise values)

### Branches (`Branch0`, `Branch1`, `Branch2`)
- **Geometry**: 10-sided cylinders, short (0.4–0.5 units long)
- **Mostly hidden** under canopy — visible between lobe gaps from above
- **Color**: `(0.15, 0.09, 0.04)` — slightly darker than trunk
- **Angles**: Spread outward from trunk top at various rotations

### Canopy (11 UV sphere lobes)
The canopy is built from **11 overlapping UV spheres** arranged in a circular pattern
to create a round, dome-shaped silhouette characteristic of oaks seen from above.

**Layout** (arranged as a central dome + ring + crown):

| Lobe | Radius | Position (x,y,z) | Scale Z | Height | Role |
|------|--------|-------------------|---------|--------|------|
| `C_core` | 0.65 | (0, 0, 1.80) | 0.45 | 0.65 | Central dome, largest |
| `C_e` | 0.50 | (0.35, 0, 1.72) | 0.42 | 0.58 | East ring lobe |
| `C_w` | 0.48 | (-0.32, 0.10, 1.74) | 0.43 | 0.60 | West ring lobe |
| `C_n` | 0.46 | (0.15, 0.32, 1.70) | 0.41 | 0.56 | North ring lobe |
| `C_s` | 0.44 | (0.10, -0.35, 1.68) | 0.40 | 0.55 | South ring lobe |
| `C_ne` | 0.42 | (0.30, 0.25, 1.73) | 0.42 | 0.58 | NE ring lobe |
| `C_sw` | 0.40 | (-0.28, -0.22, 1.71) | 0.41 | 0.57 | SW ring lobe |
| `C_nw` | 0.38 | (-0.22, 0.28, 1.72) | 0.40 | 0.56 | NW ring lobe |
| `C_se` | 0.36 | (0.25, -0.25, 1.69) | 0.39 | 0.55 | SE ring lobe |
| `C_crown` | 0.30 | (0.05, 0.05, 2.05) | 0.35 | 0.78 | Top crown, brightest |
| `C_crown2` | 0.25 | (-0.10, -0.08, 1.98) | 0.33 | 0.72 | Secondary crown |

**Canopy displacement**: Each lobe has medium vertex displacement applied:
- Large lumps: `noise(x*3.5, y*3.5, z*2.5) * 0.08`
- Fine detail: `noise(x*8, y*8, z*5) * 0.03`
- Displaced along vertex normal (outward/inward)

**Canopy vertex colors**: Per-face painting from an 8-color palette:
```
Index  RGB                  Role
0      (0.03, 0.08, 0.02)   Very dark gap/shadow
1      (0.04, 0.11, 0.03)   Dark
2      (0.05, 0.14, 0.03)   Dark-mid
3      (0.06, 0.17, 0.04)   Mid-dark
4      (0.07, 0.20, 0.05)   Base
5      (0.09, 0.23, 0.05)   Mid-light
6      (0.11, 0.26, 0.06)   Light
7      (0.13, 0.29, 0.07)   Bright tip
```

Each lobe has a `bias` that shifts which palette colors it favors:
- Ring lobes (lower/back): bias 1–2 → darker palette range
- Core: bias 3 → mid range
- Crown lobes: bias 4–5 → brighter palette range

Color is selected per-face using `noise(center * 5)` mapped to palette index + bias.
Fine jitter of ±0.012 is added per-face from high-frequency noise.

## Render Settings

- **Engine**: Workbench
- **Shading**: Flat light, Vertex color mode
- **Cavity**: Enabled, type=BOTH, valley_factor=2.0, ridge_factor=0.3
  - Valley darkens creases where canopy lobes overlap (depth separation)
  - Ridge subtly brightens exposed outer edges
- **Background**: Transparent (RGBA)
- **Resolution**: 256×256
- **Color management**: Standard (not Filmic)
- **Filter size**: 0.5
- **Camera**: 45° from vertical, orthographic, ortho_scale=4.5, tracking CamTarget at (0,0,1.5)

## Height Materials

Every mesh object has a corresponding `H_{name}` material with a grayscale
diffuse color representing its normalized height (0.0=ground, 1.0=tallest).
The height pass is rendered by swapping all materials to their H_ versions,
disabling cavity, and switching to Material color mode.

Height values: Trunk=0.20, Branches=0.40–0.52, Ring lobes=0.55–0.60, Crown=0.72–0.78.

## Variant Generation (16 variants)

Each variant uses a deterministic seed (`700 + v * 37`) and applies:

### Canopy size range
- `overall = 0.75 + (v / 15) * 0.60` → ranges from 0.75 (v0, young) to 1.35 (v15, mature)
- Multiplied by random jitter ×0.92–1.08

### Trunk scaling (nonlinear — small trees get proportionally smaller trunks)
- `trunk_scale = overall * (0.6 + overall * 0.3)`
- This ensures the canopy always covers the trunk, even at smallest sizes
- XY scale jitter ×0.88–1.12, oval variation between X and Y
- Z scale (height) jitter ×0.90–1.10
- Lean: 2–8° in random direction

### Branch scaling
- Matches trunk_scale with ×0.85–1.15 jitter
- Z rotation jitter ±0.3 radians

### Canopy per-lobe jitter
- Scale: overall × (0.88–1.12) per axis
- Position: ±0.08–0.10 × overall units in XY
- Rotation: ±0.4 radians around Z

## Atlas Packing

Each variant produces two renders:
1. **Albedo** (`oak16_v{N}_a.png`): Vertex colors + cavity, transparent background
2. **Height** (`oak16_v{N}_h.png`): Grayscale height materials, no cavity

These are combined per-pixel:
```
if albedo_alpha > 0.01 and height_alpha > 0.01:
    packed = R | (G << 8) | (B << 16) | (HEIGHT << 24)
else:
    packed = 0  # transparent
```

16 variants × 256² pixels × 4 bytes = 4,194,304 bytes (4 MB) per species.

## Lessons Learned / Blender MCP Constraints

1. **No large scripts via `exec(open(...).read())`** — causes Blender to hang.
   Do everything as small sequential MCP calls.

2. **Per-vertex noise loops crash Blender** at high vertex counts.
   Use **per-face** noise (one sample per polygon center) instead.

3. **Vertex color painting with 3 subdivisions** on multiple objects is too heavy.
   Use **2 subdivisions max**, or better yet, use per-face colors on the base geometry.

4. **UV spheres over icospheres** — icosphere triangles create visible artifacts
   at this resolution. UV sphere quads look much smoother.

5. **Cavity rendering** (valley=2.0, ridge=0.3) gives the best depth separation
   without baking directional light. Pure flat shading makes overlapping shapes
   merge into blobs; studio lighting looks fake.

6. **Functions don't persist between Blender MCP calls** — define everything
   inline or re-source utility files each time.

7. **State resets on Blender crash** — save `.blend` files frequently.
   Store original transforms before jittering variants so you can reset cleanly.

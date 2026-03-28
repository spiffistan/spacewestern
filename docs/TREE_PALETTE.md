# Tree Sprite Color Palette

All tree species share these lighting constraints:
- Rendered as flat albedo ONLY (Workbench, Flat, Material/Vertex color)
- Camera: 45° ortho, Standard color management
- The raytrace shader multiplies by `(ambient + sun_color * shadow * 0.85)`
- At noon: ambient ~(0.25,0.25,0.30), sun_color ~(1.0,0.95,0.85), so albedo × ~1.1
- At night: ambient only ~(0.05,0.05,0.08), so albedo × ~0.06
- Material colors should be in the 0.05–0.35 range for greens (higher clips in sun)

## Foliage Value Ranges (green channel as reference)

| Role | G range | Notes |
|------|---------|-------|
| Deep shadow | 0.12–0.18 | Interior canopy, shaded undersides |
| Base foliage | 0.18–0.26 | Main canopy mass |
| Light tips | 0.26–0.35 | Sun-facing crown, young growth |
| Accent | 0.20–0.30 | Color variation patches |

## Bark Value Ranges

| Type | RGB | Notes |
|------|-----|-------|
| Dark bark (conifer, oak) | (0.15–0.22, 0.08–0.14, 0.03–0.07) | Most trunks |
| Light bark (birch) | (0.65–0.80, 0.62–0.78, 0.55–0.70) | White bark species |
| Pale bark (yucca) | (0.25–0.35, 0.18–0.28, 0.10–0.18) | Desert species |

## Species Palettes

### 0: Conifer
- Shadow: (0.05, 0.16, 0.04)
- Base: (0.08, 0.22, 0.06)
- Light: (0.12, 0.30, 0.08)
- Accent: (0.10, 0.26, 0.05)
- Bark: (0.18, 0.10, 0.04)

### 1: Oak
- Shadow: (0.06, 0.14, 0.04)
- Base: (0.07, 0.20, 0.05)
- Light: (0.12, 0.28, 0.07)
- Accent: (0.10, 0.24, 0.06)
- Bark: (0.20, 0.12, 0.05)

### 2: Scrub Bush
- Shadow: (0.08, 0.14, 0.05)
- Base: (0.14, 0.22, 0.10)
- Light: (0.20, 0.30, 0.14)
- Accent: (0.18, 0.26, 0.08)
- Note: Olive/sage tones, muted

### 3: Dead Tree
- Bark: (0.16, 0.12, 0.06) (main color — minimal foliage)
- Sparse leaf: (0.12, 0.18, 0.06)

### 4: Yucca / Joshua Tree
- Leaf: (0.18, 0.24, 0.12) (dusty grey-green)
- Bark: (0.26, 0.20, 0.12) (pale desert bark)

### 5: Willow
- Shadow: (0.04, 0.14, 0.03) (very dark drooping fronds)
- Base: (0.06, 0.20, 0.05)
- Light: (0.10, 0.28, 0.08)
- Bark: (0.20, 0.14, 0.06)

### 6: Poplar / Cypress
- Shadow: (0.02, 0.08, 0.02) (nearly black-green)
- Base: (0.04, 0.14, 0.03)
- Light: (0.08, 0.22, 0.06)
- Note: Darkest species overall

### 7: Birch
- Shadow: (0.06, 0.18, 0.05)
- Base: (0.12, 0.30, 0.08)
- Light: (0.18, 0.38, 0.12)
- Bark: (0.70, 0.68, 0.60) (distinctive white)
- Note: Brightest foliage to contrast white trunk

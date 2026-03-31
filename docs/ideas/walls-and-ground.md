# Walls and Ground: Rendering Assessment

Critical look at current wall and ground rendering, what works, what doesn't, and what approach to take.

## Current Architecture

Three rendering paths exist:

1. **Procedural shader** — most blocks, walls, floors. Math in `raytrace.wgsl` draws per-pixel detail using noise, geometry, world position.
2. **Heightmap sprites** — trees, bushes. 16x16 RGBA where A=height. Stored in GPU buffer, sampled per-pixel.
3. **Flat color fallback** — `block_base_color()` returns a single vec3 per block type. Many blocks still land here.

## Ground: Already Strong

The `terrain_detail()` function (~180 lines of shader) is genuinely sophisticated:

- **Bilinear terrain blending** — hermite interpolation between neighboring terrain types eliminates hard tile boundaries
- **Individual grass blades** — per-pixel blade rendering with wind sway, short + long grass variants
- **Wildflowers** — scattered colored dots in high-vegetation areas
- **Pebbles/stones** — density driven by roughness parameter
- **Per-terrain-type detail** — chalk streaks and fragments, peat wet patches, marsh water glints
- **Data-driven** — terrain buffer fields (vegetation, compaction, grain, roughness, moisture) all affect rendering
- **Compaction paths** — foot traffic darkens and smooths the ground, creating visible paths
- **Elevation shading** — hillshade from terrain normals + altitude brightness + terrain AO

### Why Sprites Would Be a Downgrade for Ground

- **No tiling** — procedural noise is world-position-based, never repeats. Any sprite texture tiles visibly across plains.
- **Seamless blending** — bilinear interpolation between terrain types. Sprites can't do this without complex edge-blending.
- **Data reactivity** — rain darkens ground, fire scorches it, walking compacts it. Procedural responds to state; sprites are static.
- **Resolution independence** — zoom in and grass blades stay sharp (they're math). A 16x16 sprite turns to mush.

### Ground Weakness: Indoor Floors

The one gap. When you build a room with wood floors or stone floors, the interior falls through to `block_base_color()` — a single flat brown or grey. No plank lines, no tile pattern, no grain. Built interiors look bland compared to the lush outdoor terrain.

## Walls: Needs Work

### Wall Faces (South-Side Oblique View)

Mixed quality:
- **Glass** — excellent. Window with sill, lintel, crossbars, specular highlight, frame detail.
- **Insulated** — good. Visible fiber core between outer panels, panel divider lines.
- **Mud** — good. Bulging profile, craggy cracks, straw fibers, rounded top.
- **Generic wall/stone/wood** — weak. Just vertical mortar lines at `fract(fx * 4.0)`. No brick coursework, no plank grain, no material identity.

### Wall Tops (Seen From Above)

Mostly flat color from `block_base_color()`. The one exception: `BT_STONE` blocks get `stone_detail()` with cracks, mineral veins, and strata banding. Every other wall type — wood, mud, insulated, limestone — has zero top-surface detail. From above, they're colored rectangles.

## The Right Approach: Not Sprites

Sprites are the wrong tool for walls and floors because:

- Walls and floors tile across large areas — sprite repetition becomes visible
- The shader already has per-pixel world-position access — procedural textures are trivially seamless
- A sprite system would need texture atlases, UV sampling, and tile-edge blending — added complexity for a visual downgrade
- The terrain detail system already proves procedural works beautifully for this game

### Material-Specific Shader Functions (The Right Tool)

Extend the existing pattern. `stone_detail()`, `terrain_detail()`, `render_bench()` all prove the model works. What's missing:

**Wood floor** — Planks running in one direction (flag-based). Visible grain lines. Knot holes. Nail dots at plank ends. Slight color variation per plank.

**Stone floor** — Rectangular flagstone pattern with grout lines. Subtle color variation per slab. Occasional crack.

**Wood wall top** — Plank grain visible from above, running along the wall's long axis. Dark gap lines between planks.

**Stone wall face** — Replace generic mortar lines with actual brick coursework. Staggered rectangles with mortar gaps, per-brick color variation.

**Wood wall face** — Horizontal or vertical planks with grain lines, visible joins, darkened gaps.

Each of these is 10-20 lines of shader code. No asset pipeline. No loading. Instant iteration.

## Where Sprites ARE Better Than Procedural

Sprites win for:

- **Complex silhouettes** — bookshelves, weapon racks, paintings. Shapes painful to describe mathematically.
- **Signs and markings** — text or symbols that need to be readable. Can't draw "SALOON" with noise functions.
- **Unique landmarks** — crashed ships, graveyard crosses, totem poles. One-off shapes that justify hand-crafting.
- **Card art and UI** — obviously 2D image territory.

For these, the existing heightmap sprite system (tree pipeline) extends naturally. But don't use it for repeating surfaces.

## Summary

| Surface | Current State | Right Approach |
|---------|--------------|----------------|
| Outdoor ground | Strong (terrain_detail) | Keep procedural, minor tweaks |
| Indoor wood floor | Flat color | Procedural plank function |
| Indoor stone floor | Flat color | Procedural flagstone function |
| Wall faces (generic) | Weak mortar lines | Per-material procedural detail |
| Wall faces (glass/mud/insulated) | Good | Already done |
| Wall tops | Flat color (except stone) | Per-material procedural detail |
| Unique objects | N/A | Heightmap sprites |
| UI / cards | N/A | Standard 2D sprites |

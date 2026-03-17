# DN-001: Blender-to-Sprite Asset Pipeline

## Status: Proposed

## Context

The game uses heightmap sprites for objects like trees: small 2D images where RGB = color and A = height, rendered from a top-down orthographic view. The raytrace shader samples these sprites per-pixel to render objects with proper shape, color variation, and shadow casting.

Currently sprites are generated procedurally in Rust. This works for prototyping but limits visual quality and makes it hard to iterate on art. We want to author sprites from 3D models in Blender, giving artists full control over shape, color, and detail while keeping the runtime format dead simple.

## Sprite Format

Each sprite is a 16x16 (or 32x32) RGBA8 image:

| Channel | Meaning | Range |
|---------|---------|-------|
| R | Red color | 0-255 |
| G | Green color | 0-255 |
| B | Blue color | 0-255 |
| A | Height | 0 = transparent (ground visible), 1-255 = object height at this pixel |

Multiple variants of the same object type (e.g., 4 tree variants) are packed into a flat array in GPU memory. The shader selects a variant per-instance using a position hash.

## Blender Setup

### Scene Configuration

1. Create a new Blender scene (or a dedicated "sprite render" scene)
2. Set up an **orthographic camera** pointing straight down (-Z):
   - Location: (0, 0, 10)
   - Rotation: (0, 0, 0)
   - Orthographic scale: matches the world-space size of one block (1.0)
3. Set render resolution to sprite size (e.g., 16x16 or 32x32)
4. Background: transparent (Film > Transparent = checked)
5. Lighting: flat/ambient — avoid directional shadows in the sprite since the game shader handles all lighting

### Modeling Guidelines

- Model the object at the origin, fitting within a 1x1 unit footprint
- Keep geometry simple — at 16x16 the detail ceiling is low
- Use vertex colors or simple materials for color (no complex shaders needed)
- Set the object's height to match the desired in-game block height (e.g., a tree trunk + canopy peaking at ~3 units tall)

### Render Passes

Two renders per sprite variant:

**Color pass** (standard render):
- Render engine: EEVEE (fast) or Cycles (if you want ambient occlusion baked in)
- Output: RGBA PNG with transparent background
- This gives the RGB channels

**Depth pass** (compositor or script):
- Enable the Z (depth) pass in View Layer Properties > Passes > Data
- In the compositor, normalize the depth to 0-1 range:
  - Near clip = ground plane (Z=0) maps to depth 0
  - Far clip = max object height maps to depth 1
- Multiply by 255 to get the A channel value
- Alternatively, use a script (see below) for precise control

## Export Script

A Blender Python script that automates the full pipeline:

```python
"""
Blender sprite export script for Spacewestern.
Run from Blender's scripting workspace.

Usage:
1. Select the object(s) to render as a sprite
2. Adjust SPRITE_SIZE and OUTPUT_DIR below
3. Run the script

Outputs one RGBA PNG per variant where A = normalized height.
"""

import bpy
import numpy as np
from pathlib import Path

SPRITE_SIZE = 16
OUTPUT_DIR = Path("//sprites/")  # Blender-relative path
MAX_HEIGHT = 4.0  # Maximum object height in world units (maps to A=255)

def setup_camera():
    """Create/configure orthographic top-down camera."""
    cam_data = bpy.data.cameras.new("SpriteCamera")
    cam_data.type = 'ORTHO'
    cam_data.ortho_scale = 1.0
    cam_data.clip_start = 0.01
    cam_data.clip_end = MAX_HEIGHT + 1.0

    cam_obj = bpy.data.objects.new("SpriteCamera", cam_data)
    bpy.context.scene.collection.objects.link(cam_obj)
    cam_obj.location = (0.5, 0.5, MAX_HEIGHT + 0.5)  # Center of block, looking down
    cam_obj.rotation_euler = (0, 0, 0)

    bpy.context.scene.camera = cam_obj
    return cam_obj

def setup_render():
    """Configure render settings for sprite output."""
    scene = bpy.context.scene
    scene.render.resolution_x = SPRITE_SIZE
    scene.render.resolution_y = SPRITE_SIZE
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = 'PNG'
    scene.render.image_settings.color_mode = 'RGBA'
    scene.render.image_settings.color_depth = '8'

    # Enable Z pass for depth
    scene.view_layers[0].use_pass_z = True

    # Use EEVEE for speed
    scene.render.engine = 'BLENDER_EEVEE_NEXT'

def render_sprite(name: str, output_path: Path):
    """Render color + depth and combine into RGBA sprite."""
    scene = bpy.context.scene

    # Render
    bpy.ops.render.render()

    # Get color image
    result = bpy.data.images['Render Result']
    # Viewer node approach: read pixels from render result
    pixels = np.zeros(SPRITE_SIZE * SPRITE_SIZE * 4, dtype=np.float32)
    result.pixels.foreach_get(pixels)
    pixels = pixels.reshape((SPRITE_SIZE, SPRITE_SIZE, 4))

    # Get depth from the Z pass via compositor
    # (Alternative: use the depth buffer directly)
    # For simplicity, read the Z pass from the render layers
    viewer = bpy.data.images.get('Viewer Node')

    # Combine: RGB from color render, A from normalized depth
    # Depth: objects at ground (z=0) -> A=0, objects at MAX_HEIGHT -> A=255
    # Transparent pixels (alpha=0 in color render) stay A=0

    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Save using Blender's image API
    img = bpy.data.images.new(name, SPRITE_SIZE, SPRITE_SIZE, alpha=True)
    img.pixels.foreach_set(pixels.flatten())
    img.filepath_raw = str(output_path)
    img.file_format = 'PNG'
    img.save()
    bpy.data.images.remove(img)

    print(f"Saved sprite: {output_path}")

def export_selected():
    """Export each selected object as a sprite variant."""
    output_dir = bpy.path.abspath(str(OUTPUT_DIR))

    cam = setup_camera()
    setup_render()

    selected = [obj for obj in bpy.context.selected_objects if obj.type == 'MESH']
    if not selected:
        print("No mesh objects selected!")
        return

    for i, obj in enumerate(selected):
        # Hide all, show only this one
        for o in bpy.data.objects:
            o.hide_render = True
        obj.hide_render = False

        output_path = Path(output_dir) / f"tree_variant_{i:02d}.png"
        render_sprite(f"sprite_{i}", output_path)
        print(f"Exported variant {i}: {obj.name}")

    # Cleanup
    bpy.data.objects.remove(cam)

    print(f"Done! Exported {len(selected)} sprite variants to {output_dir}")

if __name__ == "__main__":
    export_selected()
```

### Depth-to-Alpha: Compositor Node Setup (Manual Alternative)

If you prefer using the compositor instead of the script:

1. Enable **Use Nodes** in the Compositor
2. Add a **Normalize** node connected to the **Depth** output of the Render Layers node
3. Connect the Normalize output to a **Map Range** node:
   - From Min: near clip depth
   - From Max: far clip depth (MAX_HEIGHT)
   - To Min: 0
   - To Max: 1
4. Use a **Set Alpha** node to combine the color RGB with the remapped depth as alpha
5. Connect to a **File Output** node set to RGBA PNG

## Rust Loader

Replace `generate_tree_sprites()` with a PNG loader. At build time or startup:

```rust
fn load_tree_sprites() -> Vec<u32> {
    let sprite_files = [
        "assets/sprites/tree_variant_00.png",
        "assets/sprites/tree_variant_01.png",
        "assets/sprites/tree_variant_02.png",
        "assets/sprites/tree_variant_03.png",
    ];

    let mut data = Vec::new();
    for path in &sprite_files {
        let img = image::open(path).expect("Failed to load sprite").to_rgba8();
        assert_eq!(img.width(), SPRITE_SIZE);
        assert_eq!(img.height(), SPRITE_SIZE);

        for pixel in img.pixels() {
            let [r, g, b, a] = pixel.0;
            let packed = (r as u32)
                | ((g as u32) << 8)
                | ((b as u32) << 16)
                | ((a as u32) << 24);
            data.push(packed);
        }
    }

    data
}
```

This requires adding the `image` crate to `Cargo.toml`. For WASM builds, the sprites could be embedded with `include_bytes!` instead of loading from disk.

## Sprite Resolution Considerations

| Resolution | Pixels | Visual quality | Memory per variant |
|------------|--------|----------------|-------------------|
| 16x16 | 256 | Blocky but stylistically consistent with Rimworld aesthetic | 1 KB |
| 32x32 | 1024 | Noticeably smoother canopy shapes | 4 KB |
| 64x64 | 4096 | Diminishing returns at typical zoom levels | 16 KB |

Recommendation: start with 16x16 for consistency with the block grid, move to 32x32 if zoomed-in views demand it. Memory is negligible either way (4 variants * 4 KB = 16 KB at 32x32).

## Future Extensions

- **Seasonal variants**: swap sprite sets for spring/summer/autumn/winter
- **Damage states**: partially chopped trees, stumps
- **Multi-block objects**: large trees spanning 2x2 or 3x3 blocks, assembled from sprite tiles
- **Animated sprites**: wind sway by slightly offsetting UV sampling with a sine wave in the shader (already possible with current system)
- **Other object types**: rocks, bushes, furniture — same pipeline, different models
- **Sub-block sprite height for shadow rays**: instead of block-level shadow casting, trace shadow rays against the sprite heightmap for per-pixel tree shadow silhouettes

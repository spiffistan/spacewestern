#!/usr/bin/env python3
"""Batch render conifer sprites from Blender.

Usage:
    blender --background assets/blender/conifer.blend --python assets/blender/render_conifer.py

Outputs:
    sprites/raw/conifer_v{0-7}_albedo.png   - flat albedo with vertex color texture (256x256)
    sprites/raw/conifer_v{0-7}_height.png   - height map (grayscale, 256x256)
    sprites/raw/conifer_v{0-7}_final.png    - combined RGBA (RGB=albedo, A=height)
    sprites/conifer_atlas_256.bin           - packed u32 atlas (256x256, 8 variants)

Atlas format:
    sprites[variant * SPRITE_SIZE^2 + y * SPRITE_SIZE + x] =
        R | (G << 8) | (B << 16) | (HEIGHT << 24)

Camera: 45° from vertical (orthographic), looking south-downward.
Renderer: Workbench flat shading with vertex colors.
"""
import bpy
import random
import struct
import os
import math

NUM_VARIANTS = 8
SIZE = 256
OUTPUT_DIR = os.path.join(os.path.dirname(bpy.data.filepath), "..", "sprites")
RAW_DIR = os.path.join(OUTPUT_DIR, "raw")
os.makedirs(RAW_DIR, exist_ok=True)


scene = bpy.context.scene
scene.render.engine = 'BLENDER_WORKBENCH'
scene.render.film_transparent = True
scene.render.image_settings.file_format = 'PNG'
scene.render.image_settings.color_mode = 'RGBA'
scene.view_settings.view_transform = 'Standard'
scene.render.resolution_x = SIZE
scene.render.resolution_y = SIZE
scene.render.filter_size = 0.5

atlas_data = []
random.seed(42)

for v in range(NUM_VARIANTS):
    # Randomize foliage per variant
    for obj in bpy.data.objects:
        if obj.name.startswith("Foliage_Tier"):
            obj.rotation_euler.z = random.uniform(-0.3, 0.3)
            obj.scale.x = random.uniform(0.85, 1.15)
            obj.scale.y = random.uniform(0.85, 1.15)

    # --- Albedo pass (vertex colors) ---
    scene.display.shading.light = 'FLAT'
    scene.display.shading.color_type = 'VERTEX'
    albedo_path = os.path.join(RAW_DIR, f"conifer_v{v}_albedo.png")
    scene.render.filepath = albedo_path
    bpy.ops.render.render(write_still=True)

    # --- Height pass (material colors = grayscale height) ---
    scene.display.shading.color_type = 'MATERIAL'
    orig_mats = {}
    for obj in bpy.data.objects:
        if obj.type != 'MESH':
            continue
        orig_mats[obj.name] = [s.material for s in obj.material_slots]
        h_mat = bpy.data.materials.get(f"Height_{obj.name}")
        if h_mat:
            for s in obj.material_slots:
                s.material = h_mat

    height_path = os.path.join(RAW_DIR, f"conifer_v{v}_height.png")
    scene.render.filepath = height_path
    bpy.ops.render.render(write_still=True)

    # Restore materials
    for name, mats in orig_mats.items():
        obj = bpy.data.objects[name]
        for i, m in enumerate(mats):
            if i < len(obj.material_slots):
                obj.material_slots[i].material = m

    # --- Combine albedo + height into final sprite ---
    a_img = bpy.data.images.load(albedo_path)
    h_img = bpy.data.images.load(height_path)
    a_px = list(a_img.pixels)
    h_px = list(h_img.pixels)

    out = bpy.data.images.new(f"v{v}", width=SIZE, height=SIZE, alpha=True)
    o_px = [0.0] * (SIZE * SIZE * 4)
    variant_packed = []

    for y in range(SIZE):
        for x in range(SIZE):
            idx = (y * SIZE + x) * 4
            aa = a_px[idx + 3]
            ha = h_px[idx + 3]
            if aa > 0.01 and ha > 0.01:
                hv = max(1.0 / 255.0, h_px[idx])
                o_px[idx:idx+4] = [a_px[idx], a_px[idx+1], a_px[idx+2], hv]
                R = int(min(255, a_px[idx] * 255))
                G = int(min(255, a_px[idx+1] * 255))
                B = int(min(255, a_px[idx+2] * 255))
                H = int(min(255, max(1, hv * 255)))
                variant_packed.append(R | (G << 8) | (B << 16) | (H << 24))
            else:
                variant_packed.append(0)

    out.pixels[:] = o_px
    final_path = os.path.join(RAW_DIR, f"conifer_v{v}_final.png")
    out.filepath_raw = final_path
    out.file_format = 'PNG'
    out.save()
    atlas_data.extend(variant_packed)

    bpy.data.images.remove(a_img)
    bpy.data.images.remove(h_img)
    bpy.data.images.remove(out)
    print(f"  Variant {v} done")

# Write atlas binary
atlas_path = os.path.join(OUTPUT_DIR, "conifer_atlas_256.bin")
with open(atlas_path, "wb") as f:
    for val in atlas_data:
        f.write(struct.pack("<I", val))

print(f"\nAtlas written: {os.path.getsize(atlas_path)} bytes")
print(f"Format: {NUM_VARIANTS} variants x {SIZE}x{SIZE} x u32")
print("Done!")

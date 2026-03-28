//! Sprite atlases — Blender-rendered trees and berry bushes.
//! All packed into a single buffer to stay within storage buffer limits.
//!
//! Layout: [tree_variants (24 × 256²)] [bush_variants (16 × 64²)]
//! Tree: sprites[variant * 256² + y * 256 + x]
//! Bush: sprites[BUSH_OFFSET + variant * 64² + y * 64 + x]

pub const SPRITE_SIZE: u32 = 256;
pub const SPRITE_VARIANTS: u32 = 24; // 8 conifer + 16 oak

pub const BUSH_SPRITE_SIZE: u32 = 64;
pub const BUSH_SPRITE_VARIANTS: u32 = 16;
/// Offset in u32 elements where bush sprites start in the combined buffer
pub const BUSH_OFFSET: u32 = SPRITE_VARIANTS * SPRITE_SIZE * SPRITE_SIZE;

static CONIFER_ATLAS: &[u8] = include_bytes!("../assets/sprites/conifer_atlas_256.bin");
static OAK_ATLAS: &[u8] = include_bytes!("../assets/sprites/oak_atlas_16.bin");
static BERRY_ATLAS: &[u8] = include_bytes!("../assets/sprites/berry_atlas_64.bin");

/// Generate combined sprite buffer: trees + bushes in one contiguous array.
pub fn generate_tree_sprites() -> Vec<u32> {
    let tree_ppv = (SPRITE_SIZE * SPRITE_SIZE) as usize;
    let tree_total = tree_ppv * SPRITE_VARIANTS as usize;
    let bush_ppv = (BUSH_SPRITE_SIZE * BUSH_SPRITE_SIZE) as usize;
    let bush_total = bush_ppv * BUSH_SPRITE_VARIANTS as usize;

    assert_eq!(
        CONIFER_ATLAS.len(),
        tree_ppv * 8 * 4,
        "Conifer atlas size mismatch"
    );
    assert_eq!(
        OAK_ATLAS.len(),
        tree_ppv * 16 * 4,
        "Oak atlas size mismatch"
    );
    assert_eq!(
        BERRY_ATLAS.len(),
        bush_total * 4,
        "Berry atlas size mismatch"
    );

    let mut data = vec![0u32; tree_total + bush_total];

    // Trees: conifer (0-7) + oak (8-23)
    for (i, chunk) in CONIFER_ATLAS.chunks_exact(4).enumerate() {
        data[i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    let oak_off = tree_ppv * 8;
    for (i, chunk) in OAK_ATLAS.chunks_exact(4).enumerate() {
        data[oak_off + i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    // Bushes: appended after trees
    for (i, chunk) in BERRY_ATLAS.chunks_exact(4).enumerate() {
        data[tree_total + i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    data
}

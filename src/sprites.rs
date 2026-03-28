//! Sprite atlases — Blender-rendered trees, berry bushes, and rocks.
//! All packed into a single GPU buffer to stay within storage buffer limits.
//!
//! Layout: [trees (24 × 256²)] [bushes (16 × 64²)] [rocks (32 × 64²)]

pub const SPRITE_SIZE: u32 = 256;
pub const SPRITE_VARIANTS: u32 = 24; // 8 conifer + 16 oak

pub const BUSH_SPRITE_SIZE: u32 = 64;
pub const BUSH_SPRITE_VARIANTS: u32 = 16;
pub const BUSH_OFFSET: u32 = SPRITE_VARIANTS * SPRITE_SIZE * SPRITE_SIZE;

pub const ROCK_SPRITE_SIZE: u32 = 64;
pub const ROCK_SPRITE_VARIANTS: u32 = 32;
pub const ROCK_OFFSET: u32 =
    BUSH_OFFSET + BUSH_SPRITE_VARIANTS * BUSH_SPRITE_SIZE * BUSH_SPRITE_SIZE;

static CONIFER_ATLAS: &[u8] = include_bytes!("../assets/sprites/conifer_atlas_256.bin");
static OAK_ATLAS: &[u8] = include_bytes!("../assets/sprites/oak_atlas_16.bin");
static BERRY_ATLAS: &[u8] = include_bytes!("../assets/sprites/berry_atlas_64.bin");
static ROCK_ATLAS: &[u8] = include_bytes!("../assets/sprites/rock_atlas_32x64.bin");

/// Generate combined sprite buffer: trees + bushes + rocks.
pub fn generate_tree_sprites() -> Vec<u32> {
    let tree_ppv = (SPRITE_SIZE * SPRITE_SIZE) as usize;
    let tree_total = tree_ppv * SPRITE_VARIANTS as usize;
    let bush_ppv = (BUSH_SPRITE_SIZE * BUSH_SPRITE_SIZE) as usize;
    let bush_total = bush_ppv * BUSH_SPRITE_VARIANTS as usize;
    let rock_ppv = (ROCK_SPRITE_SIZE * ROCK_SPRITE_SIZE) as usize;
    let rock_total = rock_ppv * ROCK_SPRITE_VARIANTS as usize;

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
    assert_eq!(ROCK_ATLAS.len(), rock_total * 4, "Rock atlas size mismatch");

    let total = tree_total + bush_total + rock_total;
    let mut data = vec![0u32; total];

    // Trees: conifer (0-7) + oak (8-23)
    for (i, chunk) in CONIFER_ATLAS.chunks_exact(4).enumerate() {
        data[i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    let oak_off = tree_ppv * 8;
    for (i, chunk) in OAK_ATLAS.chunks_exact(4).enumerate() {
        data[oak_off + i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    // Bushes: after trees
    let bush_off = tree_total;
    for (i, chunk) in BERRY_ATLAS.chunks_exact(4).enumerate() {
        data[bush_off + i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    // Rocks: after bushes
    let rock_off = tree_total + bush_total;
    for (i, chunk) in ROCK_ATLAS.chunks_exact(4).enumerate() {
        data[rock_off + i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    data
}

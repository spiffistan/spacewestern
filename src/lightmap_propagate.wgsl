// Lightmap propagation pass — iterative flood-fill.
// Reads from source lightmap, writes to destination lightmap.
// Each open cell takes the brightest neighbor minus falloff.
// Walls block propagation. Glass attenuates.

struct Camera {
    center_x: f32,
    center_y: f32,
    zoom: f32,
    show_roofs: f32,
    screen_w: f32,
    screen_h: f32,
    grid_w: f32,
    grid_h: f32,
    time: f32,
    glass_light_mul: f32,
    indoor_glow_mul: f32,
    light_bleed_mul: f32,
};

@group(0) @binding(0) var lightmap_in: texture_2d<f32>;
@group(0) @binding(1) var lightmap_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> camera: Camera;
@group(0) @binding(3) var<storage, read> grid: array<u32>;

// --- Block unpacking ---
fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 { return (b >> 8u) & 0xFFu; }
fn is_door(b: u32) -> bool { return ((b >> 16u) & 1u) != 0u; }
fn is_open(b: u32) -> bool { return ((b >> 16u) & 4u) != 0u; }

fn get_block(x: i32, y: i32) -> u32 {
    if x < 0 || y < 0 || x >= i32(camera.grid_w) || y >= i32(camera.grid_h) {
        return 0u;
    }
    return grid[u32(y) * u32(camera.grid_w) + u32(x)];
}

// Is this block a solid wall that blocks light propagation?
fn is_wall(b: u32) -> bool {
    let bh = block_height(b);
    if bh == 0u { return false; }
    let bt = block_type(b);
    // Glass transmits (attenuated)
    if bt == 5u { return false; }
    // Open doors transmit
    if is_door(b) && is_open(b) { return false; }
    // Everything else with height blocks
    return true;
}

// Propagation falloff per cardinal step (1 block)
const PROP_FALLOFF: f32 = 0.06;
// Glass attenuation factor
const GLASS_ATTEN: f32 = 0.4;
// Diagonal falloff = cardinal * sqrt(2)
const DIAG_FALLOFF: f32 = 0.0849; // 0.06 * 1.414

// Try to take light from a neighbor at (nx, ny).
// Returns the attenuated value, or zero if blocked.
fn sample_neighbor(nx: i32, ny: i32, falloff: f32) -> vec4<f32> {
    if nx < 0 || ny < 0 || nx >= i32(camera.grid_w) || ny >= i32(camera.grid_h) {
        return vec4<f32>(0.0);
    }
    let nb = get_block(nx, ny);
    // Can't receive light from a solid wall
    if is_wall(nb) {
        return vec4<f32>(0.0);
    }
    let nval = textureLoad(lightmap_in, vec2<i32>(nx, ny), 0);
    var intensity = nval.w - falloff;
    // Glass attenuates light passing through
    if block_type(nb) == 5u {
        intensity *= GLASS_ATTEN;
    }
    if intensity <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(nval.xyz, intensity);
}

@compute @workgroup_size(8, 8)
fn main_lightmap_propagate(@builtin(global_invocation_id) gid: vec3<u32>) {
    let bx = i32(gid.x);
    let by = i32(gid.y);

    if bx >= i32(camera.grid_w) || by >= i32(camera.grid_h) {
        return;
    }

    let block = get_block(bx, by);
    let bt = block_type(block);

    // Walls stay at zero — light doesn't enter them
    if is_wall(block) {
        textureStore(lightmap_out, vec2<u32>(gid.xy), vec4<f32>(0.0));
        return;
    }

    // Light sources always keep their seed value
    if bt == 6u || bt == 7u {
        let self_val = textureLoad(lightmap_in, vec2<i32>(bx, by), 0);
        textureStore(lightmap_out, vec2<u32>(gid.xy), self_val);
        return;
    }

    // Start with own current value
    var best = textureLoad(lightmap_in, vec2<i32>(bx, by), 0);

    // Cardinal neighbors
    let n0 = sample_neighbor(bx + 1, by, PROP_FALLOFF);
    let n1 = sample_neighbor(bx - 1, by, PROP_FALLOFF);
    let n2 = sample_neighbor(bx, by + 1, PROP_FALLOFF);
    let n3 = sample_neighbor(bx, by - 1, PROP_FALLOFF);

    if n0.w > best.w { best = n0; }
    if n1.w > best.w { best = n1; }
    if n2.w > best.w { best = n2; }
    if n3.w > best.w { best = n3; }

    // Diagonal neighbors (sqrt(2) falloff for circular spread)
    let d0 = sample_neighbor(bx + 1, by + 1, DIAG_FALLOFF);
    let d1 = sample_neighbor(bx - 1, by + 1, DIAG_FALLOFF);
    let d2 = sample_neighbor(bx + 1, by - 1, DIAG_FALLOFF);
    let d3 = sample_neighbor(bx - 1, by - 1, DIAG_FALLOFF);

    if d0.w > best.w { best = d0; }
    if d1.w > best.w { best = d1; }
    if d2.w > best.w { best = d2; }
    if d3.w > best.w { best = d3; }

    // Glass blocks themselves get attenuated
    if bt == 5u {
        best = vec4<f32>(best.xyz, best.w * GLASS_ATTEN);
    }

    textureStore(lightmap_out, vec2<u32>(gid.xy), max(best, vec4<f32>(0.0)));
}

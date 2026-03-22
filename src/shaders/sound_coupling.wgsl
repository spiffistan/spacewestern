// Sound→Gas coupling: the pressure gradient from the sound wave equation
// applies a force to the fluid velocity field. This makes explosion blasts
// push smoke, alarm bells oscillate nearby gas, etc.
//
// Reads: sound pressure texture, fluid velocity texture
// Writes: fluid velocity texture (with sound forces added)

struct Camera {
    center_x: f32, center_y: f32, zoom: f32, show_roofs: f32,
    screen_w: f32, screen_h: f32, grid_w: f32, grid_h: f32,
    time: f32, glass_light_mul: f32, indoor_glow_mul: f32, light_bleed_mul: f32,
    foliage_opacity: f32, foliage_variation: f32, oblique_strength: f32,
    lm_vp_min_x: f32, lm_vp_min_y: f32, lm_vp_max_x: f32, lm_vp_max_y: f32, lm_scale: f32,
    fluid_overlay: f32,
    sun_dir_x: f32, sun_dir_y: f32, sun_elevation: f32,
    sun_intensity: f32, sun_color_r: f32, sun_color_g: f32, sun_color_b: f32,
    ambient_r: f32, ambient_g: f32, ambient_b: f32,
    enable_prox_glow: f32, enable_dir_bleed: f32, force_refresh: f32,
    pleb_x: f32, pleb_y: f32, pleb_angle: f32, pleb_selected: f32,
    pleb_torch: f32, pleb_headlight: f32,
    prev_center_x: f32, prev_center_y: f32, prev_zoom: f32, prev_time: f32,
    rain_intensity: f32, cloud_cover: f32, wind_magnitude: f32, wind_angle: f32,
    use_shadow_map: f32, shadow_map_scale: f32, sound_speed: f32, sound_damping: f32,
    sound_coupling: f32, enable_terrain_detail: f32, terrain_ao_strength: f32, _pad4_c: f32,
};

@group(0) @binding(0) var sound_tex: texture_2d<f32>;          // sound pressure (R=pressure, G=velocity)
@group(0) @binding(1) var vel_in: texture_2d<f32>;             // fluid velocity input (RG)
@group(0) @binding(2) var vel_out: texture_storage_2d<rg32float, write>; // fluid velocity output
@group(0) @binding(3) var<uniform> camera: Camera;

@compute @workgroup_size(8, 8)
fn main_sound_coupling(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x);
    let y = i32(gid.y);
    // Fluid sim may run at different resolution than grid
    let fw = i32(camera.screen_w); // actually we need fluid resolution, not screen
    let fh = i32(camera.screen_h);
    let gw = i32(camera.grid_w);
    let gh = i32(camera.grid_h);

    // This shader runs at grid resolution (256×256) — same as the sound texture.
    if x >= gw || y >= gh { return; }

    // Read current fluid velocity at this grid cell
    let vel = textureLoad(vel_in, vec2(x, y), 0).rg;

    let coupling = camera.sound_coupling;
    if coupling < 0.001 {
        // No coupling — just copy velocity through
        textureStore(vel_out, vec2(gid.xy), vec4(vel, 0.0, 0.0));
        return;
    }

    // Read sound pressure at neighbors to compute gradient
    let p_here = textureLoad(sound_tex, vec2(x, y), 0).r;
    let p_left  = textureLoad(sound_tex, clamp(vec2(x - 1, y), vec2(0), vec2(gw - 1, gh - 1)), 0).r;
    let p_right = textureLoad(sound_tex, clamp(vec2(x + 1, y), vec2(0), vec2(gw - 1, gh - 1)), 0).r;
    let p_up    = textureLoad(sound_tex, clamp(vec2(x, y - 1), vec2(0), vec2(gw - 1, gh - 1)), 0).r;
    let p_down  = textureLoad(sound_tex, clamp(vec2(x, y + 1), vec2(0), vec2(gw - 1, gh - 1)), 0).r;

    // Pressure gradient: gas flows DOWN the gradient (away from high pressure)
    let grad_x = (p_right - p_left) * 0.5;
    let grad_y = (p_down - p_up) * 0.5;

    // Force on gas = -gradient * coupling (negative because flow is from high to low pressure)
    let force = vec2<f32>(-grad_x, -grad_y) * coupling;

    // Add force to velocity
    let new_vel = vel + force;

    textureStore(vel_out, vec2(gid.xy), vec4(new_vel, 0.0, 0.0));
}

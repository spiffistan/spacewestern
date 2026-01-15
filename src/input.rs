use std::collections::HashSet;
use winit::keyboard::KeyCode;

/// Rimworld-style discrete zoom levels (close to far)
const ZOOM_LEVELS: [f32; 11] = [
    0.8,   // Very close - detailed view
    1.2,   // Close
    1.8,   // Medium-close
    2.5,   // Medium
    3.5,   // Medium-far
    5.0,   // Default view
    7.0,   // Far
    10.0,  // Very far
    14.0,  // Overview
    20.0,  // Map view
    30.0,  // Full map
];

/// Voxel being dragged (world grid coordinates)
#[derive(Clone, Copy, Debug, Default)]
pub struct DraggedVoxel {
    pub active: bool,
    pub source_x: i32,
    pub source_y: i32,
    pub source_z: i32,
}

/// Camera and input state
pub struct InputState {
    pub camera_x: f32,
    pub camera_z: f32,
    pub zoom: f32,
    pub target_zoom: f32,
    pub visible_layer: i32,
    zoom_level_index: usize,
    zoom_key_released: bool, // Prevents repeated zoom on key hold
    keys_pressed: HashSet<KeyCode>,
    // Mouse camera drag state (right mouse button)
    camera_dragging: bool,
    last_mouse_x: f32,
    last_mouse_y: f32,
    // Current mouse position (for voxel picking)
    pub mouse_x: f32,
    pub mouse_y: f32,
    // Voxel drag state (left mouse button)
    pub voxel_dragging: bool,
    pub dragged_voxel: DraggedVoxel,
    // Placed voxels (moved from original position)
    // Key: (x, y, z) destination, Value: voxel type
    pub placed_voxels: std::collections::HashMap<(i32, i32, i32), u8>,
    // Removed voxel positions (source positions of dragged voxels)
    pub removed_voxels: std::collections::HashSet<(i32, i32, i32)>,
}

impl InputState {
    pub fn new() -> Self {
        let default_zoom_index = 5; // Start at zoom level 5.0
        Self {
            camera_x: 0.0,
            camera_z: 0.0,
            zoom: ZOOM_LEVELS[default_zoom_index],
            target_zoom: ZOOM_LEVELS[default_zoom_index],
            visible_layer: -1, // Show all layers by default
            zoom_level_index: default_zoom_index,
            zoom_key_released: true,
            keys_pressed: HashSet::new(),
            camera_dragging: false,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            voxel_dragging: false,
            dragged_voxel: DraggedVoxel::default(),
            placed_voxels: std::collections::HashMap::new(),
            removed_voxels: std::collections::HashSet::new(),
        }
    }

    /// Handle key press
    pub fn key_pressed(&mut self, key: KeyCode) {
        self.keys_pressed.insert(key);
    }

    /// Handle key release
    pub fn key_released(&mut self, key: KeyCode) {
        self.keys_pressed.remove(&key);

        // Re-enable zoom stepping when zoom keys are released
        if matches!(key, KeyCode::KeyQ | KeyCode::KeyE | KeyCode::Minus | KeyCode::Equal) {
            self.zoom_key_released = true;
        }
    }

    /// Handle mouse scroll wheel - Rimworld style discrete zoom
    pub fn scroll(&mut self, delta: f32) {
        if delta > 0.0 && self.zoom_level_index > 0 {
            // Scroll up = zoom in
            self.zoom_level_index -= 1;
            self.target_zoom = ZOOM_LEVELS[self.zoom_level_index];
        } else if delta < 0.0 && self.zoom_level_index < ZOOM_LEVELS.len() - 1 {
            // Scroll down = zoom out
            self.zoom_level_index += 1;
            self.target_zoom = ZOOM_LEVELS[self.zoom_level_index];
        }
    }

    /// Handle mouse button press
    pub fn mouse_pressed(&mut self, button: u8) {
        match button {
            0 => {
                // Left mouse button - start voxel drag
                self.voxel_dragging = true;
                // The actual voxel picking will be done by the shader/CPU raycast
            }
            1 | 2 => {
                // Right or middle mouse button - camera panning
                self.camera_dragging = true;
            }
            _ => {}
        }
    }

    /// Handle mouse button release
    pub fn mouse_released(&mut self, button: u8) {
        match button {
            0 => {
                // Left mouse button released - complete voxel drag
                if self.voxel_dragging && self.dragged_voxel.active {
                    // The drop will be handled externally when we know the target position
                }
                self.voxel_dragging = false;
            }
            1 | 2 => {
                self.camera_dragging = false;
            }
            _ => {}
        }
    }

    /// Start dragging a voxel from the given world position
    pub fn start_voxel_drag(&mut self, x: i32, y: i32, z: i32) {
        self.dragged_voxel = DraggedVoxel {
            active: true,
            source_x: x,
            source_y: y,
            source_z: z,
        };
    }

    /// Complete the voxel drag - move voxel from source to destination
    pub fn complete_voxel_drag(&mut self, dest_x: i32, dest_y: i32, dest_z: i32, voxel_type: u8) {
        if self.dragged_voxel.active {
            let source = (
                self.dragged_voxel.source_x,
                self.dragged_voxel.source_y,
                self.dragged_voxel.source_z,
            );
            let dest = (dest_x, dest_y, dest_z);

            // Don't do anything if dropping on same position
            if source != dest {
                // Mark source as removed
                self.removed_voxels.insert(source);
                // Remove from placed if it was there
                self.placed_voxels.remove(&source);

                // Place at destination
                self.placed_voxels.insert(dest, voxel_type);
                // Remove from removed if it was there
                self.removed_voxels.remove(&dest);
            }
        }
        self.dragged_voxel = DraggedVoxel::default();
    }

    /// Cancel the current voxel drag
    pub fn cancel_voxel_drag(&mut self) {
        self.dragged_voxel = DraggedVoxel::default();
    }

    /// Handle mouse movement (for drag panning and voxel picking)
    pub fn mouse_moved(&mut self, x: f32, y: f32) {
        // Always track current mouse position for voxel picking
        self.mouse_x = x;
        self.mouse_y = y;

        if self.camera_dragging {
            let dx = x - self.last_mouse_x;
            let dy = y - self.last_mouse_y;

            // Pan camera - scale by zoom level
            let pan_speed = self.zoom * 0.5;
            self.camera_x -= dx * pan_speed;
            self.camera_z += dy * pan_speed;  // Y mouse = Z world (forward/back)
        }

        // Always update last position for smooth camera drag start
        self.last_mouse_x = x;
        self.last_mouse_y = y;
    }
    /// Update state based on held keys (call once per frame)
    pub fn update(&mut self, delta_time: f32) {
        let move_speed = 30.0 * self.zoom * delta_time;
        let zoom_lerp_speed = 10.0 * delta_time; // Snappy zoom transitions

        // Camera movement (WASD)
        if self.keys_pressed.contains(&KeyCode::KeyW) {
            self.camera_z += move_speed;
        }
        if self.keys_pressed.contains(&KeyCode::KeyS) {
            self.camera_z -= move_speed;
        }
        if self.keys_pressed.contains(&KeyCode::KeyA) {
            self.camera_x -= move_speed;
        }
        if self.keys_pressed.contains(&KeyCode::KeyD) {
            self.camera_x += move_speed;
        }

        // Discrete zoom controls (Q/E or -/+) - Rimworld style: step to next level on press
        let zoom_out = self.keys_pressed.contains(&KeyCode::KeyQ) || self.keys_pressed.contains(&KeyCode::Minus);
        let zoom_in = self.keys_pressed.contains(&KeyCode::KeyE) || self.keys_pressed.contains(&KeyCode::Equal);

        if self.zoom_key_released {
            if zoom_out && self.zoom_level_index < ZOOM_LEVELS.len() - 1 {
                self.zoom_level_index += 1;
                self.target_zoom = ZOOM_LEVELS[self.zoom_level_index];
                self.zoom_key_released = false;
            }
            if zoom_in && self.zoom_level_index > 0 {
                self.zoom_level_index -= 1;
                self.target_zoom = ZOOM_LEVELS[self.zoom_level_index];
                self.zoom_key_released = false;
            }
        }

        // Layer visibility controls (1-9 show that layer, 0 shows all)
        if self.keys_pressed.contains(&KeyCode::Digit1) {
            self.visible_layer = 1;
        }
        if self.keys_pressed.contains(&KeyCode::Digit2) {
            self.visible_layer = 2;
        }
        if self.keys_pressed.contains(&KeyCode::Digit3) {
            self.visible_layer = 3;
        }
        if self.keys_pressed.contains(&KeyCode::Digit4) {
            self.visible_layer = 4;
        }
        if self.keys_pressed.contains(&KeyCode::Digit5) {
            self.visible_layer = 5;
        }
        if self.keys_pressed.contains(&KeyCode::Digit6) {
            self.visible_layer = 6;
        }
        if self.keys_pressed.contains(&KeyCode::Digit7) {
            self.visible_layer = 7;
        }
        if self.keys_pressed.contains(&KeyCode::Digit8) {
            self.visible_layer = 8;
        }
        if self.keys_pressed.contains(&KeyCode::Digit9) {
            self.visible_layer = 9;
        }
        if self.keys_pressed.contains(&KeyCode::Digit0) {
            self.visible_layer = -1; // Show all
        }

        // Smooth zoom interpolation to target level
        self.zoom = self.zoom + (self.target_zoom - self.zoom) * zoom_lerp_speed.min(1.0);
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

//! Weather system — global state machine with rain, wetness, and environmental effects.

use crate::grid::*;

/// Global weather state.
#[derive(Clone, Debug, PartialEq)]
pub enum WeatherState {
    Clear,
    Cloudy,
    LightRain,
    HeavyRain,
}

impl WeatherState {
    pub fn rain_intensity(&self) -> f32 {
        match self {
            WeatherState::Clear => 0.0,
            WeatherState::Cloudy => 0.0,
            WeatherState::LightRain => 0.4,
            WeatherState::HeavyRain => 1.0,
        }
    }

    pub fn cloud_cover(&self) -> f32 {
        match self {
            WeatherState::Clear => 0.0,
            WeatherState::Cloudy => 0.5,
            WeatherState::LightRain => 0.7,
            WeatherState::HeavyRain => 0.95,
        }
    }

    pub fn sun_dimming(&self) -> f32 {
        1.0 - self.cloud_cover() * 0.6
    }
}

/// Transition weather based on time elapsed. Returns new state.
/// Weather changes roughly every 30-90 game seconds.
pub fn tick_weather(
    current: &WeatherState,
    timer: &mut f32,
    dt: f32,
    time_speed: f32,
) -> Option<WeatherState> {
    *timer -= dt * time_speed;
    if *timer > 0.0 {
        return None;
    }

    // Random-ish transition using timer underflow as seed
    let seed = ((*timer * 10000.0).abs() as u32).wrapping_mul(2654435761);
    let r = (seed & 0xFFFF) as f32 / 65535.0;

    let next = match current {
        WeatherState::Clear => {
            if r < 0.6 {
                WeatherState::Clear
            } else if r < 0.85 {
                WeatherState::Cloudy
            } else {
                WeatherState::LightRain
            }
        }
        WeatherState::Cloudy => {
            if r < 0.3 {
                WeatherState::Clear
            } else if r < 0.6 {
                WeatherState::Cloudy
            } else if r < 0.85 {
                WeatherState::LightRain
            } else {
                WeatherState::HeavyRain
            }
        }
        WeatherState::LightRain => {
            if r < 0.2 {
                WeatherState::Cloudy
            } else if r < 0.5 {
                WeatherState::LightRain
            } else if r < 0.8 {
                WeatherState::HeavyRain
            } else {
                WeatherState::Clear
            }
        }
        WeatherState::HeavyRain => {
            if r < 0.4 {
                WeatherState::HeavyRain
            } else if r < 0.7 {
                WeatherState::LightRain
            } else {
                WeatherState::Cloudy
            }
        }
    };

    // Next transition in 30-90 game seconds
    *timer = 30.0 + r * 60.0;
    Some(next)
}

/// Update per-tile wetness based on weather and sun.
/// `wetness` is 256x256 matching the grid.
pub fn tick_wetness(
    wetness: &mut [f32],
    grid: &[u32],
    rain_intensity: f32,
    sun_intensity: f32,
    dt: f32,
    time_speed: f32,
    _grid_w: u32,
) {
    let t = dt * time_speed;
    for (i, w) in wetness.iter_mut().enumerate() {
        let b = grid[i];
        let roof_h = (b >> 24) & 0xFF;
        let bt = b & 0xFF;
        let is_outdoor = roof_h == 0;
        let is_ground = bt_is!(
            bt,
            BT_GROUND,
            BT_WOOD_FLOOR,
            BT_STONE_FLOOR,
            BT_CONCRETE_FLOOR,
            BT_ROUGH_FLOOR,
            BT_DUG_GROUND
        );

        if is_ground {
            if is_outdoor && rain_intensity > 0.0 {
                // Rain wets outdoor ground
                *w = (*w + rain_intensity * 0.08 * t).min(1.0);
            }
            // Evaporation: faster in sun, slower indoors
            let evap_rate = if is_outdoor {
                0.01 + sun_intensity * 0.03
            } else {
                0.005
            };
            *w = (*w - evap_rate * t).max(0.0);
        } else {
            // Non-ground tiles don't hold wetness
            *w = (*w - 0.1 * t).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rain_intensity() {
        assert_eq!(WeatherState::Clear.rain_intensity(), 0.0);
        assert!(WeatherState::LightRain.rain_intensity() > 0.0);
        assert!(
            WeatherState::HeavyRain.rain_intensity() > WeatherState::LightRain.rain_intensity()
        );
    }

    #[test]
    fn test_weather_transition() {
        let mut timer = 0.0;
        let result = tick_weather(&WeatherState::Clear, &mut timer, 1.0, 1.0);
        assert!(result.is_some());
        assert!(timer > 0.0, "timer should be reset");
    }

    #[test]
    fn test_wetness_increases_in_rain() {
        use crate::grid::make_block;
        let grid = vec![make_block(2, 0, 0)]; // outdoor dirt
        let mut wetness = vec![0.0];
        tick_wetness(&mut wetness, &grid, 1.0, 0.0, 1.0, 1.0, 1);
        assert!(wetness[0] > 0.0, "wetness should increase in rain");
    }

    #[test]
    fn test_wetness_evaporates_in_sun() {
        use crate::grid::make_block;
        let grid = vec![make_block(2, 0, 0)]; // outdoor dirt
        let mut wetness = vec![0.8];
        tick_wetness(&mut wetness, &grid, 0.0, 1.0, 1.0, 1.0, 1);
        assert!(wetness[0] < 0.8, "wetness should evaporate in sun");
    }
}

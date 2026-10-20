use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScreentoneConfig {
    pub frequency: f32, // Lines per inch / frequency (e.g. 40.0, 50.0, 60.0 LPI)
    pub density: f32,   // 0.0 to 1.0 (shade intensity)
    pub angle_deg: f32, // Standard manga angle is 45.0 degrees
}

impl Default for ScreentoneConfig {
    fn default() -> Self {
        Self {
            frequency: 50.0,
            density: 0.25,
            angle_deg: 45.0,
        }
    }
}

/// Generates a standard Clip Studio Paint circular dot manga halftone screentone pattern
pub fn generate_screentone_tile(
    width: u32,
    height: u32,
    config: &ScreentoneConfig,
) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let angle_rad = config.angle_deg.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();

    // Cell spacing in pixels
    let cell_size = (300.0 / config.frequency.max(10.0)).max(3.0);
    let dot_radius_ratio = config.density.sqrt().clamp(0.0, 0.95);
    let max_radius = cell_size * 0.5 * dot_radius_ratio;

    for y in 0..height {
        for x in 0..width {
            let fx = x as f32;
            let fy = y as f32;

            // Rotate coordinates to halftone angle (e.g. 45 degrees)
            let rot_x = fx * cos_a - fy * sin_a;
            let rot_y = fx * sin_a + fy * cos_a;

            // Grid coordinate inside halftone cell (-0.5 .. 0.5)
            let u = (rot_x / cell_size).fract() - 0.5;
            let v = (rot_y / cell_size).fract() - 0.5;

            let dist = (u * u + v * v).sqrt() * cell_size;

            let is_dot = dist <= max_radius;
            let idx = ((y * width + x) * 4) as usize;

            if is_dot {
                // Black screentone dot with full opacity
                pixels[idx] = 15;
                pixels[idx + 1] = 15;
                pixels[idx + 2] = 20;
                pixels[idx + 3] = 255;
            } else {
                pixels[idx] = 0;
                pixels[idx + 1] = 0;
                pixels[idx + 2] = 0;
                pixels[idx + 3] = 0; // Transparent space between dots
            }
        }
    }

    pixels
}

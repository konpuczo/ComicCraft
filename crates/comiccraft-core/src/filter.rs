//! Clip Studio Paint-style Tonal Correction & Image Filters.
//!
//! Provides Brightness/Contrast, Hue/Saturation/Luminosity, Gaussian Blur,
//! and Color Inversion, constrained to the active layer and selection mask.

use crate::layer::Layer;
use crate::selection::Selection;

/// Applies Brightness (-100..=100) and Contrast (-100..=100) correction.
pub fn apply_brightness_contrast(
    layer: &mut Layer,
    sel: &Selection,
    brightness: i32, // -100 to 100
    contrast: i32,   // -100 to 100
) {
    let b = brightness.clamp(-100, 100) as f32 * 2.55;
    let c = contrast.clamp(-100, 100) as f32;
    // Standard photo contrast formula: factor = (259 * (C + 255)) / (255 * (259 - C))
    let factor = (259.0 * (c + 255.0)) / (255.0 * (259.0 - c).max(1.0));

    let w = layer.width;
    let h = layer.height;

    for y in 0..h {
        let row_start = (y * w) as usize * 4;
        for x in 0..w {
            if !sel.contains(x, y) {
                continue;
            }
            let idx = row_start + (x as usize) * 4;
            if layer.pixels[idx + 3] == 0 {
                continue;
            }
            for ch in 0..3 {
                let orig = layer.pixels[idx + ch] as f32;
                // Apply brightness shift then contrast scaling around midtone 128
                let val = factor * (orig + b - 128.0) + 128.0;
                layer.pixels[idx + ch] = val.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// Applies Hue shift (-180..=180 deg), Saturation multiplier (-100..=100%), and Luminosity (-100..=100%).
pub fn apply_hue_saturation_luminosity(
    layer: &mut Layer,
    sel: &Selection,
    hue_deg: f32, // -180.0 to 180.0
    sat_pct: f32, // -100.0 to 100.0
    lum_pct: f32, // -100.0 to 100.0
) {
    let w = layer.width;
    let h = layer.height;
    let sat_factor = (1.0 + (sat_pct / 100.0)).clamp(0.0, 3.0);
    let lum_factor = lum_pct / 100.0;

    for y in 0..h {
        let row_start = (y * w) as usize * 4;
        for x in 0..w {
            if !sel.contains(x, y) {
                continue;
            }
            let idx = row_start + (x as usize) * 4;
            if layer.pixels[idx + 3] == 0 {
                continue;
            }
            let r = layer.pixels[idx] as f32 / 255.0;
            let g = layer.pixels[idx + 1] as f32 / 255.0;
            let b = layer.pixels[idx + 2] as f32 / 255.0;

            let (mut h, mut s, mut l) = rgb_to_hsl(r, g, b);
            h = (h + hue_deg).rem_euclid(360.0);
            s = (s * sat_factor).clamp(0.0, 1.0);
            if lum_factor > 0.0 {
                l += (1.0 - l) * lum_factor;
            } else if lum_factor < 0.0 {
                l += l * lum_factor;
            }
            l = l.clamp(0.0, 1.0);

            let (nr, ng, nb) = hsl_to_rgb(h, s, l);
            layer.pixels[idx] = (nr * 255.0).round().clamp(0.0, 255.0) as u8;
            layer.pixels[idx + 1] = (ng * 255.0).round().clamp(0.0, 255.0) as u8;
            layer.pixels[idx + 2] = (nb * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// Applies high-performance separable Gaussian / box blur with radius `radius_px`.
pub fn apply_gaussian_blur(layer: &mut Layer, sel: &Selection, radius_px: f32) {
    let r = radius_px.round().clamp(1.0, 100.0) as i32;
    let w = layer.width as usize;
    let h = layer.height as usize;
    if w == 0 || h == 0 || r == 0 {
        return;
    }

    // Two-pass horizontal and vertical box filtering (3 iterations approximates Gaussian closely)
    let passes = 3;
    let mut current = layer.pixels.clone();
    let mut temp = vec![0u8; w * h * 4];

    for _ in 0..passes {
        // Horizontal pass
        for y in 0..h {
            let row_off = y * w * 4;
            for x in 0..w {
                let mut acc = [0u32; 4];
                let mut count = 0u32;
                let x_start = (x as i32 - r).max(0) as usize;
                let x_end = (x as i32 + r).min(w as i32 - 1) as usize;
                for kx in x_start..=x_end {
                    let idx = row_off + kx * 4;
                    for c in 0..4 {
                        acc[c] += current[idx + c] as u32;
                    }
                    count += 1;
                }
                let out_idx = row_off + x * 4;
                for c in 0..4 {
                    temp[out_idx + c] = (acc[c] / count) as u8;
                }
            }
        }

        // Vertical pass
        for y in 0..h {
            let y_start = (y as i32 - r).max(0) as usize;
            let y_end = (y as i32 + r).min(h as i32 - 1) as usize;
            let out_row_off = y * w * 4;
            for x in 0..w {
                let mut acc = [0u32; 4];
                let mut count = 0u32;
                for ky in y_start..=y_end {
                    let idx = ky * w * 4 + x * 4;
                    for c in 0..4 {
                        acc[c] += temp[idx + c] as u32;
                    }
                    count += 1;
                }
                let out_idx = out_row_off + x * 4;
                for c in 0..4 {
                    current[out_idx + c] = (acc[c] / count) as u8;
                }
            }
        }
    }

    // Blend blurred pixels back into layer respecting selection and alpha lock
    for y in 0..h {
        let row_off = y * w * 4;
        for x in 0..w {
            if !sel.contains(x as u32, y as u32) {
                continue;
            }
            let idx = row_off + x * 4;
            if layer.alpha_locked {
                if layer.pixels[idx + 3] > 0 {
                    layer.pixels[idx..idx + 3].copy_from_slice(&current[idx..idx + 3]);
                }
            } else {
                layer.pixels[idx..idx + 4].copy_from_slice(&current[idx..idx + 4]);
            }
        }
    }
}

/// Inverts RGB colors of the layer (optionally preserving alpha).
pub fn apply_invert(layer: &mut Layer, sel: &Selection) {
    let w = layer.width;
    let h = layer.height;
    for y in 0..h {
        let row = (y * w) as usize * 4;
        for x in 0..w {
            if !sel.contains(x, y) {
                continue;
            }
            let idx = row + (x as usize) * 4;
            if layer.pixels[idx + 3] > 0 {
                layer.pixels[idx] = 255 - layer.pixels[idx];
                layer.pixels[idx + 1] = 255 - layer.pixels[idx + 1];
                layer.pixels[idx + 2] = 255 - layer.pixels[idx + 2];
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Color conversion helpers (RGB <-> HSL)
// -----------------------------------------------------------------------------

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let l = (max + min) * 0.5;

    if delta <= 1e-5 {
        return (0.0, 0.0, l);
    }

    let s = if l < 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    let mut h = if (max - r).abs() <= 1e-5 {
        ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() <= 1e-5 {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    h *= 60.0;
    if h < 0.0 {
        h += 360.0;
    }

    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s <= 1e-5 {
        return (l, l, l);
    }
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0).rem_euclid(2.0)) - 1.0).abs());
    let m = l - c * 0.5;

    let (r1, g1, b1) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (r1 + m, g1 + m, b1 + m)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GradientStop {
    pub position: f32, // 0.0 to 1.0
    pub color: [u8; 4],
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GradientMap {
    pub stops: Vec<GradientStop>,
}

impl GradientMap {
    pub fn new(mut stops: Vec<GradientStop>) -> Self {
        stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap_or(std::cmp::Ordering::Equal));
        Self { stops }
    }

    pub fn sample(&self, t: f32) -> [u8; 4] {
        if self.stops.is_empty() {
            return [0, 0, 0, 255];
        }
        let t = t.clamp(0.0, 1.0);
        if t <= self.stops[0].position {
            return self.stops[0].color;
        }
        if t >= self.stops.last().unwrap().position {
            return self.stops.last().unwrap().color;
        }
        for w in self.stops.windows(2) {
            let p0 = w[0].position;
            let p1 = w[1].position;
            if t >= p0 && t <= p1 {
                let range = p1 - p0;
                let factor = if range > 1e-5 { (t - p0) / range } else { 0.0 };
                let c0 = w[0].color;
                let c1 = w[1].color;
                return [
                    (c0[0] as f32 + (c1[0] as f32 - c0[0] as f32) * factor).round() as u8,
                    (c0[1] as f32 + (c1[1] as f32 - c0[1] as f32) * factor).round() as u8,
                    (c0[2] as f32 + (c1[2] as f32 - c0[2] as f32) * factor).round() as u8,
                    (c0[3] as f32 + (c1[3] as f32 - c0[3] as f32) * factor).round() as u8,
                ];
            }
        }
        self.stops[0].color
    }

    pub fn preset_sunset() -> Self {
        Self::new(vec![
            GradientStop { position: 0.0, color: [20, 10, 45, 255] },
            GradientStop { position: 0.35, color: [160, 40, 80, 255] },
            GradientStop { position: 0.70, color: [240, 140, 40, 255] },
            GradientStop { position: 1.0, color: [255, 245, 180, 255] },
        ])
    }

    pub fn preset_sepia() -> Self {
        Self::new(vec![
            GradientStop { position: 0.0, color: [30, 20, 15, 255] },
            GradientStop { position: 0.5, color: [140, 110, 80, 255] },
            GradientStop { position: 1.0, color: [245, 230, 205, 255] },
        ])
    }

    pub fn preset_anime_sky() -> Self {
        Self::new(vec![
            GradientStop { position: 0.0, color: [25, 45, 95, 255] },
            GradientStop { position: 0.45, color: [75, 140, 220, 255] },
            GradientStop { position: 0.85, color: [180, 220, 250, 255] },
            GradientStop { position: 1.0, color: [255, 255, 255, 255] },
        ])
    }

    pub fn preset_cyberpunk() -> Self {
        Self::new(vec![
            GradientStop { position: 0.0, color: [10, 5, 30, 255] },
            GradientStop { position: 0.4, color: [140, 15, 130, 255] },
            GradientStop { position: 0.75, color: [0, 220, 255, 255] },
            GradientStop { position: 1.0, color: [240, 255, 180, 255] },
        ])
    }
}

pub fn apply_gradient_map(layer: &mut Layer, sel: &Selection, map: &GradientMap) {
    let w = layer.width;
    let h = layer.height;

    for y in 0..h {
        let row_start = (y * w) as usize * 4;
        for x in 0..w {
            if !sel.contains(x, y) {
                continue;
            }
            let idx = row_start + (x as usize) * 4;
            let a = layer.pixels[idx + 3];
            if a == 0 {
                continue;
            }
            let r = layer.pixels[idx] as f32;
            let g = layer.pixels[idx + 1] as f32;
            let b = layer.pixels[idx + 2] as f32;
            let luma = ((0.299 * r + 0.587 * g + 0.114 * b) / 255.0).clamp(0.0, 1.0);
            let mapped = map.sample(luma);
            layer.pixels[idx] = mapped[0];
            layer.pixels[idx + 1] = mapped[1];
            layer.pixels[idx + 2] = mapped[2];
            layer.pixels[idx + 3] = ((a as u32 * mapped[3] as u32) / 255) as u8;
        }
    }
}

pub fn apply_binarization(layer: &mut Layer, sel: &Selection, threshold: u8) {
    let w = layer.width;
    let h = layer.height;
    let th = threshold as f32;

    for y in 0..h {
        let row_start = (y * w) as usize * 4;
        for x in 0..w {
            if !sel.contains(x, y) {
                continue;
            }
            let idx = row_start + (x as usize) * 4;
            if layer.pixels[idx + 3] == 0 {
                continue;
            }
            let r = layer.pixels[idx] as f32;
            let g = layer.pixels[idx + 1] as f32;
            let b = layer.pixels[idx + 2] as f32;
            let luma = 0.299 * r + 0.587 * g + 0.114 * b;
            let val = if luma >= th { 255 } else { 0 };
            layer.pixels[idx] = val;
            layer.pixels[idx + 1] = val;
            layer.pixels[idx + 2] = val;
        }
    }
}

pub fn apply_levels(
    layer: &mut Layer,
    sel: &Selection,
    in_black: u8,
    in_white: u8,
    gamma: f32,
    out_black: u8,
    out_white: u8,
) {
    let in_b = in_black as f32;
    let in_w = (in_white as f32).max(in_b + 1.0);
    let out_b = out_black as f32;
    let out_w = out_white as f32;
    let inv_gamma = 1.0 / gamma.max(0.01);

    let mut lut = [0u8; 256];
    for i in 0..256 {
        let v = (i as f32 - in_b) / (in_w - in_b);
        let clamped = v.clamp(0.0, 1.0);
        let g = clamped.powf(inv_gamma);
        let out = out_b + g * (out_w - out_b);
        lut[i] = out.round().clamp(0.0, 255.0) as u8;
    }

    let w = layer.width;
    let h = layer.height;
    for y in 0..h {
        let row_start = (y * w) as usize * 4;
        for x in 0..w {
            if !sel.contains(x, y) {
                continue;
            }
            let idx = row_start + (x as usize) * 4;
            if layer.pixels[idx + 3] == 0 {
                continue;
            }
            layer.pixels[idx] = lut[layer.pixels[idx] as usize];
            layer.pixels[idx + 1] = lut[layer.pixels[idx + 1] as usize];
            layer.pixels[idx + 2] = lut[layer.pixels[idx + 2] as usize];
        }
    }
}

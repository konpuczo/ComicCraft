//! Manga Action Lines & Focus Lines (Clip Studio Paint Figure / Speed Lines).
//!
//! Generates dramatic Radiation Lines (Sunburst / Focus Lines) converging on a focal point,
//! and Directional Stream Lines (Parallel Action Lines).

use crate::layer::Layer;
use crate::selection::Selection;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FocusLinesConfig {
    pub center_x: f32,
    pub center_y: f32,
    /// Distance from center where speed lines stop (leaving the subject's face clear).
    pub inner_radius: f32,
    /// Outer boundary limit (defaults to outer canvas corner).
    pub outer_radius: f32,
    pub count: u32,
    pub thickness: f32,
    pub jitter: f32, // 0.0 to 1.0 (randomization of inner spike start positions)
    pub color: [u8; 4],
}

impl Default for FocusLinesConfig {
    fn default() -> Self {
        Self {
            center_x: 800.0,
            center_y: 1200.0,
            inner_radius: 220.0,
            outer_radius: 1800.0,
            count: 120,
            thickness: 3.5,
            jitter: 0.45,
            color: [20, 20, 25, 255],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StreamLinesConfig {
    pub angle_deg: f32,
    pub count: u32,
    pub length: f32,
    pub thickness: f32,
    pub jitter: f32,
    pub color: [u8; 4],
}

impl Default for StreamLinesConfig {
    fn default() -> Self {
        Self {
            angle_deg: 0.0,
            count: 90,
            length: 450.0,
            thickness: 2.5,
            jitter: 0.4,
            color: [20, 20, 25, 255],
        }
    }
}

/// Renders manga radiating focus lines (sunburst) converging towards `(center_x, center_y)`.
pub fn render_focus_lines(layer: &mut Layer, sel: &Selection, cfg: &FocusLinesConfig) {
    let count = cfg.count.clamp(12, 600) as usize;
    let cx = cfg.center_x;
    let cy = cfg.center_y;
    let r_in_base = cfg.inner_radius.max(10.0);
    let r_out = cfg.outer_radius.max(r_in_base + 50.0);
    let col = cfg.color;

    let mut rng = 0x517c_c1b7 ^ (cfg.count * 31);
    let pseudo_rand = |state: &mut u32| -> f32 {
        *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        (*state >> 8) as f32 / 16777215.0
    };

    let step = std::f32::consts::TAU / count as f32;

    for i in 0..count {
        let base_angle = i as f32 * step;
        let angle_jitter = (pseudo_rand(&mut rng) - 0.5) * step * 0.75;
        let theta = base_angle + angle_jitter;
        let (sin_t, cos_t) = theta.sin_cos();

        let rad_jitter = (pseudo_rand(&mut rng) - 0.5) * r_in_base * cfg.jitter.clamp(0.0, 1.0);
        let r_start = (r_in_base + rad_jitter).max(10.0);

        let thick = cfg.thickness * (0.6 + pseudo_rand(&mut rng) * 0.8);

        // Draw wedge line from r_start to r_out
        draw_tapered_line(
            layer,
            sel,
            cx + cos_t * r_start,
            cy + sin_t * r_start,
            cx + cos_t * r_out,
            cy + sin_t * r_out,
            0.5,
            thick,
            col,
        );
    }
}

/// Renders directional parallel manga speed lines (stream lines) across the canvas.
pub fn render_stream_lines(layer: &mut Layer, sel: &Selection, cfg: &StreamLinesConfig) {
    let count = cfg.count.clamp(10, 500) as usize;
    let rad = cfg.angle_deg.to_radians();
    let (sin_a, cos_a) = rad.sin_cos();
    let (perp_sin, perp_cos) = (-cos_a, sin_a);
    let w = layer.width as f32;
    let h = layer.height as f32;
    let diag = (w * w + h * h).sqrt();

    let mut rng = 0x9e37_79b9 ^ (cfg.count * 17);
    let pseudo_rand = |state: &mut u32| -> f32 {
        *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        (*state >> 8) as f32 / 16777215.0
    };

    let len_base = cfg.length.clamp(50.0, diag);

    for _ in 0..count {
        let span = (pseudo_rand(&mut rng) - 0.5) * diag * 1.2;
        let offset = (pseudo_rand(&mut rng) - 0.5) * diag * 0.9;
        let l_len = len_base * (0.5 + pseudo_rand(&mut rng) * cfg.jitter.clamp(0.0, 1.0));

        let x0 = w * 0.5 + perp_cos * span + cos_a * (offset - l_len * 0.5);
        let y0 = h * 0.5 + perp_sin * span + sin_a * (offset - l_len * 0.5);
        let x1 = x0 + cos_a * l_len;
        let y1 = y0 + sin_a * l_len;

        let thick = cfg.thickness * (0.5 + pseudo_rand(&mut rng) * 0.9);
        draw_tapered_line(layer, sel, x0, y0, x1, y1, 0.4, thick, cfg.color);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_tapered_line(
    layer: &mut Layer,
    sel: &Selection,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    w0: f32,
    w1: f32,
    col: [u8; 4],
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 1.0 {
        return;
    }

    let steps = (dist * 1.5).ceil() as usize;
    let (ux, uy) = (dx / dist, dy / dist);
    let (px, py) = (-uy, ux);

    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let cx = x0 + dx * t;
        let cy = y0 + dy * t;
        let r = (w0 + (w1 - w0) * t) * 0.5;
        let reach = r.ceil() as i32;

        let ix0 = (cx - reach as f32).floor() as i32;
        let iy0 = (cy - reach as f32).floor() as i32;
        let ix1 = (cx + reach as f32).ceil() as i32;
        let iy1 = (cy + reach as f32).ceil() as i32;

        for y in iy0..=iy1 {
            for x in ix0..=ix1 {
                if x < 0 || y < 0 || x >= layer.width as i32 || y >= layer.height as i32 {
                    continue;
                }
                let ux = x as u32;
                let uy = y as u32;
                if !sel.contains(ux, uy) {
                    continue;
                }
                // Distance from pixel center to line center
                let pdx = x as f32 + 0.5 - cx;
                let pdy = y as f32 + 0.5 - cy;
                let d_perp = (pdx * px + pdy * py).abs();
                if d_perp <= r {
                    let alpha = ((r - d_perp + 0.5).clamp(0.0, 1.0) * (col[3] as f32 / 255.0))
                        .clamp(0.0, 1.0);
                    if alpha > 0.01 {
                        let mut pixel_col = col;
                        pixel_col[3] = (alpha * 255.0).round() as u8;
                        layer.blend_pixel_direct(ux, uy, pixel_col, 1.0);
                    }
                }
            }
        }
    }
}

/// Renders a uniform straight line segment onto the layer.
pub fn draw_straight_line(
    layer: &mut Layer,
    sel: &Selection,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    thickness: f32,
    color: [u8; 4],
) {
    let t = thickness.max(1.0);
    draw_tapered_line(layer, sel, x0, y0, x1, y1, t, t, color);
}

/// Renders a rectangular outline onto the layer.
pub fn draw_rect_outline(
    layer: &mut Layer,
    sel: &Selection,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    thickness: f32,
    color: [u8; 4],
) {
    let min_x = x0.min(x1);
    let max_x = x0.max(x1);
    let min_y = y0.min(y1);
    let max_y = y0.max(y1);
    draw_straight_line(layer, sel, min_x, min_y, max_x, min_y, thickness, color);
    draw_straight_line(layer, sel, max_x, min_y, max_x, max_y, thickness, color);
    draw_straight_line(layer, sel, max_x, max_y, min_x, max_y, thickness, color);
    draw_straight_line(layer, sel, min_x, max_y, min_x, min_y, thickness, color);
}

/// Renders an elliptical outline onto the layer.
pub fn draw_ellipse_outline(
    layer: &mut Layer,
    sel: &Selection,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    thickness: f32,
    color: [u8; 4],
) {
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    let rx = (x1 - x0).abs() * 0.5;
    let ry = (y1 - y0).abs() * 0.5;
    if rx < 1.0 || ry < 1.0 {
        return;
    }
    let steps = ((rx.max(ry) * 0.8).clamp(24.0, 360.0)) as usize;
    let step_rad = std::f32::consts::TAU / steps as f32;
    for i in 0..steps {
        let t0 = i as f32 * step_rad;
        let t1 = (i + 1) as f32 * step_rad;
        let px0 = cx + rx * t0.cos();
        let py0 = cy + ry * t0.sin();
        let px1 = cx + rx * t1.cos();
        let py1 = cy + ry * t1.sin();
        draw_straight_line(layer, sel, px0, py0, px1, py1, thickness, color);
    }
}


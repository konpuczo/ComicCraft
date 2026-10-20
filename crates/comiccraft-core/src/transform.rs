//! Clip Studio Paint-style Free Transform, Scale & Rotate (Ctrl+T).
//!
//! Cuts out selected pixels (or active layer region), provides an interactive
//! bounding box with handles, and commits with bilinear resampling upon completion.

use crate::layer::Layer;
use crate::selection::Selection;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransformState {
    pub orig_x0: u32,
    pub orig_y0: u32,
    pub orig_x1: u32,
    pub orig_y1: u32,
    pub patch_w: u32,
    pub patch_h: u32,
    pub patch_pixels: Vec<u8>,
    pub center_x: f32,
    pub center_y: f32,
    pub translate_x: f32,
    pub translate_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub angle_deg: f32,
    /// Mask array of which pixels inside patch_w * patch_h were selected
    pub patch_mask: Vec<bool>,
}

impl TransformState {
    /// Begins a transform session by cutting out pixels from the active layer.
    pub fn begin(layer: &mut Layer, sel: &Selection) -> Option<Self> {
        let (x0, y0, x1, y1) = match sel.bounds {
            Some(b) => b,
            None => {
                // If no selection, find bounding box of non-zero pixels on the layer
                let mut min_x = layer.width;
                let mut min_y = layer.height;
                let mut max_x = 0u32;
                let mut max_y = 0u32;
                let mut has_px = false;
                for y in 0..layer.height {
                    let row = (y * layer.width) as usize * 4;
                    for x in 0..layer.width {
                        if layer.pixels[row + x as usize * 4 + 3] > 0 {
                            min_x = min_x.min(x);
                            min_y = min_y.min(y);
                            max_x = max_x.max(x);
                            max_y = max_y.max(y);
                            has_px = true;
                        }
                    }
                }
                if !has_px {
                    return None;
                }
                (min_x, min_y, max_x, max_y)
            }
        };

        let pw = x1.saturating_sub(x0) + 1;
        let ph = y1.saturating_sub(y0) + 1;
        if pw == 0 || ph == 0 {
            return None;
        }

        let mut patch_pixels = vec![0u8; (pw * ph * 4) as usize];
        let mut patch_mask = vec![false; (pw * ph) as usize];

        for py in 0..ph {
            let ly = y0 + py;
            let l_row = (ly * layer.width) as usize * 4;
            let p_row = (py * pw) as usize * 4;
            for px in 0..pw {
                let lx = x0 + px;
                if sel.contains(lx, ly) {
                    let l_idx = l_row + (lx as usize) * 4;
                    let p_idx = p_row + (px as usize) * 4;
                    patch_pixels[p_idx..p_idx + 4].copy_from_slice(&layer.pixels[l_idx..l_idx + 4]);
                    patch_mask[(py * pw + px) as usize] = true;
                    // Cut out from original layer
                    layer.pixels[l_idx..l_idx + 4].fill(0);
                }
            }
        }

        let cx = x0 as f32 + pw as f32 * 0.5;
        let cy = y0 as f32 + ph as f32 * 0.5;

        Some(Self {
            orig_x0: x0,
            orig_y0: y0,
            orig_x1: x1,
            orig_y1: y1,
            patch_w: pw,
            patch_h: ph,
            patch_pixels,
            center_x: cx,
            center_y: cy,
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            angle_deg: 0.0,
            patch_mask,
        })
    }

    /// Restores the original pixels to the layer without changes.
    pub fn cancel(self, layer: &mut Layer) {
        let pw = self.patch_w;
        let ph = self.patch_h;
        for py in 0..ph {
            let ly = self.orig_y0 + py;
            if ly >= layer.height {
                continue;
            }
            let l_row = (ly * layer.width) as usize * 4;
            let p_row = (py * pw) as usize * 4;
            for px in 0..pw {
                let lx = self.orig_x0 + px;
                if lx >= layer.width {
                    continue;
                }
                if self.patch_mask[(py * pw + px) as usize] {
                    let l_idx = l_row + (lx as usize) * 4;
                    let p_idx = p_row + (px as usize) * 4;
                    layer.pixels[l_idx..l_idx + 4].copy_from_slice(&self.patch_pixels[p_idx..p_idx + 4]);
                }
            }
        }
    }

    /// Bakes the transformed pixels back into the layer using bilinear resampling.
    pub fn commit(self, layer: &mut Layer) {
        let cx = self.center_x + self.translate_x;
        let cy = self.center_y + self.translate_y;
        let sx = self.scale_x.max(0.01);
        let sy = self.scale_y.max(0.01);
        let rad = (-self.angle_deg).to_radians();
        let (sin_a, cos_a) = rad.sin_cos();

        let pw = self.patch_w as f32;
        let ph = self.patch_h as f32;
        let half_pw = pw * 0.5;
        let half_ph = ph * 0.5;

        // Bounding box of the rotated/scaled patch on canvas
        let corners = [
            (-half_pw * sx, -half_ph * sy),
            (half_pw * sx, -half_ph * sy),
            (half_pw * sx, half_ph * sy),
            (-half_pw * sx, half_ph * sy),
        ];

        let mut min_bx = f32::MAX;
        let mut max_bx = f32::MIN;
        let mut min_by = f32::MAX;
        let mut max_by = f32::MIN;

        let fwd_rad = self.angle_deg.to_radians();
        let (fwd_sin, fwd_cos) = fwd_rad.sin_cos();

        for &(lx, ly) in &corners {
            let wx = cx + (lx * fwd_cos - ly * fwd_sin);
            let wy = cy + (lx * fwd_sin + ly * fwd_cos);
            min_bx = min_bx.min(wx);
            max_bx = max_bx.max(wx);
            min_by = min_by.min(wy);
            max_by = max_by.max(wy);
        }

        let ix0 = (min_bx.floor() as i32).max(0) as u32;
        let iy0 = (min_by.floor() as i32).max(0) as u32;
        let ix1 = (max_bx.ceil() as i32).min(layer.width as i32 - 1).max(0) as u32;
        let iy1 = (max_by.ceil() as i32).min(layer.height as i32 - 1).max(0) as u32;

        for y in iy0..=iy1 {
            let row = (y * layer.width) as usize * 4;
            for x in ix0..=ix1 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;

                // Inverse transform back to patch coordinates
                let rx = (dx * cos_a - dy * sin_a) / sx;
                let ry = (dx * sin_a + dy * cos_a) / sy;

                let px = rx + half_pw;
                let py = ry + half_ph;

                if px >= 0.0 && px < pw && py >= 0.0 && py < ph {
                    let col = self.sample_bilinear(px, py);
                    if col[3] > 0 {
                        let l_idx = row + (x as usize) * 4;
                        let dst = [
                            layer.pixels[l_idx],
                            layer.pixels[l_idx + 1],
                            layer.pixels[l_idx + 2],
                            layer.pixels[l_idx + 3],
                        ];
                        let blended = crate::blend::BlendMode::Normal.blend_pixel(dst, col, 1.0);
                        layer.pixels[l_idx..l_idx + 4].copy_from_slice(&blended);
                    }
                }
            }
        }
    }

    /// Bilinear sample inside patch_pixels.
    #[inline]
    pub fn sample_bilinear(&self, px: f32, py: f32) -> [u8; 4] {
        let x0 = px.floor() as usize;
        let y0 = py.floor() as usize;
        let x1 = (x0 + 1).min(self.patch_w as usize - 1);
        let y1 = (y0 + 1).min(self.patch_h as usize - 1);

        let fx = px - x0 as f32;
        let fy = py - y0 as f32;

        let w = self.patch_w as usize;
        let get_px = |x: usize, y: usize| -> [f32; 4] {
            if !self.patch_mask[y * w + x] {
                return [0.0, 0.0, 0.0, 0.0];
            }
            let idx = (y * w + x) * 4;
            [
                self.patch_pixels[idx] as f32,
                self.patch_pixels[idx + 1] as f32,
                self.patch_pixels[idx + 2] as f32,
                self.patch_pixels[idx + 3] as f32,
            ]
        };

        let c00 = get_px(x0, y0);
        let c10 = get_px(x1, y0);
        let c01 = get_px(x0, y1);
        let c11 = get_px(x1, y1);

        let mut out = [0u8; 4];
        for i in 0..4 {
            let top = c00[i] + (c10[i] - c00[i]) * fx;
            let bot = c01[i] + (c11[i] - c01[i]) * fx;
            let val = top + (bot - top) * fy;
            out[i] = val.round().clamp(0.0, 255.0) as u8;
        }
        out
    }
}

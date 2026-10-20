use crate::balloon::SpeechBalloon;
use crate::brush::{MirrorStrokesSlot, StrokeSlot, StrokeState};
use crate::layer::{Layer, LayerKind};
use crate::panel::ComicFrameManager;
use crate::ruler::{PerspectiveRuler, SymmetryRuler};
use crate::selection::{Selection, SelectionBorderPosition};
use crate::stroke::StrokePoint;
use crate::tools::{FillReferenceMode, ToolConfig};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub dpi: f32,
    pub background_color: [u8; 4],
    pub layers: Vec<Layer>,
    pub active_layer_idx: usize,
    pub frame_manager: ComicFrameManager,
    pub symmetry_ruler: SymmetryRuler,
    pub perspective_ruler: PerspectiveRuler,
    #[serde(skip)]
    pub perspective_stroke_start: Option<StrokePoint>,
    #[serde(skip)]
    pub selection: Selection,
    #[serde(skip)]
    pub composite_cache: Option<Vec<u8>>,
    #[serde(skip)]
    pub is_dirty: bool,
    /// Set whenever the composite is rebuilt; the viewport clears it after uploading the texture.
    #[serde(skip)]
    pub texture_stale: bool,
    /// Pending partial recomposite region [x0, y0, x1, y1); None while is_dirty means a full recomposite.
    #[serde(skip)]
    pub dirty_rect: Option<[u32; 4]>,
    /// Stateful stroke engine for active brush/pen/eraser strokes.
    #[serde(skip)]
    pub active_stroke: StrokeSlot,
    /// Mirror stroke engines for symmetrical drawing.
    #[serde(skip)]
    pub mirror_strokes: MirrorStrokesSlot,
    /// Active points accumulated during a stroke on a vector layer.
    #[serde(skip)]
    pub active_vector_points: Vec<StrokePoint>,
    /// Tool configuration for the active stroke.
    #[serde(skip)]
    pub active_stroke_config: Option<ToolConfig>,
}

impl Document {
    pub fn new(name: impl Into<String>, width: u32, height: u32, dpi: f32) -> Self {
        let bg_layer = Layer::new_with_color(1, "Paper", width, height, [255, 255, 255, 255]);
        let ink_layer = Layer::new(2, "Layer 1", width, height);
        let frame_manager = ComicFrameManager::new(width, height, 80.0);
        let selection = Selection::new(width, height);
        let symmetry_ruler = SymmetryRuler {
            enabled: false,
            center_x: width as f32 * 0.5,
            center_y: height as f32 * 0.5,
            angle_deg: 0.0,
            lines: 2,
            line_symmetric: true,
        };

        let perspective_ruler = PerspectiveRuler::new_two_point(width as f32, height as f32);

        let mut doc = Self {
            name: name.into(),
            width,
            height,
            dpi,
            background_color: [255, 255, 255, 255],
            layers: vec![bg_layer, ink_layer],
            active_layer_idx: 1,
            frame_manager,
            symmetry_ruler,
            perspective_ruler,
            perspective_stroke_start: None,
            selection,
            composite_cache: None,
            is_dirty: true,
            texture_stale: true,
            dirty_rect: None,
            active_stroke: StrokeSlot::default(),
            mirror_strokes: MirrorStrokesSlot::default(),
            active_vector_points: Vec::new(),
            active_stroke_config: None,
        };
        doc.update_composite();
        doc
    }



    pub fn active_layer(&self) -> Option<&Layer> {
        self.layers.get(self.active_layer_idx)
    }

    pub fn active_layer_mut(&mut self) -> Option<&mut Layer> {
        self.mark_full_dirty();
        self.layers.get_mut(self.active_layer_idx)
    }

    pub fn add_layer(&mut self, name: impl Into<String>) -> usize {
        let new_id = self.layers.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        let layer = Layer::new(new_id, name, self.width, self.height);
        self.layers.push(layer);
        self.active_layer_idx = self.layers.len() - 1;
        self.mark_full_dirty();
        self.active_layer_idx
    }

    pub fn add_vector_layer(&mut self, name: impl Into<String>) -> usize {
        let new_id = self.layers.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        let mut layer = Layer::new(new_id, name, self.width, self.height);
        layer.kind = crate::layer::LayerKind::Vector;
        self.layers.push(layer);
        self.active_layer_idx = self.layers.len() - 1;
        self.mark_full_dirty();
        self.active_layer_idx
    }

    pub fn add_folder(&mut self, name: impl Into<String>) -> usize {
        let new_id = self.layers.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        let layer = Layer::new_folder(new_id, name);
        self.layers.push(layer);
        self.active_layer_idx = self.layers.len() - 1;
        self.mark_full_dirty();
        self.active_layer_idx
    }

    pub fn toggle_reference_layer(&mut self, idx: usize) {
        if let Some(l) = self.layers.get_mut(idx) {
            l.is_reference = !l.is_reference;
        }
    }

    pub fn add_layer_mask(&mut self, idx: usize) {
        if let Some(l) = self.layers.get_mut(idx) {
            l.add_mask();
            self.mark_full_dirty();
        }
    }

    pub fn delete_layer_mask(&mut self, idx: usize) {
        if let Some(l) = self.layers.get_mut(idx) {
            l.delete_mask();
            self.mark_full_dirty();
        }
    }

    pub fn apply_layer_mask(&mut self, idx: usize) {
        if let Some(l) = self.layers.get_mut(idx) {
            l.apply_mask();
            self.mark_full_dirty();
        }
    }

    pub fn toggle_mask_enabled(&mut self, idx: usize) {
        if let Some(l) = self.layers.get_mut(idx) {
            if l.mask.is_some() {
                l.mask_enabled = !l.mask_enabled;
                self.mark_full_dirty();
            }
        }
    }

    pub fn toggle_editing_mask(&mut self, idx: usize) {
        if let Some(l) = self.layers.get_mut(idx) {
            if l.mask.is_some() {
                l.editing_mask = !l.editing_mask;
            }
        }
    }

    pub fn delete_layer(&mut self, idx: usize) -> bool {
        if self.layers.len() <= 1 || idx >= self.layers.len() {
            return false;
        }
        self.layers.remove(idx);
        if self.active_layer_idx >= self.layers.len() {
            self.active_layer_idx = self.layers.len() - 1;
        }
        self.mark_full_dirty();
        true
    }

    pub fn move_layer_up(&mut self, idx: usize) {
        if idx + 1 < self.layers.len() {
            self.layers.swap(idx, idx + 1);
            if self.active_layer_idx == idx {
                self.active_layer_idx = idx + 1;
            } else if self.active_layer_idx == idx + 1 {
                self.active_layer_idx = idx;
            }
            self.mark_full_dirty();
        }
    }

    pub fn move_layer_down(&mut self, idx: usize) {
        if idx > 0 && idx < self.layers.len() {
            self.layers.swap(idx, idx - 1);
            if self.active_layer_idx == idx {
                self.active_layer_idx = idx - 1;
            } else if self.active_layer_idx == idx - 1 {
                self.active_layer_idx = idx;
            }
            self.mark_full_dirty();
        }
    }

    /// Duplicates the layer at `idx` (CSP Duplicate Layer, Ctrl+J).
    pub fn duplicate_layer(&mut self, idx: usize) -> Option<usize> {
        if idx >= self.layers.len() {
            return None;
        }
        let mut dup = self.layers[idx].clone();
        let new_id = self.layers.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        dup.id = new_id;
        dup.name = format!("{} copy", dup.name);
        let new_idx = idx + 1;
        self.layers.insert(new_idx, dup);
        self.active_layer_idx = new_idx;
        self.mark_full_dirty();
        Some(new_idx)
    }

    /// Merges the layer at `top_idx` down into `top_idx - 1` (CSP Merge Down, Ctrl+E).
    pub fn merge_down(&mut self, top_idx: usize) -> bool {
        if top_idx == 0 || top_idx >= self.layers.len() {
            return false;
        }
        let bot_idx = top_idx - 1;
        let top = self.layers.remove(top_idx);
        let bot = &mut self.layers[bot_idx];
        let mode = top.blend_mode;
        let opacity = top.opacity;

        let top_pixels = if let Some(m) = &top.mask {
            let mut masked = top.pixels.clone();
            for (i, px) in masked.chunks_exact_mut(4).enumerate() {
                px[3] = ((px[3] as u32 * m[i] as u32) / 255) as u8;
            }
            masked
        } else {
            top.pixels
        };

        let total_pixels = bot.pixels.len() / 4;
        for i in 0..total_pixels {
            let idx = i * 4;
            let dst = [bot.pixels[idx], bot.pixels[idx + 1], bot.pixels[idx + 2], bot.pixels[idx + 3]];
            let src = [top_pixels[idx], top_pixels[idx + 1], top_pixels[idx + 2], top_pixels[idx + 3]];
            let out = mode.blend_pixel(dst, src, opacity);
            bot.pixels[idx..idx + 4].copy_from_slice(&out);
        }

        self.active_layer_idx = bot_idx;
        self.mark_full_dirty();
        true
    }

    /// Flattens the entire document composite into a single Background layer (CSP Flatten Image).
    pub fn flatten_image(&mut self) {
        self.update_composite();
        let comp = match &self.composite_cache {
            Some(c) => c.clone(),
            None => return,
        };
        self.layers.clear();
        let mut flat = Layer::new(1, "Background", self.width, self.height);
        flat.pixels = comp;
        self.layers.push(flat);
        self.active_layer_idx = 0;
        self.mark_full_dirty();
    }

    /// Merges all currently visible layers into a single layer (CSP Merge Visible).
    pub fn merge_visible(&mut self) {
        if self.layers.len() <= 1 {
            return;
        }
        self.update_composite();
        let comp = match &self.composite_cache {
            Some(c) => c.clone(),
            None => return,
        };
        let new_id = self.layers.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        self.layers.retain(|l| !l.visible);
        let mut merged = Layer::new(new_id, "Merged", self.width, self.height);
        merged.pixels = comp;
        self.layers.insert(0, merged);
        self.active_layer_idx = 0;
        self.mark_full_dirty();
    }

    /// Strokes the active selection perimeter with `width_px` and `color` (CSP Border Selected Area).
    pub fn stroke_selection_border(
        &mut self,
        width_px: u32,
        position: SelectionBorderPosition,
        color: [u8; 4],
    ) -> bool {
        let border_mask = match self.selection.compute_border_mask(width_px, position) {
            Some(b) => b,
            None => return false,
        };
        let idx = self.active_layer_idx;
        if let Some(layer) = self.layers.get_mut(idx) {
            if layer.locked || layer.kind == LayerKind::Folder {
                return false;
            }
            let w = layer.width as usize;
            for (i, &is_border) in border_mask.iter().enumerate() {
                if is_border {
                    let x = (i % w) as u32;
                    let y = (i / w) as u32;
                    layer.set_pixel(x, y, color);
                }
            }
            self.mark_full_dirty();
            true
        } else {
            false
        }
    }

    /// Converts light/white pixels to transparent alpha and keeps dark lineart as opaque ink (CSP Edit > Convert brightness to opacity).
    pub fn convert_brightness_to_opacity(&mut self) -> bool {
        let idx = self.active_layer_idx;
        let sel = &self.selection;
        if let Some(layer) = self.layers.get_mut(idx) {
            if layer.locked || layer.kind == LayerKind::Folder {
                return false;
            }
            let w = layer.width;
            let h = layer.height;
            for y in 0..h {
                for x in 0..w {
                    if sel.contains(x, y) {
                        let i = ((y * w + x) * 4) as usize;
                        if i + 3 < layer.pixels.len() {
                            let luma = 0.299_f32 * layer.pixels[i] as f32
                                + 0.587_f32 * layer.pixels[i + 1] as f32
                                + 0.114_f32 * layer.pixels[i + 2] as f32;
                            let old_a = layer.pixels[i + 3] as f32 / 255.0_f32;
                            let new_a = ((255.0_f32 - luma) * old_a).clamp(0.0_f32, 255.0_f32) as u8;
                            layer.pixels[i] = 0;
                            layer.pixels[i + 1] = 0;
                            layer.pixels[i + 2] = 0;
                            layer.pixels[i + 3] = new_a;
                        }
                    }
                }
            }
            self.mark_full_dirty();
            true
        } else {
            false
        }
    }

    /// Simplifies all vector strokes on a vector layer using Ramer-Douglas-Peucker (CSP Simplify Vector Line).
    pub fn simplify_vector_layer(&mut self, idx: usize, tolerance: f32) -> bool {
        if let Some(layer) = self.layers.get_mut(idx) {
            if !layer.is_vector() {
                return false;
            }
            for stroke in &mut layer.vector_strokes {
                stroke.points = ramer_douglas_peucker(&stroke.points, tolerance);
            }
        } else {
            return false;
        }
        self.rerasterize_vector_layer(idx);
        true
    }

    /// Increases or decreases the width of all vector strokes on a vector layer by `delta` px (CSP Alter Vector Line Width).
    pub fn adjust_vector_line_width(&mut self, idx: usize, delta: f32) -> bool {
        if let Some(layer) = self.layers.get_mut(idx) {
            if !layer.is_vector() {
                return false;
            }
            for stroke in &mut layer.vector_strokes {
                stroke.style.size = (stroke.style.size + delta).clamp(0.5, 500.0);
            }
        } else {
            return false;
        }
        self.rerasterize_vector_layer(idx);
        true
    }

    /// Flags the entire canvas as needing a recomposite (layer-structure or whole-layer changes).
    pub fn mark_full_dirty(&mut self) {
        self.is_dirty = true;
        self.dirty_rect = None;
    }

    /// Flags only `[x0, y0, x1, y1)` (exclusive end) as needing a recomposite.
    /// If a full recomposite is already pending this is a no-op.
    pub fn mark_rect_dirty(&mut self, rect: [u32; 4]) {
        if self.is_dirty && self.dirty_rect.is_none() {
            return; // full recomposite already pending
        }
        let r = [
            rect[0].min(self.width),
            rect[1].min(self.height),
            rect[2].min(self.width),
            rect[3].min(self.height),
        ];
        if r[2] <= r[0] || r[3] <= r[1] {
            return;
        }
        self.dirty_rect = Some(match self.dirty_rect {
            Some(o) if self.is_dirty => [o[0].min(r[0]), o[1].min(r[1]), o[2].max(r[2]), o[3].max(r[3])],
            _ => r,
        });
        self.is_dirty = true;
    }

    /// Renders all visible layers into the composite buffer (full canvas) with Clip Studio Paint
    /// Clipping Mask support. Marks the GPU texture stale so the viewport re-uploads it.
    pub fn update_composite(&mut self) {
        self.composite_rect(0, 0, self.width, self.height);
        self.is_dirty = false;
        self.dirty_rect = None;
        self.texture_stale = true;
    }

    /// Recomposites whatever is pending. Returns `Some(rect)` if only that region changed,
    /// or `None` if the whole canvas was rebuilt.
    pub fn refresh_composite(&mut self) -> Option<[u32; 4]> {
        let size_ok = self
            .composite_cache
            .as_ref()
            .map_or(false, |c| c.len() == (self.width * self.height * 4) as usize);
        match self.dirty_rect {
            Some(r) if self.is_dirty && size_ok => {
                self.composite_rect(r[0], r[1], r[2], r[3]);
                self.is_dirty = false;
                self.dirty_rect = None;
                Some(r)
            }
            _ => {
                self.composite_rect(0, 0, self.width, self.height);
                self.is_dirty = false;
                self.dirty_rect = None;
                None
            }
        }
    }

    /// Composites the layer stack for the pixel rectangle `[x0, x1) x [y0, y1)`.
    /// Processes row by row (cache friendly) and in parallel across rows.
    fn composite_rect(&mut self, x0: u32, y0: u32, x1: u32, y1: u32) {
        use rayon::prelude::*;

        let w = self.width as usize;
        let h = self.height as usize;
        let total = w * h * 4;
        let mut composite = match self.composite_cache.take() {
            Some(c) if c.len() == total => c,
            _ => vec![0u8; total],
        };

        let (x0, x1) = (x0 as usize, (x1 as usize).min(w));
        let (y0, y1) = (y0 as usize, (y1 as usize).min(h));
        if x1 > x0 && y1 > y0 {
            let rw = x1 - x0;
            let layers = &self.layers;
            let row_stride = w * 4;
            let slice_start = y0 * row_stride;
            let slice_end = y1 * row_stride;
            let sub_slice = &mut composite[slice_start..slice_end];

            let is_vis: Vec<bool> = (0..layers.len())
                .map(|i| {
                    if !layers[i].is_drawable() || layers[i].opacity <= 0.001 {
                        return false;
                    }
                    let mut cur = layers[i].parent;
                    while let Some(pid) = cur {
                        if let Some(p) = layers.iter().find(|l| l.id == pid) {
                            if !p.visible {
                                return false;
                            }
                            cur = p.parent;
                        } else {
                            break;
                        }
                    }
                    layers[i].visible
                })
                .collect();

            let row_fn = |y: usize, row: &mut [u8], base_alpha: &mut [u8]| {
                let dst_row = &mut row[x0 * 4..x1 * 4];
                dst_row.fill(0);

                let mut have_base = false;
                for (l_idx, layer) in layers.iter().enumerate() {
                    if !is_vis[l_idx] {
                        if !layer.clipped_to_below {
                            have_base = false;
                        }
                        continue;
                    }
                    let start = (y * w + x0) * 4;
                    let src_row = &layer.pixels[start..start + rw * 4];
                    let mask_row = if layer.mask_enabled {
                        layer.mask.as_ref().map(|m| &m[y * w + x0..y * w + x0 + rw])
                    } else {
                        None
                    };
                    let opaque_fast = layer.opacity >= 0.999
                        && layer.blend_mode == crate::blend::BlendMode::Normal
                        && mask_row.is_none();

                    if !layer.clipped_to_below {
                        for i in 0..rw {
                            let s = &src_row[i * 4..i * 4 + 4];
                            let mask_val = mask_row.map_or(255, |m| m[i]);
                            let effective_alpha = ((s[3] as u32 * mask_val as u32) / 255) as u8;
                            base_alpha[i] = effective_alpha;
                            if effective_alpha == 0 {
                                continue;
                            }
                            let d = &mut dst_row[i * 4..i * 4 + 4];
                            if opaque_fast && effective_alpha == 255 {
                                d.copy_from_slice(s);
                            } else {
                                let alpha_ratio = mask_val as f32 / 255.0;
                                let out = layer.blend_mode.blend_pixel(
                                    [d[0], d[1], d[2], d[3]],
                                    [s[0], s[1], s[2], s[3]],
                                    layer.opacity * alpha_ratio,
                                );
                                d.copy_from_slice(&out);
                            }
                        }
                        have_base = true;
                    } else {
                        for i in 0..rw {
                            let s = &src_row[i * 4..i * 4 + 4];
                            let mask_val = mask_row.map_or(255, |m| m[i]);
                            let effective_alpha = ((s[3] as u32 * mask_val as u32) / 255) as u8;
                            if effective_alpha == 0 {
                                continue;
                            }
                            let mask = if have_base {
                                base_alpha[i] as f32 / 255.0
                            } else {
                                1.0
                            };
                            let alpha_ratio = (mask_val as f32 / 255.0) * mask;
                            if alpha_ratio <= 0.001 {
                                continue;
                            }
                            let d = &mut dst_row[i * 4..i * 4 + 4];
                            let out = layer.blend_mode.blend_pixel(
                                [d[0], d[1], d[2], d[3]],
                                [s[0], s[1], s[2], s[3]],
                                layer.opacity * alpha_ratio,
                            );
                            d.copy_from_slice(&out);
                        }
                    }
                }
            };

            if y1 - y0 <= 16 {
                let mut base_alpha = vec![0u8; rw];
                for (rel_y, row) in sub_slice.chunks_mut(row_stride).enumerate() {
                    row_fn(y0 + rel_y, row, &mut base_alpha);
                }
            } else {
                sub_slice
                    .par_chunks_mut(row_stride)
                    .enumerate()
                    .for_each_init(
                        || vec![0u8; rw],
                        |base_alpha, (rel_y, row)| {
                            row_fn(y0 + rel_y, row, base_alpha);
                        },
                    );
            }
        }

        self.composite_cache = Some(composite);
    }

    /// Begins a stateful brush/pen/eraser stroke on the active layer.
    pub fn begin_stroke(&mut self, pt: StrokePoint, config: &ToolConfig) {
        let mut dirty = None;
        let idx = self.active_layer_idx;
        self.perspective_stroke_start = Some(pt);
        if let Some(layer) = self.layers.get_mut(idx) {
            if layer.locked {
                return;
            }
            if layer.is_vector() {
                self.active_vector_points.clear();
                self.active_vector_points.push(pt);
                self.active_stroke_config = Some(config.clone());
            }
            let mut state = StrokeState::new(layer, idx, config);
            dirty = state.push(pt, layer, &self.selection);
            self.active_stroke.0 = Some(Box::new(state));

            self.mirror_strokes.0.clear();
            if self.symmetry_ruler.enabled && self.symmetry_ruler.lines >= 2 {
                let counterparts = self.symmetry_ruler.symmetrical_counterparts(pt);
                for cpt in counterparts {
                    let mut m_state = StrokeState::new(layer, idx, config);
                    let m_dirty = m_state.push(cpt, layer, &self.selection);
                    if let Some(md) = m_dirty {
                        dirty = match dirty {
                            Some(d) => Some([d[0].min(md[0]), d[1].min(md[1]), d[2].max(md[2]), d[3].max(md[3])]),
                            None => Some(md),
                        };
                    }
                    self.mirror_strokes.0.push(Box::new(m_state));
                }
            }
        }
        if let Some(r) = dirty {
            self.mark_rect_dirty(r);
        }
    }

    /// Appends a new point to the active stroke on the active layer.
    pub fn continue_stroke(&mut self, pt: StrokePoint) {
        let pt = if let Some(start) = self.perspective_stroke_start {
            self.perspective_ruler.snap_point(start, pt)
        } else {
            pt
        };
        let mut dirty = None;
        if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
            if !layer.locked {
                if layer.is_vector() {
                    self.active_vector_points.push(pt);
                }
                if let Some(state) = self.active_stroke.0.as_mut() {
                    dirty = state.push(pt, layer, &self.selection);
                }
                if self.symmetry_ruler.enabled && !self.mirror_strokes.0.is_empty() {
                    let counterparts = self.symmetry_ruler.symmetrical_counterparts(pt);
                    for (m_state, cpt) in self.mirror_strokes.0.iter_mut().zip(counterparts) {
                        let m_dirty = m_state.push(cpt, layer, &self.selection);
                        if let Some(md) = m_dirty {
                            dirty = match dirty {
                                Some(d) => Some([d[0].min(md[0]), d[1].min(md[1]), d[2].max(md[2]), d[3].max(md[3])]),
                                None => Some(md),
                            };
                        }
                    }
                }
            }
        }
        if let Some(r) = dirty {
            self.mark_rect_dirty(r);
        }
    }

    /// Finishes the active stroke (applies end-taper pass if configured).
    pub fn end_stroke(&mut self) {
        self.perspective_stroke_start = None;
        let mut dirty = None;
        if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
            if !layer.locked {
                if layer.is_vector() && !self.active_vector_points.is_empty() {
                    if let Some(cfg) = self.active_stroke_config.take() {
                        if cfg.tool != crate::tools::ToolKind::Eraser {
                            let stroke = crate::layer::VectorStroke {
                                style: crate::layer::VectorStyle::from_tool_config(&cfg),
                                points: std::mem::take(&mut self.active_vector_points),
                            };
                            layer.vector_strokes.push(stroke);
                        }
                    }
                }
                if let Some(mut state) = self.active_stroke.0.take() {
                    dirty = state.finish(layer, &self.selection);
                }
                for mut m_state in self.mirror_strokes.0.drain(..) {
                    let m_dirty = m_state.finish(layer, &self.selection);
                    if let Some(md) = m_dirty {
                        dirty = match dirty {
                            Some(d) => Some([d[0].min(md[0]), d[1].min(md[1]), d[2].max(md[2]), d[3].max(md[3])]),
                            None => Some(md),
                        };
                    }
                }
            }
        }
        self.active_vector_points.clear();
        self.active_stroke_config = None;
        self.mirror_strokes.0.clear();
        if let Some(r) = dirty {
            self.mark_rect_dirty(r);
        }
    }

    /// Draws a standalone segment between two stroke points onto the active layer (used by CLI / batch tools).
    pub fn paint_stroke_segment(
        &mut self,
        p1: StrokePoint,
        p2: StrokePoint,
        config: &ToolConfig,
    ) {
        self.begin_stroke(p1, config);
        self.continue_stroke(p2);
        self.end_stroke();
    }

    /// Single tap dab stamp at a specific point
    pub fn paint_dab(&mut self, pt: StrokePoint, config: &ToolConfig) {
        self.begin_stroke(pt, config);
        self.end_stroke();
    }


    /// Returns the RGBA reference buffer for Fill / Magic Wand / Close-and-Fill.
    pub fn get_reference_buffer(&mut self, mode: FillReferenceMode) -> Vec<u8> {
        match mode {
            FillReferenceMode::ReferActiveLayer => {
                if let Some(l) = self.layers.get(self.active_layer_idx) {
                    if l.is_drawable() {
                        return l.pixels.clone();
                    }
                }
                vec![0; (self.width * self.height * 4) as usize]
            }
            FillReferenceMode::ReferReferenceLayer => {
                let has_ref = self.layers.iter().any(|l| l.is_reference && l.visible && l.is_drawable());
                if has_ref {
                    let total = (self.width * self.height * 4) as usize;
                    let mut comp = vec![0u8; total];
                    for layer in &self.layers {
                        if !layer.is_reference || !layer.visible || !layer.is_drawable() {
                            continue;
                        }
                        for (d, s) in comp.chunks_exact_mut(4).zip(layer.pixels.chunks_exact(4)) {
                            if s[3] > 0 {
                                let out = layer.blend_mode.blend_pixel(
                                    [d[0], d[1], d[2], d[3]],
                                    [s[0], s[1], s[2], s[3]],
                                    layer.opacity,
                                );
                                d.copy_from_slice(&out);
                            }
                        }
                    }
                    comp
                } else {
                    if self.composite_cache.is_none() || self.is_dirty {
                        self.update_composite();
                    }
                    self.composite_cache.clone().unwrap_or_else(|| vec![0; (self.width * self.height * 4) as usize])
                }
            }
            FillReferenceMode::ReferAllLayers => {
                if self.composite_cache.is_none() || self.is_dirty {
                    self.update_composite();
                }
                self.composite_cache.clone().unwrap_or_else(|| vec![0; (self.width * self.height * 4) as usize])
            }
        }
    }

    /// Flood fill (bucket) starting at (start_x, start_y)
    pub fn flood_fill(&mut self, start_x: u32, start_y: u32, config: &ToolConfig) {
        if start_x >= self.width || start_y >= self.height {
            return;
        }
        if !self.selection.contains(start_x, start_y) {
            return;
        }

        let ref_buf = self.get_reference_buffer(config.fill_mode);
        let opt = crate::fill::RegionOptions {
            tolerance: config.fill_tolerance,
            close_gap: config.fill_close_gap,
            area_scale: config.fill_area_scaling,
            contiguous: true,
        };
        let mask = crate::fill::region_mask(
            &ref_buf,
            self.width as usize,
            self.height as usize,
            start_x as usize,
            start_y as usize,
            opt,
        );

        let (w, h) = (self.width, self.height);
        let idx = self.active_layer_idx;
        if let Some(layer) = self.layers.get_mut(idx) {
            if layer.locked || !layer.is_drawable() {
                return;
            }
            let mut min_x = w;
            let mut min_y = h;
            let mut max_x = 0;
            let mut max_y = 0;
            let mut changed = false;

            for y in 0..h {
                let row = (y * w) as usize;
                for x in 0..w {
                    let i = row + x as usize;
                    if mask[i] && self.selection.contains(x, y) {
                        if layer.editing_mask && layer.mask.is_some() {
                            if let Some(m) = layer.mask.as_mut() {
                                m[i] = if config.primary_color[3] == 0 { 0 } else { 255 };
                            }
                        } else {
                            layer.set_pixel(x, y, config.primary_color);
                        }
                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x);
                        max_y = max_y.max(y);
                        changed = true;
                    }
                }
            }
            if changed {
                self.mark_rect_dirty([min_x, min_y, (max_x + 1).min(w), (max_y + 1).min(h)]);
            }
        }
    }

    /// Auto Select (Magic Wand) starting at (sx, sy)
    pub fn auto_select(&mut self, sx: u32, sy: u32, config: &ToolConfig) {
        if sx >= self.width || sy >= self.height {
            return;
        }
        let ref_buf = self.get_reference_buffer(config.fill_mode);
        let opt = crate::fill::RegionOptions {
            tolerance: config.wand_tolerance,
            close_gap: config.wand_close_gap,
            area_scale: config.wand_area_scaling,
            contiguous: config.wand_contiguous,
        };
        let mask = crate::fill::region_mask(
            &ref_buf,
            self.width as usize,
            self.height as usize,
            sx as usize,
            sy as usize,
            opt,
        );
        self.selection.combine(mask, config.selection_op);
    }

    /// CSP "Close and fill" (Enclose and fill) using a lasso boundary.
    pub fn close_and_fill(&mut self, lasso: &[(f32, f32)], config: &ToolConfig) {
        if lasso.len() < 3 {
            return;
        }
        let ref_buf = self.get_reference_buffer(config.fill_mode);
        let active_only = matches!(config.fill_mode, FillReferenceMode::ReferActiveLayer);
        let barrier = crate::fill::line_barrier(&ref_buf, self.width as usize, self.height as usize, active_only);
        let mask = crate::fill::enclose_fill_mask(
            &barrier,
            self.width as usize,
            self.height as usize,
            lasso,
            config.fill_close_gap,
            config.fill_area_scaling,
        );
        let (w, h) = (self.width, self.height);
        let idx = self.active_layer_idx;
        if let Some(layer) = self.layers.get_mut(idx) {
            if layer.locked || !layer.is_drawable() {
                return;
            }
            let mut min_x = w;
            let mut min_y = h;
            let mut max_x = 0;
            let mut max_y = 0;
            let mut changed = false;

            for y in 0..h {
                let row = (y * w) as usize;
                for x in 0..w {
                    let i = row + x as usize;
                    if mask[i] && self.selection.contains(x, y) {
                        if layer.editing_mask && layer.mask.is_some() {
                            if let Some(m) = layer.mask.as_mut() {
                                m[i] = if config.primary_color[3] == 0 { 0 } else { 255 };
                            }
                        } else {
                            layer.set_pixel(x, y, config.primary_color);
                        }
                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x);
                        max_y = max_y.max(y);
                        changed = true;
                    }
                }
            }
            if changed {
                self.mark_rect_dirty([min_x, min_y, (max_x + 1).min(w), (max_y + 1).min(h)]);
            }
        }
    }

    /// Erases vector strokes that pass within `radius` of `(x, y)` on the active layer.
    pub fn erase_vector_at(&mut self, x: f32, y: f32, radius: f32) -> bool {
        let idx = self.active_layer_idx;
        let mut removed = false;
        if let Some(layer) = self.layers.get_mut(idx) {
            if !layer.is_vector() || layer.locked {
                return false;
            }
            let initial_len = layer.vector_strokes.len();
            layer.vector_strokes.retain(|stroke| {
                if stroke.points.is_empty() {
                    return false;
                }
                let touch_r = radius + stroke.style.size * 0.5;
                let touch_sq = touch_r * touch_r;
                for i in 0..stroke.points.len() {
                    let p = stroke.points[i];
                    let dx = p.x - x;
                    let dy = p.y - y;
                    if dx * dx + dy * dy <= touch_sq {
                        return false;
                    }
                    if i > 0 {
                        let p_prev = stroke.points[i - 1];
                        if dist_to_segment_sq((x, y), (p_prev.x, p_prev.y), (p.x, p.y)) <= touch_sq {
                            return false;
                        }
                    }
                }
                true
            });
            removed = layer.vector_strokes.len() != initial_len;
        }
        if removed {
            self.rerasterize_vector_layer(idx);
        }
        removed
    }

    /// Re-renders all vector strokes onto the vector layer pixels.
    pub fn rerasterize_vector_layer(&mut self, idx: usize) {
        if let Some(layer) = self.layers.get_mut(idx) {
            if !layer.is_vector() {
                return;
            }
            layer.clear();
            let strokes = layer.vector_strokes.clone();
            for stroke in &strokes {
                render_vector_stroke(layer, stroke, &self.selection);
            }
        }
        self.mark_full_dirty();
    }

    /// Eyedropper sample color at (x, y)
    pub fn sample_color(&mut self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        if self.composite_cache.is_none() || self.is_dirty {
            self.update_composite();
        }
        if let Some(comp) = &self.composite_cache {
            let idx = ((y * self.width + x) * 4) as usize;
            if idx + 3 < comp.len() {
                return Some([comp[idx], comp[idx + 1], comp[idx + 2], comp[idx + 3]]);
            }
        }
        None
    }

    /// Clears either the active selection or the entire active layer
    pub fn clear_active_selection_or_layer(&mut self) {
        if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
            if self.selection.is_empty() {
                layer.clear();
            } else {
                for y in 0..self.height {
                    for x in 0..self.width {
                        if self.selection.contains(x, y) {
                            layer.set_pixel(x, y, [0, 0, 0, 0]);
                        }
                    }
                }
            }
            self.mark_full_dirty();
        }
    }

    /// Renders speech balloon to active layer
    pub fn add_speech_balloon(&mut self, balloon: &SpeechBalloon) {
        if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
            balloon.render_to_layer(layer);
            self.mark_full_dirty();
        }
    }

    /// Renders comic panel borders to active layer
    pub fn render_comic_panels(&mut self, border_color: [u8; 4]) {
        if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
            self.frame_manager.render_to_layer(layer, border_color);
            self.mark_full_dirty();
        }
    }

    /// Converts composite into an image::RgbaImage for saving to disk
    pub fn to_rgba_image(&mut self) -> image::RgbaImage {
        self.update_composite();

        let raw = self.composite_cache.clone().unwrap_or_default();
        image::RgbaImage::from_raw(self.width, self.height, raw)
            .unwrap_or_else(|| image::RgbaImage::new(self.width, self.height))
    }
}

fn dist_to_segment_sq(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-6 {
        let ex = p.0 - a.0;
        let ey = p.1 - a.1;
        return ex * ex + ey * ey;
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len_sq).clamp(0.0, 1.0);
    let proj_x = a.0 + t * dx;
    let proj_y = a.1 + t * dy;
    let rx = p.0 - proj_x;
    let ry = p.1 - proj_y;
    rx * rx + ry * ry
}

fn render_vector_stroke(layer: &mut Layer, stroke: &crate::layer::VectorStroke, selection: &Selection) {
    if stroke.points.is_empty() {
        return;
    }
    let w = layer.width as usize;
    let h = layer.height as usize;
    let col = stroke.style.color;
    let base_size = stroke.style.size;
    let opacity = stroke.style.opacity;

    let mut all_points = Vec::new();
    let step = (base_size * 0.25).max(1.0);
    all_points.push(stroke.points[0]);
    for pair in stroke.points.windows(2) {
        let seg = crate::stroke::interpolate_stroke(pair[0], pair[1], step);
        all_points.extend(seg);
    }

    for pt in all_points {
        let r = (base_size * 0.5 * pt.pressure).max(0.5);
        let ir = (r + 1.5).ceil() as i32;
        let cx = pt.x;
        let cy = pt.y;
        let min_x = (cx.floor() as i32 - ir).max(0) as usize;
        let max_x = (cx.ceil() as i32 + ir).min(w as i32 - 1).max(0) as usize;
        let min_y = (cy.floor() as i32 - ir).max(0) as usize;
        let max_y = (cy.ceil() as i32 + ir).min(h as i32 - 1).max(0) as usize;

        for py in min_y..=max_y {
            let dy = py as f32 + 0.5 - cy;
            for px in min_x..=max_x {
                if !selection.contains(px as u32, py as u32) {
                    continue;
                }
                let dx = px as f32 + 0.5 - cx;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= r {
                    let alpha_factor = (r + 0.5 - dist).clamp(0.0, 1.0) * opacity;
                    let idx = (py * w + px) * 4;
                    let existing_a = layer.pixels[idx + 3] as f32 / 255.0;
                    let src_a = (col[3] as f32 / 255.0) * alpha_factor;
                    let out_a = src_a + existing_a * (1.0 - src_a);
                    if out_a > 0.001 {
                        let out_r = (col[0] as f32 * src_a + layer.pixels[idx] as f32 * existing_a * (1.0 - src_a)) / out_a;
                        let out_g = (col[1] as f32 * src_a + layer.pixels[idx + 1] as f32 * existing_a * (1.0 - src_a)) / out_a;
                        let out_b = (col[2] as f32 * src_a + layer.pixels[idx + 2] as f32 * existing_a * (1.0 - src_a)) / out_a;
                        layer.pixels[idx] = out_r.round() as u8;
                        layer.pixels[idx + 1] = out_g.round() as u8;
                        layer.pixels[idx + 2] = out_b.round() as u8;
                        layer.pixels[idx + 3] = (out_a * 255.0).round().min(255.0) as u8;
                    }
                }
            }
        }
    }
}

fn perpendicular_distance(p: StrokePoint, a: StrokePoint, b: StrokePoint) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-6 {
        let px = p.x - a.x;
        let py = p.y - a.y;
        return (px * px + py * py).sqrt();
    }
    ((dy * p.x - dx * p.y + b.x * a.y - b.y * a.x).abs()) / len_sq.sqrt()
}

fn ramer_douglas_peucker(pts: &[StrokePoint], tol: f32) -> Vec<StrokePoint> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let mut max_dist = 0.0_f32;
    let mut index = 0;
    let first = pts[0];
    let last = pts[pts.len() - 1];

    for (i, &p) in pts.iter().enumerate().skip(1).take(pts.len() - 2) {
        let d = perpendicular_distance(p, first, last);
        if d > max_dist {
            max_dist = d;
            index = i;
        }
    }

    if max_dist > tol {
        let mut left = ramer_douglas_peucker(&pts[..=index], tol);
        let right = ramer_douglas_peucker(&pts[index..], tol);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![first, last]
    }
}


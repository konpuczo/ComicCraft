use comiccraft_core::balloon::SpeechBalloon;
use comiccraft_core::blend::BlendMode;
use comiccraft_core::document::Document;
use comiccraft_core::history::HistoryManager;
use comiccraft_core::screentone::{generate_screentone_tile, ScreentoneConfig};
use comiccraft_core::speed_lines::{
    draw_ellipse_outline, draw_rect_outline, draw_straight_line, render_focus_lines,
    render_stream_lines,
};
use comiccraft_core::stroke::{StrokePoint, StrokeStabilizer};
use comiccraft_core::tools::{SubTool, ToolConfig, ToolKind};
use comiccraft_core::transform::TransformState;
use eframe::egui::{
    self, Align2, Color32, ColorImage, FontId, Pos2, Rect, Sense, Stroke, TextureHandle,
    TextureOptions, Vec2,
};
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransformDragMode {
    Move,
    ScaleCorner(usize),
    Rotate,
}

pub struct CanvasViewport {
    pub pan: Vec2,
    pub zoom: f32,
    pub rotation: f32, // in radians
    pub flip_h: bool,
    pub flip_v: bool,
    pub stabilizer: StrokeStabilizer,
    pub last_point: Option<StrokePoint>,
    pub texture: Option<TextureHandle>,
    pub is_drawing: bool,
    pub drag_start_canvas: Option<Pos2>,
    pub lasso_points: Vec<(f32, f32)>,
    pub stroke_point_count: usize,
    pub last_stroke_time: f64,
    /// Hash of layer properties (visibility/opacity/blend/clip/order) used to detect UI edits that need a recomposite.
    pub layer_sig: u64,
    /// Screen rect of the canvas area from the last frame (used by Fit to Screen).
    pub last_rect: Rect,
    pub initialized: bool,
    pub show_manga_guides: bool,
    pub show_grid: bool,
    pub show_rulers: bool,
    pub guides_h: Vec<f32>,
    pub guides_v: Vec<f32>,
    pub last_cursor_canvas: Option<(i32, i32)>,
    pub active_transform: Option<TransformState>,
    pub transform_texture: Option<TextureHandle>,
    pub transform_drag_mode: Option<TransformDragMode>,
}

impl Default for CanvasViewport {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 0.8,
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
            stabilizer: StrokeStabilizer::new(15),
            last_point: None,
            texture: None,
            is_drawing: false,
            drag_start_canvas: None,
            lasso_points: Vec::new(),
            stroke_point_count: 0,
            last_stroke_time: 0.0,
            layer_sig: 0,
            last_rect: Rect::NOTHING,
            initialized: false,
            show_manga_guides: true,
            show_grid: false,
            show_rulers: true,
            guides_h: Vec::new(),
            guides_v: Vec::new(),
            last_cursor_canvas: None,
            active_transform: None,
            transform_texture: None,
            transform_drag_mode: None,
        }
    }
}

impl CanvasViewport {
    /// Begins Free Transform / Scale / Rotate on the active layer or selection.
    pub fn start_transform(&mut self, doc: &mut Document) -> bool {
        if self.active_transform.is_some() {
            return false;
        }
        doc.mark_full_dirty();
        let idx = doc.active_layer_idx;
        if let Some(layer) = doc.layers.get_mut(idx) {
            if let Some(t) = TransformState::begin(layer, &doc.selection) {
                self.active_transform = Some(t);
                self.transform_texture = None;
                self.transform_drag_mode = None;
                return true;
            }
        }
        false
    }

    /// Commits the active transform with bilinear resampling and records undo history.
    pub fn commit_transform(&mut self, doc: &mut Document, history: &mut HistoryManager) {
        if let Some(t) = self.active_transform.take() {
            history.push_state("Free Transform", doc);
            if let Some(layer) = doc.active_layer_mut() {
                t.commit(layer);
            }
            doc.mark_full_dirty();
            self.transform_texture = None;
            self.transform_drag_mode = None;
        }
    }

    /// Cancels the active transform, restoring the original cut pixels.
    pub fn cancel_transform(&mut self, doc: &mut Document) {
        if let Some(t) = self.active_transform.take() {
            if let Some(layer) = doc.active_layer_mut() {
                t.cancel(layer);
            }
            doc.mark_full_dirty();
            self.transform_texture = None;
            self.transform_drag_mode = None;
        }
    }
}

impl CanvasViewport {
    /// Fits the document into the canvas area and centers it. Uses the last known canvas rect;
    /// fallback_size is only used before the first frame has been drawn.
    pub fn reset_view(&mut self, fallback_size: Vec2, doc_width: u32, doc_height: u32) {
        self.rotation = 0.0;
        self.flip_h = false;
        self.flip_v = false;

        let (size, center) = if self.last_rect.is_positive() {
            (self.last_rect.size(), self.last_rect.center().to_vec2())
        } else {
            (fallback_size, fallback_size * 0.5)
        };
        let scale_x = (size.x * 0.86) / doc_width as f32;
        let scale_y = (size.y * 0.86) / doc_height as f32;
        self.zoom = scale_x.min(scale_y).clamp(0.05, 4.0);
        self.pan = center;
    }

    /// Converts screen position to canvas coordinates (origin at center: -w/2 .. w/2)
    pub fn screen_to_canvas(&self, screen_pos: Pos2) -> Pos2 {
        let centered = screen_pos - self.pan;
        let cos = (-self.rotation).cos();
        let sin = (-self.rotation).sin();
        let unrotated_x = centered.x * cos - centered.y * sin;
        let unrotated_y = centered.x * sin + centered.y * cos;

        let unzoomed_x = unrotated_x / self.zoom;
        let unzoomed_y = unrotated_y / self.zoom;

        let flipped_x = if self.flip_h { -unzoomed_x } else { unzoomed_x };
        let flipped_y = if self.flip_v { -unzoomed_y } else { unzoomed_y };

        Pos2::new(flipped_x, flipped_y)
    }

    /// Converts canvas coordinate (centered origin) to screen position
    pub fn canvas_to_screen(&self, canvas_pos: Pos2) -> Pos2 {
        let flipped_x = if self.flip_h { -canvas_pos.x } else { canvas_pos.x };
        let flipped_y = if self.flip_v { -canvas_pos.y } else { canvas_pos.y };

        let zoomed_x = flipped_x * self.zoom;
        let zoomed_y = flipped_y * self.zoom;

        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let rotated_x = zoomed_x * cos - zoomed_y * sin;
        let rotated_y = zoomed_x * sin + zoomed_y * cos;

        Pos2::new(self.pan.x + rotated_x, self.pan.y + rotated_y)
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut Document,
        config: &mut ToolConfig,
        history: &mut HistoryManager,
    ) -> (egui::Response, bool) {
        let mut stroke_started = false;
        let avail = ui.available_size();
        // Reserve 24px at bottom of CentralPanel for the Canvas Navigation Bar
        let canvas_alloc = Vec2::new(avail.x, (avail.y - 25.0).max(200.0));
        let (outer_rect, response) = ui.allocate_exact_size(canvas_alloc, Sense::click_and_drag());
        let painter = ui.painter_at(outer_rect);

        // Dark recessed Clip Studio Paint canvas well backdrop (#1e2024)
        painter.rect_filled(outer_rect, 0.0, Color32::from_rgb(30, 32, 36));

        let ruler_thickness = if self.show_rulers { 18.0 } else { 0.0 };
        let scrollbar_thickness = 11.0;
        let rect = Rect::from_min_max(
            outer_rect.min + Vec2::splat(ruler_thickness),
            outer_rect.max - Vec2::splat(scrollbar_thickness),
        );

        self.last_rect = rect;
        if !self.initialized {
            self.initialized = true;
            self.reset_view(rect.size(), doc.width, doc.height);
        }

        let half_w = doc.width as f32 * 0.5;
        let half_h = doc.height as f32 * 0.5;

        let input = ui.input(|i| i.clone());
        let latest_ptr = input.pointer.latest_pos();

        if let Some(ptr) = latest_ptr {
            if rect.contains(ptr) {
                let cp = self.screen_to_canvas(ptr);
                let px = (cp.x + half_w).round() as i32;
                let py = (cp.y + half_h).round() as i32;
                self.last_cursor_canvas = Some((px, py));
            }
        }

        // Check if Floating Selection Launcher buttons were clicked before canvas tool input
        let mut selection_launcher_consumed = false;
        let mut launcher_draw_info: Option<(Rect, (u32, u32, u32, u32))> = None;
        if !doc.selection.is_empty() {
            if let Some((min_x, min_y, max_x, max_y)) = doc.selection.bounds {
                let s_bl =
                    self.canvas_to_screen(Pos2::new(min_x as f32 - half_w, max_y as f32 - half_h));
                let s_br =
                    self.canvas_to_screen(Pos2::new(max_x as f32 - half_w, max_y as f32 - half_h));
                let bot_y = s_bl.y.max(s_br.y) + 8.0;
                let center_x = (s_bl.x + s_br.x) * 0.5;
                let launcher_w = 524.0;
                let launcher_rect = Rect::from_min_size(
                    Pos2::new(
                        (center_x - launcher_w * 0.5)
                            .clamp(rect.left() + 6.0, rect.right() - launcher_w - 6.0),
                        bot_y.clamp(rect.top() + 8.0, rect.bottom() - 32.0),
                    ),
                    Vec2::new(launcher_w, 24.0),
                );
                launcher_draw_info = Some((launcher_rect, (min_x, min_y, max_x, max_y)));

                let actions = [
                    ("Scale/Rotate", 74.0),
                    ("Cut", 36.0),
                    ("Copy", 38.0),
                    ("Deselect", 50.0),
                    ("Invert", 40.0),
                    ("Clear", 38.0),
                    ("Fill", 36.0),
                    ("Border", 44.0),
                    ("+Grow", 42.0),
                    ("-Shrink", 46.0),
                    ("Tone", 38.0),
                ];
                let mut ax = launcher_rect.left() + 4.0;
                for (act_name, aw) in actions {
                    let btn_rect = Rect::from_min_size(
                        Pos2::new(ax, launcher_rect.top() + 2.0),
                        Vec2::new(aw, 20.0),
                    );
                    let btn_resp = ui.interact(
                        btn_rect,
                        ui.id().with("sel_launch").with(act_name),
                        Sense::click(),
                    );
                    if btn_resp.clicked() {
                        selection_launcher_consumed = true;
                        match act_name {
                            "Scale/Rotate" => {
                                self.start_transform(doc);
                            }
                            "Cut" => {
                                let _ = crate::clipboard_utils::copy_selection_or_layer_to_clipboard(doc, false);
                                history.push_state("Cut Selection", doc);
                                doc.clear_active_selection_or_layer();
                            }
                            "Copy" => {
                                let _ = crate::clipboard_utils::copy_selection_or_layer_to_clipboard(doc, false);
                            }
                            "Deselect" => doc.selection.deselect(),
                            "Invert" => doc.selection.invert(),
                            "Clear" => {
                                history.push_state("Clear Selection", doc);
                                doc.clear_active_selection_or_layer();
                            }
                            "Fill" => {
                                history.push_state("Fill Selection", doc);
                                doc.flood_fill((min_x + max_x) / 2, (min_y + max_y) / 2, config);
                            }
                            "Border" => {
                                history.push_state("Border Selected Area", doc);
                                doc.stroke_selection_border(2, comiccraft_core::SelectionBorderPosition::Centered, config.primary_color);
                            }
                            "+Grow" => {
                                history.push_state("Expand Selection", doc);
                                doc.selection.grow(4);
                            }
                            "-Shrink" => {
                                history.push_state("Shrink Selection", doc);
                                doc.selection.grow(-4);
                            }
                            "Tone" => {
                                history.push_state("New Screentone", doc);
                                let sc = ScreentoneConfig::default();
                                let tiles = generate_screentone_tile(doc.width, doc.height, &sc);
                                let l_idx = doc.add_layer("Tone 60L 30%");
                                if let Some(l) = doc.layers.get_mut(l_idx) {
                                    l.pixels = tiles;
                                    l.blend_mode = BlendMode::Multiply;
                                }
                                doc.mark_full_dirty();
                            }
                            _ => {}
                        }
                    }
                    ax += aw + 2.0;
                }
            }
        }

        // 1. Process Navigation (Zoom with Wheel, Pan with Middle Drag or Space) FIRST
        if response.hovered() && input.raw_scroll_delta.y != 0.0 {
            let zoom_factor = if input.raw_scroll_delta.y > 0.0 {
                1.15
            } else {
                0.87
            };
            let old_zoom = self.zoom;
            self.zoom = (self.zoom * zoom_factor).clamp(0.05, 32.0);

            if let Some(pointer) = latest_ptr {
                let offset = pointer.to_vec2() - self.pan;
                self.pan += offset * (1.0 - self.zoom / old_zoom);
            }
        }

        let is_pan_active = input.pointer.middle_down()
            || (input.key_down(egui::Key::Space) && input.pointer.primary_down())
            || config.tool == ToolKind::Hand;

        if is_pan_active {
            self.pan += input.pointer.delta();
            if self.is_drawing {
                doc.end_stroke();
                self.is_drawing = false;
            }
            self.last_point = None;
        } else if self.active_transform.is_some() {
            let commit_requested = input.key_pressed(egui::Key::Enter);
            let cancel_requested = input.key_pressed(egui::Key::Escape);

            let (cx, cy, hw, hh, rad, patch_w, patch_h) = {
                let t = self.active_transform.as_ref().unwrap();
                (
                    t.center_x + t.translate_x,
                    t.center_y + t.translate_y,
                    (t.patch_w as f32 * t.scale_x) * 0.5,
                    (t.patch_h as f32 * t.scale_y) * 0.5,
                    t.angle_deg.to_radians(),
                    t.patch_w as f32,
                    t.patch_h as f32,
                )
            };
            let (sin_a, cos_a) = rad.sin_cos();

            let local_corners = [
                (-hw, -hh), // TL: 0
                (hw, -hh),  // TR: 1
                (hw, hh),   // BR: 2
                (-hw, hh),  // BL: 3
            ];
            let screen_corners: [Pos2; 4] = std::array::from_fn(|i| {
                let (lx, ly) = local_corners[i];
                let wx = cx + (lx * cos_a - ly * sin_a);
                let wy = cy + (lx * sin_a + ly * cos_a);
                self.canvas_to_screen(Pos2::new(wx - half_w, wy - half_h))
            });
            let screen_center = self.canvas_to_screen(Pos2::new(cx - half_w, cy - half_h));
            let zoom = self.zoom;

            if let Some(ptr) = latest_ptr {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    let mut found_mode = None;
                    for (i, &sc) in screen_corners.iter().enumerate() {
                        let d = ptr.distance(sc);
                        if d <= 14.0 {
                            found_mode = Some(TransformDragMode::ScaleCorner(i));
                            break;
                        } else if d <= 32.0 {
                            found_mode = Some(TransformDragMode::Rotate);
                            break;
                        }
                    }
                    if found_mode.is_none() {
                        let d_center = ptr.distance(screen_center);
                        let max_reach = hw.max(hh) * zoom * 1.4;
                        if d_center <= max_reach.max(30.0) {
                            found_mode = Some(TransformDragMode::Move);
                        }
                    }
                    self.transform_drag_mode = found_mode;
                }

                if input.pointer.primary_down() {
                    let delta = input.pointer.delta();
                    if let Some(t) = self.active_transform.as_mut() {
                        match self.transform_drag_mode {
                            Some(TransformDragMode::Move) => {
                                t.translate_x += delta.x / zoom;
                                t.translate_y += delta.y / zoom;
                            }
                            Some(TransformDragMode::ScaleCorner(_)) => {
                                let cur_dist = ptr.distance(screen_center);
                                let orig_half_diag = (((patch_w * 0.5).powi(2)
                                    + (patch_h * 0.5).powi(2))
                                .sqrt()
                                    * zoom)
                                    .max(6.0);
                                let new_scale = (cur_dist / orig_half_diag).clamp(0.02, 20.0);
                                t.scale_x = new_scale;
                                t.scale_y = new_scale;
                            }
                            Some(TransformDragMode::Rotate) => {
                                let v_curr = ptr - screen_center;
                                let v_prev = (ptr - delta) - screen_center;
                                let a_curr = v_curr.y.atan2(v_curr.x).to_degrees();
                                let a_prev = v_prev.y.atan2(v_prev.x).to_degrees();
                                let d_angle = a_curr - a_prev;
                                t.angle_deg = (t.angle_deg + d_angle).rem_euclid(360.0);
                            }
                            None => {}
                        }
                    }
                } else if response.drag_stopped() {
                    self.transform_drag_mode = None;
                }
            }

            if commit_requested {
                self.commit_transform(doc, history);
            } else if cancel_requested {
                self.cancel_transform(doc);
            }
        } else if !selection_launcher_consumed {
            // 2. Process Tool & Brush Input BEFORE uploading GPU texture
            let is_eyedropper_active = config.tool == ToolKind::Eyedropper || input.modifiers.alt;

            let mut stylus_pressure = None;
            for ev in &input.events {
                if let egui::Event::Touch { force, .. } = ev {
                    if let Some(f) = force {
                        stylus_pressure = Some(*f);
                    }
                }
            }

            if let Some(screen_ptr) = latest_ptr {
                let canvas_pos = self.screen_to_canvas(screen_ptr);
                let pixel_x = (canvas_pos.x + half_w).clamp(0.0, doc.width as f32);
                let pixel_y = (canvas_pos.y + half_h).clamp(0.0, doc.height as f32);

                if is_eyedropper_active && input.pointer.primary_down() {
                    if let Some(sampled) = doc.sample_color(pixel_x as u32, pixel_y as u32) {
                        config.primary_color = sampled;
                    }
                } else if config.tool == ToolKind::Fill {
                    if response.clicked() || response.drag_started_by(egui::PointerButton::Primary)
                    {
                        stroke_started = true;
                        history.push_state("Fill", doc);
                        doc.flood_fill(pixel_x as u32, pixel_y as u32, config);
                    }
                } else if config.tool == ToolKind::Zoom {
                    if response.clicked() {
                        let factor = if input.modifiers.alt { 0.77 } else { 1.3 };
                        let old_zoom = self.zoom;
                        self.zoom = (self.zoom * factor).clamp(0.05, 32.0);
                        let offset = screen_ptr.to_vec2() - self.pan;
                        self.pan += offset * (1.0 - self.zoom / old_zoom);
                    }
                } else if config.tool == ToolKind::SelectionRect {
                    if response.drag_started_by(egui::PointerButton::Primary) {
                        self.drag_start_canvas = Some(Pos2::new(pixel_x, pixel_y));
                    }
                    if let Some(start) = self.drag_start_canvas {
                        if response.drag_stopped() {
                            doc.selection.select_rect(
                                start.x as u32,
                                start.y as u32,
                                pixel_x as u32,
                                pixel_y as u32,
                            );
                            self.drag_start_canvas = None;
                        }
                    }
                } else if config.tool == ToolKind::SelectionLasso {
                    if response.drag_started_by(egui::PointerButton::Primary) {
                        self.lasso_points.clear();
                        self.lasso_points.push((pixel_x, pixel_y));
                    }
                    if input.pointer.primary_down() && !self.lasso_points.is_empty() {
                        self.lasso_points.push((pixel_x, pixel_y));
                    }
                    if response.drag_stopped() {
                        if self.lasso_points.len() >= 3 {
                            doc.selection.select_lasso(&self.lasso_points);
                        }
                        self.lasso_points.clear();
                    }
                } else if config.tool == ToolKind::SelectionEllipse {
                    if response.drag_started_by(egui::PointerButton::Primary) {
                        self.drag_start_canvas = Some(Pos2::new(pixel_x, pixel_y));
                    }
                    if let Some(start) = self.drag_start_canvas {
                        if response.drag_stopped() {
                            let min_x = start.x.min(pixel_x);
                            let max_x = start.x.max(pixel_x);
                            let min_y = start.y.min(pixel_y);
                            let max_y = start.y.max(pixel_y);
                            let cx = (min_x + max_x) * 0.5;
                            let cy = (min_y + max_y) * 0.5;
                            let rx = (max_x - min_x) * 0.5;
                            let ry = (max_y - min_y) * 0.5;
                            if rx > 0.5 && ry > 0.5 {
                                doc.selection.select_ellipse(cx, cy, rx, ry, config.selection_op);
                            }
                            self.drag_start_canvas = None;
                        }
                    }
                } else if config.tool == ToolKind::AutoSelect {
                    if response.clicked() || response.drag_started_by(egui::PointerButton::Primary) {
                        stroke_started = true;
                        history.push_state("Auto Select", doc);
                        doc.auto_select(pixel_x as u32, pixel_y as u32, config);
                    }
                } else if config.tool == ToolKind::CloseAndFill {
                    if response.drag_started_by(egui::PointerButton::Primary) {
                        self.lasso_points.clear();
                        self.lasso_points.push((pixel_x, pixel_y));
                    }
                    if input.pointer.primary_down() && !self.lasso_points.is_empty() {
                        self.lasso_points.push((pixel_x, pixel_y));
                    }
                    if response.drag_stopped() {
                        if self.lasso_points.len() >= 3 {
                            stroke_started = true;
                            history.push_state("Close and Fill", doc);
                            doc.close_and_fill(&self.lasso_points, config);
                        }
                        self.lasso_points.clear();
                    }
                } else if config.tool == ToolKind::ComicFrame {
                    if response.drag_started_by(egui::PointerButton::Primary) {
                        self.drag_start_canvas = Some(Pos2::new(pixel_x, pixel_y));
                    }
                    if let Some(start) = self.drag_start_canvas {
                        if response.drag_stopped() {
                            let dx = pixel_x - start.x;
                            let dy = pixel_y - start.y;
                            if dx.abs() > 10.0 || dy.abs() > 10.0 {
                                stroke_started = true;
                                history.push_state("Split Panel", doc);
                                if dy.abs() > dx.abs() {
                                    let mid_y = (start.y + pixel_y) * 0.5;
                                    doc.frame_manager.split_horizontal(0, mid_y);
                                } else {
                                    let mid_x = (start.x + pixel_x) * 0.5;
                                    doc.frame_manager.split_vertical(0, mid_x);
                                }
                                doc.render_comic_panels([20, 20, 25, 255]);
                            }
                            self.drag_start_canvas = None;
                        }
                    }
                } else if config.tool == ToolKind::SpeechBalloon {
                    if response.drag_started_by(egui::PointerButton::Primary) {
                        self.drag_start_canvas = Some(Pos2::new(pixel_x, pixel_y));
                    }
                    if let Some(start) = self.drag_start_canvas {
                        if response.drag_stopped() {
                            let min_x = start.x.min(pixel_x);
                            let max_x = start.x.max(pixel_x);
                            let min_y = start.y.min(pixel_y);
                            let max_y = start.y.max(pixel_y);
                            let w = max_x - min_x;
                            let h = max_y - min_y;

                            if w > 20.0 && h > 20.0 {
                                stroke_started = true;
                                history.push_state("Add Balloon", doc);
                                let mut balloon = SpeechBalloon::default();
                                balloon.center_x = (min_x + max_x) * 0.5;
                                balloon.center_y = (min_y + max_y) * 0.5;
                                balloon.radius_x = w * 0.5;
                                balloon.radius_y = h * 0.5;
                                balloon.tail_x = Some(balloon.center_x + balloon.radius_x * 0.3);
                                balloon.tail_y = Some(balloon.center_y + balloon.radius_y * 1.3);
                                doc.add_speech_balloon(&balloon);
                            }
                            self.drag_start_canvas = None;
                        }
                    }
                } else if config.tool == ToolKind::Ruler {
                    if response.clicked() || response.drag_started_by(egui::PointerButton::Primary) {
                        if config.sub_tool == SubTool::PerspectiveRuler {
                            doc.perspective_ruler.enabled = true;
                            doc.perspective_ruler.eye_level_y = pixel_y;
                            doc.perspective_ruler.vp1_y = pixel_y;
                            doc.perspective_ruler.vp2_y = pixel_y;
                            doc.mark_full_dirty();
                        } else {
                            doc.symmetry_ruler.enabled = true;
                            doc.symmetry_ruler.center_x = pixel_x;
                            doc.symmetry_ruler.center_y = pixel_y;
                            doc.mark_full_dirty();
                        }
                    }
                } else if config.tool == ToolKind::Figure {
                    match config.sub_tool {
                        SubTool::FocusLines => {
                            if response.clicked() || response.drag_started_by(egui::PointerButton::Primary) {
                                stroke_started = true;
                                history.push_state("Radiation Focus Lines", doc);
                                config.focus_lines.center_x = pixel_x;
                                config.focus_lines.center_y = pixel_y;
                                config.focus_lines.color = config.primary_color;
                                doc.mark_full_dirty();
                                let idx = doc.active_layer_idx;
                                if let Some(layer) = doc.layers.get_mut(idx) {
                                    render_focus_lines(layer, &doc.selection, &config.focus_lines);
                                }
                            }
                        }
                        SubTool::StreamLines => {
                            if response.clicked() || response.drag_started_by(egui::PointerButton::Primary) {
                                stroke_started = true;
                                history.push_state("Directional Speed Lines", doc);
                                config.stream_lines.color = config.primary_color;
                                doc.mark_full_dirty();
                                let idx = doc.active_layer_idx;
                                if let Some(layer) = doc.layers.get_mut(idx) {
                                    render_stream_lines(layer, &doc.selection, &config.stream_lines);
                                }
                            }
                        }
                        SubTool::FigureRect | SubTool::FigureEllipse | SubTool::FigureLine => {
                            if response.drag_started_by(egui::PointerButton::Primary) {
                                self.drag_start_canvas = Some(Pos2::new(pixel_x, pixel_y));
                            }
                            if let Some(start) = self.drag_start_canvas {
                                if response.drag_stopped() {
                                    let dx = (pixel_x - start.x).abs();
                                    let dy = (pixel_y - start.y).abs();
                                    if dx > 2.0 || dy > 2.0 {
                                        stroke_started = true;
                                        let act_name = match config.sub_tool {
                                            SubTool::FigureRect => "Direct Draw Rect",
                                            SubTool::FigureEllipse => "Direct Draw Ellipse",
                                            _ => "Direct Draw Line",
                                        };
                                        history.push_state(act_name, doc);
                                        doc.mark_full_dirty();
                                        let idx = doc.active_layer_idx;
                                        if let Some(layer) = doc.layers.get_mut(idx) {
                                            match config.sub_tool {
                                                SubTool::FigureRect => {
                                                    draw_rect_outline(
                                                        layer,
                                                        &doc.selection,
                                                        start.x,
                                                        start.y,
                                                        pixel_x,
                                                        pixel_y,
                                                        config.size,
                                                        config.primary_color,
                                                    );
                                                }
                                                SubTool::FigureEllipse => {
                                                    draw_ellipse_outline(
                                                        layer,
                                                        &doc.selection,
                                                        start.x,
                                                        start.y,
                                                        pixel_x,
                                                        pixel_y,
                                                        config.size,
                                                        config.primary_color,
                                                    );
                                                }
                                                _ => {
                                                    draw_straight_line(
                                                        layer,
                                                        &doc.selection,
                                                        start.x,
                                                        start.y,
                                                        pixel_x,
                                                        pixel_y,
                                                        config.size,
                                                        config.primary_color,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    self.drag_start_canvas = None;
                                }
                            }
                        }
                        _ => {}
                    }
                } else {
                    let is_brush_tool = matches!(
                        config.tool,
                        ToolKind::Pen
                            | ToolKind::Pencil
                            | ToolKind::Brush
                            | ToolKind::Airbrush
                            | ToolKind::Eraser
                            | ToolKind::Screentone
                    );

                    let is_vector_layer = doc.layers.get(doc.active_layer_idx).map_or(false, |l| l.is_vector());
                    if config.tool == ToolKind::Eraser && is_vector_layer {
                        if response.drag_started_by(egui::PointerButton::Primary) {
                            stroke_started = true;
                            history.push_state("Vector Erase", doc);
                            self.is_drawing = true;
                            doc.erase_vector_at(pixel_x, pixel_y, config.size * 0.5);
                        } else if self.is_drawing && input.pointer.primary_down() {
                            doc.erase_vector_at(pixel_x, pixel_y, config.size * 0.5);
                        }
                        if response.drag_stopped() && self.is_drawing {
                            self.is_drawing = false;
                        }
                    } else if is_brush_tool {
                        if response.drag_started_by(egui::PointerButton::Primary) {
                            stroke_started = true;
                            history.push_state("Brush Stroke", doc);
                            self.is_drawing = true;
                            self.stabilizer = StrokeStabilizer::new(config.stabilization);
                            self.stroke_point_count = 1;
                            self.last_stroke_time = input.time;

                            let initial_pressure = stylus_pressure.unwrap_or(0.65);
                            let pt = StrokePoint::new(pixel_x, pixel_y, initial_pressure);
                            let smooth_pt = self.stabilizer.push(pt);
                            doc.begin_stroke(smooth_pt, config);
                            self.last_point = Some(smooth_pt);
                        } else if self.is_drawing && input.pointer.primary_down() {
                            // Collect all high-frequency PointerMoved samples from this frame
                            let mut screen_samples: Vec<Pos2> = input
                                .events
                                .iter()
                                .filter_map(|ev| match ev {
                                    egui::Event::PointerMoved(p) => Some(*p),
                                    _ => None,
                                })
                                .collect();
                            if screen_samples.is_empty() {
                                screen_samples.push(screen_ptr);
                            }

                            let dt_total = (input.time - self.last_stroke_time).max(0.001) as f32;
                            let dt_step = (dt_total / screen_samples.len() as f32).max(0.0005);
                            self.last_stroke_time = input.time;

                            for sp in screen_samples {
                                let cp = self.screen_to_canvas(sp);
                                let sx = (cp.x + half_w).clamp(0.0, doc.width as f32);
                                let sy = (cp.y + half_h).clamp(0.0, doc.height as f32);

                                let pressure = match stylus_pressure {
                                    Some(p) => p.clamp(0.05, 1.0),
                                    None => {
                                        if let Some(prev) = self.last_point {
                                            let dist =
                                                ((sx - prev.x).powi(2) + (sy - prev.y).powi(2))
                                                    .sqrt();
                                            let speed = dist / dt_step;
                                            let target_p =
                                                (0.88 - (speed / 2400.0)).clamp(0.22, 0.95);
                                            prev.pressure * 0.65 + target_p * 0.35
                                        } else {
                                            0.65
                                        }
                                    }
                                };

                                let raw_point = StrokePoint::new(sx, sy, pressure);
                                let smooth_point = self.stabilizer.push(raw_point);
                                doc.continue_stroke(smooth_point);
                                self.last_point = Some(smooth_point);
                                self.stroke_point_count += 1;
                            }
                        }

                        if response.drag_stopped() && self.is_drawing {
                            doc.end_stroke();
                            self.is_drawing = false;
                            self.last_point = None;
                            self.stabilizer.reset();
                            self.stroke_point_count = 0;
                        }
                    }
                }
            }
        }

        // 3. Synchronize & Upload GPU Texture Immediately After Input Processing
        // Use Nearest-Neighbor magnification so zooming in (>100%) shows razor-sharp pixels
        // without bilinear blur, and Linear minification when zoomed out (<100%).
        const CANVAS_TEX_OPTS: TextureOptions = TextureOptions {
            magnification: egui::TextureFilter::Nearest,
            minification: egui::TextureFilter::Linear,
            wrap_mode: egui::TextureWrapMode::ClampToEdge,
            mipmap_mode: None,
        };

        let sig = layer_signature(doc);
        if sig != self.layer_sig {
            self.layer_sig = sig;
            doc.mark_full_dirty();
        }

        let mut upload: Option<Option<[u32; 4]>> = None;
        if doc.is_dirty {
            upload = Some(doc.refresh_composite());
        }
        if doc.texture_stale || self.texture.is_none() {
            if doc.composite_cache.is_none() {
                doc.update_composite();
            }
            upload = Some(None);
        }
        doc.texture_stale = false;

        if let (Some(region), Some(composite)) = (upload, &doc.composite_cache) {
            let dw = doc.width as usize;
            match (region, &mut self.texture) {
                (Some(r), Some(tex)) => {
                    let rw = (r[2] - r[0]) as usize;
                    let rh = (r[3] - r[1]) as usize;
                    let mut buf: Vec<u8> = Vec::with_capacity(rw * rh * 4);
                    for y in r[1] as usize..r[3] as usize {
                        let start = (y * dw + r[0] as usize) * 4;
                        buf.extend_from_slice(&composite[start..start + rw * 4]);
                    }
                    let img = ColorImage::from_rgba_unmultiplied([rw, rh], &buf);
                    tex.set_partial([r[0] as usize, r[1] as usize], img, CANVAS_TEX_OPTS);
                }
                (_, slot) => {
                    let img =
                        ColorImage::from_rgba_unmultiplied([dw, doc.height as usize], composite);
                    match slot {
                        Some(tex) => tex.set(img, CANVAS_TEX_OPTS),
                        None => {
                            *slot = Some(ui.ctx().load_texture(
                                "canvas_composite",
                                img,
                                CANVAS_TEX_OPTS,
                            ));
                        }
                    }
                }
            }
        }

        if self.is_drawing || self.active_transform.is_some() || doc.is_dirty {
            ui.ctx().request_repaint();
        }


        // 4. Render Clipped Canvas (Drop Shadow + Solid Paper Backing + Textured Mesh + Guides)
        let canvas_painter = painter.with_clip_rect(rect);

        let p_tl = self.canvas_to_screen(Pos2::new(-half_w, -half_h));
        let p_tr = self.canvas_to_screen(Pos2::new(half_w, -half_h));
        let p_br = self.canvas_to_screen(Pos2::new(half_w, half_h));
        let p_bl = self.canvas_to_screen(Pos2::new(-half_w, half_h));

        let shadow_offset_1 = Vec2::new(3.0, 3.0);
        let shadow_offset_2 = Vec2::new(8.0, 8.0);
        canvas_painter.add(egui::Shape::convex_polygon(
            vec![
                p_tl + shadow_offset_2,
                p_tr + shadow_offset_2,
                p_br + shadow_offset_2,
                p_bl + shadow_offset_2,
            ],
            Color32::from_black_alpha(48),
            egui::Stroke::NONE,
        ));
        canvas_painter.add(egui::Shape::convex_polygon(
            vec![
                p_tl + shadow_offset_1,
                p_tr + shadow_offset_1,
                p_br + shadow_offset_1,
                p_bl + shadow_offset_1,
            ],
            Color32::from_black_alpha(85),
            egui::Stroke::NONE,
        ));

        // Solid white paper backing so canvas never flashes dark
        canvas_painter.add(egui::Shape::convex_polygon(
            vec![p_tl, p_tr, p_br, p_bl],
            Color32::WHITE,
            egui::Stroke::NONE,
        ));

        // Draw Textured Canvas Mesh
        if let Some(tex) = &self.texture {
            let mut mesh = egui::Mesh::with_texture(tex.id());
            mesh.add_rect_with_uv(
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            mesh.vertices[0].pos = p_tl;
            mesh.vertices[1].pos = p_tr;
            mesh.vertices[2].pos = p_bl;
            mesh.vertices[3].pos = p_br;

            canvas_painter.add(egui::Shape::mesh(mesh));
        }

        // Canvas Border
        canvas_painter.add(egui::Shape::closed_line(
            vec![p_tl, p_tr, p_br, p_bl],
            egui::Stroke::new(1.0_f32, Color32::from_rgb(105, 110, 118)),
        ));

        // Authentic Clip Studio Paint Manga Manuscript Paper Guidelines (内枠・仕上がり線・トンボ)
        if self.show_manga_guides {
            let guide_cyan = Color32::from_rgba_unmultiplied(0, 165, 240, 175);
            let guide_faint = Color32::from_rgba_unmultiplied(0, 165, 240, 95);

            let in_mx = (doc.width as f32 * 0.10).max(60.0);
            let in_my = (doc.height as f32 * 0.10).max(80.0);
            let in_tl = self.canvas_to_screen(Pos2::new(-half_w + in_mx, -half_h + in_my));
            let in_tr = self.canvas_to_screen(Pos2::new(half_w - in_mx, -half_h + in_my));
            let in_br = self.canvas_to_screen(Pos2::new(half_w - in_mx, half_h - in_my));
            let in_bl = self.canvas_to_screen(Pos2::new(-half_w + in_mx, half_h - in_my));
            canvas_painter.add(egui::Shape::closed_line(
                vec![in_tl, in_tr, in_br, in_bl],
                egui::Stroke::new(1.2_f32, guide_cyan),
            ));

            let tr_mx = (doc.width as f32 * 0.045).max(30.0);
            let tr_my = (doc.height as f32 * 0.045).max(40.0);
            let tr_tl = self.canvas_to_screen(Pos2::new(-half_w + tr_mx, -half_h + tr_my));
            let tr_tr = self.canvas_to_screen(Pos2::new(half_w - tr_mx, -half_h + tr_my));
            let tr_br = self.canvas_to_screen(Pos2::new(half_w - tr_mx, half_h - tr_my));
            let tr_bl = self.canvas_to_screen(Pos2::new(-half_w + tr_mx, half_h - tr_my));
            canvas_painter.add(egui::Shape::closed_line(
                vec![tr_tl, tr_tr, tr_br, tr_bl],
                egui::Stroke::new(1.0_f32, guide_faint),
            ));

            let mark_len = 16.0;
            for corner in [tr_tl, tr_tr, tr_bl, tr_br] {
                canvas_painter.line_segment(
                    [
                        corner - Vec2::new(mark_len, 0.0),
                        corner + Vec2::new(mark_len, 0.0),
                    ],
                    egui::Stroke::new(1.0_f32, guide_cyan),
                );
                canvas_painter.line_segment(
                    [
                        corner - Vec2::new(0.0, mark_len),
                        corner + Vec2::new(0.0, mark_len),
                    ],
                    egui::Stroke::new(1.0_f32, guide_cyan),
                );
            }

            let tr_y0 = -half_h + tr_my;
            let tr_y1 = half_h - tr_my;
            let tr_x0 = -half_w + tr_mx;
            let tr_x1 = half_w - tr_mx;
            let mid_top = self.canvas_to_screen(Pos2::new(0.0, tr_y0));
            let mid_bottom = self.canvas_to_screen(Pos2::new(0.0, tr_y1));
            let mid_left = self.canvas_to_screen(Pos2::new(tr_x0, 0.0));
            let mid_right = self.canvas_to_screen(Pos2::new(tr_x1, 0.0));
            canvas_painter.line_segment(
                [
                    mid_top - Vec2::new(0.0, mark_len),
                    mid_top + Vec2::new(0.0, mark_len),
                ],
                egui::Stroke::new(1.0_f32, guide_cyan),
            );
            canvas_painter.line_segment(
                [
                    mid_bottom - Vec2::new(0.0, mark_len),
                    mid_bottom + Vec2::new(0.0, mark_len),
                ],
                egui::Stroke::new(1.0_f32, guide_cyan),
            );
            canvas_painter.line_segment(
                [
                    mid_left - Vec2::new(mark_len, 0.0),
                    mid_left + Vec2::new(mark_len, 0.0),
                ],
                egui::Stroke::new(1.0_f32, guide_cyan),
            );
            canvas_painter.line_segment(
                [
                    mid_right - Vec2::new(mark_len, 0.0),
                    mid_right + Vec2::new(mark_len, 0.0),
                ],
                egui::Stroke::new(1.0_f32, guide_cyan),
            );
        }

        // Symmetry Ruler Guide Lines (CSP Authentic Purple / Cyan Guideline)
        if doc.symmetry_ruler.enabled && doc.symmetry_ruler.lines >= 2 {
            let r_cx = doc.symmetry_ruler.center_x;
            let r_cy = doc.symmetry_ruler.center_y;
            let r_screen = self.canvas_to_screen(Pos2::new(r_cx - half_w, r_cy - half_h));
            let n = doc.symmetry_ruler.lines;
            let base_rad = doc.symmetry_ruler.angle_deg.to_radians();
            let sector_rad = std::f32::consts::TAU / n as f32;
            let ruler_stroke = Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(185, 120, 255, 190));
            let ruler_len = 5000.0 * self.zoom;

            for i in 0..n {
                let theta = base_rad + i as f32 * sector_rad;
                let (sin_t, cos_t) = theta.sin_cos();
                let p0 = r_screen - Vec2::new(cos_t, sin_t) * ruler_len;
                let p1 = r_screen + Vec2::new(cos_t, sin_t) * ruler_len;
                canvas_painter.line_segment([p0, p1], ruler_stroke);
            }

            canvas_painter.circle_filled(r_screen, 5.0, Color32::from_rgb(185, 120, 255));
            canvas_painter.circle_stroke(r_screen, 5.0, Stroke::new(1.5_f32, Color32::WHITE));
        }

        // Perspective Ruler Guide Lines (CSP Authentic Cyan / Purple Horizon & Vanishing Rays)
        if doc.perspective_ruler.enabled {
            let pr = &doc.perspective_ruler;
            let eye_y_screen = self.canvas_to_screen(Pos2::new(0.0, pr.eye_level_y - half_h)).y;
            let horizon_stroke = Stroke::new(1.5_f32, Color32::from_rgb(45, 175, 245));
            // Horizon line across canvas view
            canvas_painter.line_segment(
                [
                    Pos2::new(rect.min.x - 2000.0, eye_y_screen),
                    Pos2::new(rect.max.x + 2000.0, eye_y_screen),
                ],
                horizon_stroke,
            );

            let vp1_screen = self.canvas_to_screen(Pos2::new(pr.vp1_x - half_w, pr.vp1_y - half_h));
            let ray_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(65, 180, 255, 95));
            let num_rays = pr.guide_divisions.clamp(4, 24);

            // Draw rays from VP1
            for i in 0..num_rays {
                let frac = i as f32 / (num_rays.saturating_sub(1).max(1)) as f32;
                let target_y = -half_h + (doc.height as f32) * frac;
                let target_screen = self.canvas_to_screen(Pos2::new(half_w, target_y));
                let dir = (target_screen - vp1_screen).normalized();
                canvas_painter.line_segment([vp1_screen, vp1_screen + dir * 6000.0 * self.zoom], ray_stroke);
            }
            // Draw VP1 handle
            canvas_painter.circle_filled(vp1_screen, 5.0, Color32::from_rgb(45, 175, 245));
            canvas_painter.circle_stroke(vp1_screen, 5.0, Stroke::new(1.5_f32, Color32::WHITE));

            if matches!(
                pr.kind,
                comiccraft_core::PerspectiveKind::TwoPoint | comiccraft_core::PerspectiveKind::ThreePoint
            ) {
                let vp2_screen = self.canvas_to_screen(Pos2::new(pr.vp2_x - half_w, pr.vp2_y - half_h));
                let ray2_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(190, 110, 255, 95));
                for i in 0..num_rays {
                    let frac = i as f32 / (num_rays.saturating_sub(1).max(1)) as f32;
                    let target_y = -half_h + (doc.height as f32) * frac;
                    let target_screen = self.canvas_to_screen(Pos2::new(-half_w, target_y));
                    let dir = (target_screen - vp2_screen).normalized();
                    canvas_painter.line_segment([vp2_screen, vp2_screen + dir * 6000.0 * self.zoom], ray2_stroke);
                }
                // Draw VP2 handle
                canvas_painter.circle_filled(vp2_screen, 5.0, Color32::from_rgb(190, 110, 255));
                canvas_painter.circle_stroke(vp2_screen, 5.0, Stroke::new(1.5_f32, Color32::WHITE));
            }
        }

        // Interactive Canvas Guidelines (Cyan guides)
        let guide_color = Color32::from_rgb(0, 195, 255);
        for &gh in &self.guides_h {
            let pt1 = self.canvas_to_screen(Pos2::new(-half_w, gh - half_h));
            let pt2 = self.canvas_to_screen(Pos2::new(half_w, gh - half_h));
            canvas_painter.line_segment([pt1, pt2], egui::Stroke::new(1.0_f32, guide_color));
        }
        for &gv in &self.guides_v {
            let pt1 = self.canvas_to_screen(Pos2::new(gv - half_w, -half_h));
            let pt2 = self.canvas_to_screen(Pos2::new(gv - half_w, half_h));
            canvas_painter.line_segment([pt1, pt2], egui::Stroke::new(1.0_f32, guide_color));
        }

        // Canvas Grid (100px structural grid)
        if self.show_grid {
            let grid_step = 100.0_f32;
            let grid_color = Color32::from_rgba_unmultiplied(160, 170, 185, 40);
            let mut gx = -half_w + grid_step;
            while gx < half_w {
                let pt1 = self.canvas_to_screen(Pos2::new(gx, -half_h));
                let pt2 = self.canvas_to_screen(Pos2::new(gx, half_h));
                canvas_painter.line_segment([pt1, pt2], egui::Stroke::new(0.75_f32, grid_color));
                gx += grid_step;
            }
            let mut gy = -half_h + grid_step;
            while gy < half_h {
                let pt1 = self.canvas_to_screen(Pos2::new(-half_w, gy));
                let pt2 = self.canvas_to_screen(Pos2::new(half_w, gy));
                canvas_painter.line_segment([pt1, pt2], egui::Stroke::new(0.75_f32, grid_color));
                gy += grid_step;
            }
        }

        // High-Magnification Pixel Grid (Clip Studio Paint pixel boundaries when zoomed >= 1000%)
        if self.zoom >= 10.0 && self.rotation.abs() < 0.01 {
            let tl_canvas = self.screen_to_canvas(rect.left_top());
            let br_canvas = self.screen_to_canvas(rect.right_bottom());
            let px0 = ((tl_canvas.x.min(br_canvas.x) + half_w).floor() as i32).clamp(0, doc.width as i32);
            let px1 = ((tl_canvas.x.max(br_canvas.x) + half_w).ceil() as i32).clamp(0, doc.width as i32);
            let py0 = ((tl_canvas.y.min(br_canvas.y) + half_h).floor() as i32).clamp(0, doc.height as i32);
            let py1 = ((tl_canvas.y.max(br_canvas.y) + half_h).ceil() as i32).clamp(0, doc.height as i32);

            let alpha = ((self.zoom - 10.0) * 4.5).clamp(14.0, 48.0) as u8;
            let px_grid_col = Color32::from_rgba_unmultiplied(125, 132, 145, alpha);
            if (px1 - px0) <= 260 && (py1 - py0) <= 260 {
                let sy_top = self.canvas_to_screen(Pos2::new(0.0, py0 as f32 - half_h)).y;
                let sy_bot = self.canvas_to_screen(Pos2::new(0.0, py1 as f32 - half_h)).y;
                for x in px0..=px1 {
                    let sx = self.canvas_to_screen(Pos2::new(x as f32 - half_w, 0.0)).x;
                    canvas_painter.line_segment(
                        [Pos2::new(sx, sy_top), Pos2::new(sx, sy_bot)],
                        Stroke::new(0.6_f32, px_grid_col),
                    );
                }
                let sx_left = self.canvas_to_screen(Pos2::new(px0 as f32 - half_w, 0.0)).x;
                let sx_right = self.canvas_to_screen(Pos2::new(px1 as f32 - half_w, 0.0)).x;
                for y in py0..=py1 {
                    let sy = self.canvas_to_screen(Pos2::new(0.0, y as f32 - half_h)).y;
                    canvas_painter.line_segment(
                        [Pos2::new(sx_left, sy), Pos2::new(sx_right, sy)],
                        Stroke::new(0.6_f32, px_grid_col),
                    );
                }
            }
        }

        // Tool Drag Previews (Selection Rect / Lasso / Frame Split / Balloon)
        if let Some(screen_ptr) = latest_ptr {
            let canvas_pos = self.screen_to_canvas(screen_ptr);
            let pixel_x = (canvas_pos.x + half_w).clamp(0.0, doc.width as f32);
            let pixel_y = (canvas_pos.y + half_h).clamp(0.0, doc.height as f32);

            if config.tool == ToolKind::SelectionRect {
                if let Some(start) = self.drag_start_canvas {
                    let sp1 = self.canvas_to_screen(Pos2::new(start.x - half_w, start.y - half_h));
                    let sp2 = self.canvas_to_screen(Pos2::new(pixel_x - half_w, start.y - half_h));
                    let sp3 = self.canvas_to_screen(Pos2::new(pixel_x - half_w, pixel_y - half_h));
                    let sp4 = self.canvas_to_screen(Pos2::new(start.x - half_w, pixel_y - half_h));
                    canvas_painter.add(egui::Shape::closed_line(
                        vec![sp1, sp2, sp3, sp4],
                        egui::Stroke::new(1.5_f32, Color32::from_rgb(0, 160, 255)),
                    ));
                }
            } else if config.tool == ToolKind::SelectionEllipse {
                if let Some(start) = self.drag_start_canvas {
                    let min_x = start.x.min(pixel_x);
                    let max_x = start.x.max(pixel_x);
                    let min_y = start.y.min(pixel_y);
                    let max_y = start.y.max(pixel_y);
                    let cx = (min_x + max_x) * 0.5;
                    let cy = (min_y + max_y) * 0.5;
                    let rx = (max_x - min_x) * 0.5;
                    let ry = (max_y - min_y) * 0.5;
                    let steps = 48;
                    let mut pts = Vec::with_capacity(steps);
                    for step in 0..steps {
                        let theta = step as f32 * std::f32::consts::TAU / steps as f32;
                        let ex = cx + rx * theta.cos();
                        let ey = cy + ry * theta.sin();
                        pts.push(self.canvas_to_screen(Pos2::new(ex - half_w, ey - half_h)));
                    }
                    canvas_painter.add(egui::Shape::closed_line(
                        pts,
                        egui::Stroke::new(1.5_f32, Color32::from_rgb(0, 160, 255)),
                    ));
                }
            } else if (config.tool == ToolKind::SelectionLasso || config.tool == ToolKind::CloseAndFill) && self.lasso_points.len() >= 2 {
                let screen_pts: Vec<Pos2> = self
                    .lasso_points
                    .iter()
                    .map(|&(px, py)| self.canvas_to_screen(Pos2::new(px - half_w, py - half_h)))
                    .collect();
                let col = if config.tool == ToolKind::CloseAndFill {
                    Color32::from_rgb(60, 220, 120)
                } else {
                    Color32::from_rgb(0, 160, 255)
                };
                canvas_painter.add(egui::Shape::line(
                    screen_pts,
                    egui::Stroke::new(1.5_f32, col),
                ));
            } else if config.tool == ToolKind::ComicFrame {
                if let Some(start) = self.drag_start_canvas {
                    let sp_start =
                        self.canvas_to_screen(Pos2::new(start.x - half_w, start.y - half_h));
                    let sp_curr =
                        self.canvas_to_screen(Pos2::new(pixel_x - half_w, pixel_y - half_h));
                    canvas_painter.add(egui::Shape::line_segment(
                        [sp_start, sp_curr],
                        egui::Stroke::new(2.5_f32, Color32::from_rgb(255, 100, 50)),
                    ));
                }
            } else if config.tool == ToolKind::SpeechBalloon {
                if let Some(start) = self.drag_start_canvas {
                    let sp_start =
                        self.canvas_to_screen(Pos2::new(start.x - half_w, start.y - half_h));
                    let sp_curr =
                        self.canvas_to_screen(Pos2::new(pixel_x - half_w, pixel_y - half_h));
                    let preview_rect = Rect::from_two_pos(sp_start, sp_curr);
                    canvas_painter.rect_stroke(
                        preview_rect,
                        16.0,
                        egui::Stroke::new(2.0_f32, Color32::from_rgb(60, 180, 240)),
                    );
                }
            } else if config.tool == ToolKind::Figure {
                if let Some(start) = self.drag_start_canvas {
                    let sp_start =
                        self.canvas_to_screen(Pos2::new(start.x - half_w, start.y - half_h));
                    let sp_curr =
                        self.canvas_to_screen(Pos2::new(pixel_x - half_w, pixel_y - half_h));
                    let stroke_col = Color32::from_rgb(0, 160, 255);
                    match config.sub_tool {
                        SubTool::FigureRect => {
                            let p_rect = Rect::from_two_pos(sp_start, sp_curr);
                            canvas_painter.rect_stroke(p_rect, 0.0, Stroke::new(1.5_f32, stroke_col));
                        }
                        SubTool::FigureEllipse => {
                            let p_rect = Rect::from_two_pos(sp_start, sp_curr);
                            canvas_painter.circle_stroke(
                                p_rect.center(),
                                p_rect.width().max(p_rect.height()) * 0.5,
                                Stroke::new(1.5_f32, stroke_col),
                            );
                        }
                        SubTool::FigureLine => {
                            canvas_painter.line_segment([sp_start, sp_curr], Stroke::new(1.5_f32, stroke_col));
                        }
                        _ => {}
                    }
                }
            }
        }

        // Draw Selection Bounds + CSP Floating Selection Launcher Bar
        if let Some((launcher_rect, (min_x, min_y, max_x, max_y))) = launcher_draw_info {
            let s_tl = self.canvas_to_screen(Pos2::new(min_x as f32 - half_w, min_y as f32 - half_h));
            let s_tr = self.canvas_to_screen(Pos2::new(max_x as f32 - half_w, min_y as f32 - half_h));
            let s_br = self.canvas_to_screen(Pos2::new(max_x as f32 - half_w, max_y as f32 - half_h));
            let s_bl = self.canvas_to_screen(Pos2::new(min_x as f32 - half_w, max_y as f32 - half_h));

            canvas_painter.add(egui::Shape::convex_polygon(
                vec![s_tl, s_tr, s_br, s_bl],
                Color32::from_rgba_unmultiplied(60, 140, 240, 22),
                egui::Stroke::NONE,
            ));
            canvas_painter.add(egui::Shape::closed_line(
                vec![s_tl, s_tr, s_br, s_bl],
                egui::Stroke::new(1.5_f32, Color32::from_rgb(45, 135, 235)),
            ));

            canvas_painter.rect_filled(launcher_rect, 3.0, Color32::from_rgb(38, 41, 46));
            canvas_painter.rect_stroke(
                launcher_rect,
                3.0,
                Stroke::new(1.0_f32, Color32::from_rgb(80, 86, 96)),
            );

            let actions = [
                ("Scale/Rotate", 74.0),
                ("Cut", 36.0),
                ("Copy", 38.0),
                ("Deselect", 50.0),
                ("Invert", 40.0),
                ("Clear", 38.0),
                ("Fill", 36.0),
                ("Border", 44.0),
                ("+Grow", 42.0),
                ("-Shrink", 46.0),
                ("Tone", 38.0),
            ];
            let mut ax = launcher_rect.left() + 4.0;
            for (act_name, aw) in actions {
                let btn_rect = Rect::from_min_size(
                    Pos2::new(ax, launcher_rect.top() + 2.0),
                    Vec2::new(aw, 20.0),
                );
                if let Some(ptr) = latest_ptr {
                    if btn_rect.contains(ptr) {
                        canvas_painter.rect_filled(
                            btn_rect,
                            2.0,
                            Color32::from_rgb(56, 110, 195),
                        );
                    }
                }
                canvas_painter.text(
                    btn_rect.center(),
                    Align2::CENTER_CENTER,
                    act_name,
                    FontId::proportional(10.5),
                    Color32::WHITE,
                );
                ax += aw + 2.0;
            }
        }

        // ---------------- Free Transform Quad & Interactive Controls ----------------
        if let Some(t) = &self.active_transform {
            if self.transform_texture.is_none() {
                let img = ColorImage::from_rgba_unmultiplied(
                    [t.patch_w as usize, t.patch_h as usize],
                    &t.patch_pixels,
                );
                self.transform_texture = Some(ui.ctx().load_texture(
                    "transform_patch",
                    img,
                    TextureOptions::LINEAR,
                ));
            }

            let cx = t.center_x + t.translate_x;
            let cy = t.center_y + t.translate_y;
            let hw = (t.patch_w as f32 * t.scale_x) * 0.5;
            let hh = (t.patch_h as f32 * t.scale_y) * 0.5;
            let rad = t.angle_deg.to_radians();
            let (sin_a, cos_a) = rad.sin_cos();

            let local_corners = [
                (-hw, -hh), // TL: 0
                (hw, -hh),  // TR: 1
                (hw, hh),   // BR: 2
                (-hw, hh),  // BL: 3
            ];
            let screen_corners: [Pos2; 4] = std::array::from_fn(|i| {
                let (lx, ly) = local_corners[i];
                let wx = cx + (lx * cos_a - ly * sin_a);
                let wy = cy + (lx * sin_a + ly * cos_a);
                self.canvas_to_screen(Pos2::new(wx - half_w, wy - half_h))
            });
            let screen_center = self.canvas_to_screen(Pos2::new(cx - half_w, cy - half_h));

            // Render GPU textured quad
            if let Some(tex) = &self.transform_texture {
                let mut mesh = egui::Mesh::with_texture(tex.id());
                let idx0 = mesh.vertices.len() as u32;
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: screen_corners[0],
                    uv: Pos2::new(0.0, 0.0),
                    color: Color32::WHITE,
                });
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: screen_corners[1],
                    uv: Pos2::new(1.0, 0.0),
                    color: Color32::WHITE,
                });
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: screen_corners[2],
                    uv: Pos2::new(1.0, 1.0),
                    color: Color32::WHITE,
                });
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: screen_corners[3],
                    uv: Pos2::new(0.0, 1.0),
                    color: Color32::WHITE,
                });
                mesh.add_triangle(idx0, idx0 + 1, idx0 + 2);
                mesh.add_triangle(idx0, idx0 + 2, idx0 + 3);
                canvas_painter.add(egui::Shape::mesh(mesh));
            }

            // Cyan bounding box lines
            let bbox_col = Color32::from_rgb(0, 190, 255);
            canvas_painter.add(egui::Shape::closed_line(
                screen_corners.to_vec(),
                Stroke::new(1.8_f32, bbox_col),
            ));

            // 8 handles (4 corners + 4 edge midpoints)
            let midpoints = [
                Pos2::new(
                    (screen_corners[0].x + screen_corners[1].x) * 0.5,
                    (screen_corners[0].y + screen_corners[1].y) * 0.5,
                ),
                Pos2::new(
                    (screen_corners[1].x + screen_corners[2].x) * 0.5,
                    (screen_corners[1].y + screen_corners[2].y) * 0.5,
                ),
                Pos2::new(
                    (screen_corners[2].x + screen_corners[3].x) * 0.5,
                    (screen_corners[2].y + screen_corners[3].y) * 0.5,
                ),
                Pos2::new(
                    (screen_corners[3].x + screen_corners[0].x) * 0.5,
                    (screen_corners[3].y + screen_corners[0].y) * 0.5,
                ),
            ];
            for p in screen_corners.iter().chain(midpoints.iter()) {
                let h_rect = Rect::from_center_size(*p, Vec2::splat(7.0));
                canvas_painter.rect_filled(h_rect, 1.0, Color32::WHITE);
                canvas_painter.rect_stroke(
                    h_rect,
                    1.0,
                    Stroke::new(1.0_f32, Color32::from_rgb(20, 40, 60)),
                );
            }

            // Center pivot
            canvas_painter.circle_filled(screen_center, 4.0, Color32::WHITE);
            canvas_painter.circle_stroke(screen_center, 4.0, Stroke::new(1.5_f32, bbox_col));

            // Floating Transform HUD bar below the box
            let max_sy = screen_corners.iter().map(|p| p.y).fold(f32::MIN, f32::max);
            let hud_pos = Pos2::new(
                screen_center.x - 130.0,
                (max_sy + 14.0).min(rect.bottom() - 32.0),
            );
            let hud_rect = Rect::from_min_size(hud_pos, Vec2::new(260.0, 26.0));
            canvas_painter.rect_filled(hud_rect, 3.0, Color32::from_rgb(34, 37, 42));
            canvas_painter.rect_stroke(
                hud_rect,
                3.0,
                Stroke::new(1.0_f32, Color32::from_rgb(70, 75, 85)),
            );

            let info_text = format!(
                "Scale: {:.0}%  Angle: {:.1}°",
                t.scale_x * 100.0,
                t.angle_deg
            );
            canvas_painter.text(
                Pos2::new(hud_rect.left() + 8.0, hud_rect.center().y),
                Align2::LEFT_CENTER,
                info_text,
                FontId::proportional(11.0),
                Color32::from_gray(220),
            );

            let commit_rect = Rect::from_min_size(
                Pos2::new(hud_rect.right() - 86.0, hud_rect.top() + 3.0),
                Vec2::new(40.0, 20.0),
            );
            let cancel_rect = Rect::from_min_size(
                Pos2::new(hud_rect.right() - 44.0, hud_rect.top() + 3.0),
                Vec2::new(40.0, 20.0),
            );

            let commit_resp =
                ui.interact(commit_rect, ui.id().with("hud_commit"), Sense::click());
            let cancel_resp =
                ui.interact(cancel_rect, ui.id().with("hud_cancel"), Sense::click());

            let c_col = if commit_resp.hovered() {
                Color32::from_rgb(45, 140, 235)
            } else {
                Color32::from_rgb(35, 90, 160)
            };
            canvas_painter.rect_filled(commit_rect, 2.0, c_col);
            canvas_painter.text(
                commit_rect.center(),
                Align2::CENTER_CENTER,
                "OK",
                FontId::proportional(10.5),
                Color32::WHITE,
            );

            let can_col = if cancel_resp.hovered() {
                Color32::from_rgb(180, 50, 50)
            } else {
                Color32::from_rgb(120, 40, 40)
            };
            canvas_painter.rect_filled(cancel_rect, 2.0, can_col);
            canvas_painter.text(
                cancel_rect.center(),
                Align2::CENTER_CENTER,
                "Cancel",
                FontId::proportional(10.5),
                Color32::WHITE,
            );

            if commit_resp.clicked() {
                self.commit_transform(doc, history);
            }
            if cancel_resp.clicked() {
                self.cancel_transform(doc);
            }
        }

        // Authentic Clip Studio Paint Brush Outline Cursor (circular or angled nib)
        if response.hovered() && !input.pointer.middle_down() && !input.key_down(egui::Key::Space) {
            if let Some(screen_ptr) = latest_ptr {
                if rect.contains(screen_ptr) {
                    let is_brush = matches!(
                        config.tool,
                        ToolKind::Pen
                            | ToolKind::Pencil
                            | ToolKind::Brush
                            | ToolKind::Airbrush
                            | ToolKind::Eraser
                            | ToolKind::Screentone
                    );
                    if is_brush {
                        let radius = (config.size * 0.5 * self.zoom).max(1.5);
                        let preset = comiccraft_core::BrushPreset::for_tool(
                            config.tool,
                            config.sub_tool,
                        );
                        if preset.aspect < 0.95 {
                            let ry = (radius * preset.aspect).max(1.0);
                            let ang = preset.angle_deg.to_radians() + self.rotation;
                            let (sin_a, cos_a) = ang.sin_cos();
                            let steps = 28;
                            let mut outer_pts = Vec::with_capacity(steps);
                            let mut inner_pts = Vec::with_capacity(steps);
                            for i in 0..steps {
                                let t = (i as f32 / steps as f32) * std::f32::consts::TAU;
                                let (st, ct) = t.sin_cos();
                                let lx = (radius + 0.75) * ct;
                                let ly = (ry + 0.75) * st;
                                outer_pts.push(Pos2::new(
                                    screen_ptr.x + lx * cos_a - ly * sin_a,
                                    screen_ptr.y + lx * sin_a + ly * cos_a,
                                ));
                                let ix = radius * ct;
                                let iy = ry * st;
                                inner_pts.push(Pos2::new(
                                    screen_ptr.x + ix * cos_a - iy * sin_a,
                                    screen_ptr.y + ix * sin_a + iy * cos_a,
                                ));
                            }
                            canvas_painter.add(egui::Shape::closed_line(
                                outer_pts,
                                egui::Stroke::new(1.0_f32, Color32::from_black_alpha(150)),
                            ));
                            canvas_painter.add(egui::Shape::closed_line(
                                inner_pts,
                                egui::Stroke::new(1.0_f32, Color32::from_white_alpha(220)),
                            ));
                        } else {
                            canvas_painter.circle_stroke(
                                screen_ptr,
                                radius + 0.75,
                                egui::Stroke::new(1.0_f32, Color32::from_black_alpha(150)),
                            );
                            canvas_painter.circle_stroke(
                                screen_ptr,
                                radius,
                                egui::Stroke::new(1.0_f32, Color32::from_white_alpha(220)),
                            );
                        }
                        if radius > 5.0 {
                            canvas_painter.line_segment(
                                [
                                    screen_ptr - Vec2::new(2.5, 0.0),
                                    screen_ptr + Vec2::new(2.5, 0.0),
                                ],
                                egui::Stroke::new(1.0_f32, Color32::from_white_alpha(190)),
                            );
                            canvas_painter.line_segment(
                                [
                                    screen_ptr - Vec2::new(0.0, 2.5),
                                    screen_ptr + Vec2::new(0.0, 2.5),
                                ],
                                egui::Stroke::new(1.0_f32, Color32::from_white_alpha(190)),
                            );
                        }
                    }
                }
            }
        }

        // 5. Draw Top & Left Pixel Rulers + Right & Bottom Scrollbars on top of canvas border
        if self.show_rulers {
            self.draw_csp_rulers(&painter, outer_rect, rect, doc.width, doc.height, latest_ptr);
        }
        self.draw_csp_scrollbars(&painter, outer_rect, rect, doc.width, doc.height);

        (response, stroke_started)
    }

    fn draw_csp_rulers(
        &self,
        painter: &egui::Painter,
        outer_rect: Rect,
        canvas_rect: Rect,
        doc_w: u32,
        doc_h: u32,
        latest_ptr: Option<Pos2>,
    ) {
        let r_thick = 18.0;
        let top_ruler = Rect::from_min_max(
            Pos2::new(canvas_rect.left(), outer_rect.top()),
            Pos2::new(canvas_rect.right(), outer_rect.top() + r_thick),
        );
        let left_ruler = Rect::from_min_max(
            Pos2::new(outer_rect.left(), canvas_rect.top()),
            Pos2::new(outer_rect.left() + r_thick, canvas_rect.bottom()),
        );
        let corner_box = Rect::from_min_size(outer_rect.left_top(), Vec2::splat(r_thick));

        let ruler_bg = Color32::from_rgb(39, 41, 46);
        let border_col = Color32::from_rgb(24, 25, 28);
        let tick_col = Color32::from_rgb(168, 174, 184);

        painter.rect_filled(top_ruler, 0.0, ruler_bg);
        painter.rect_filled(left_ruler, 0.0, ruler_bg);
        painter.rect_filled(corner_box, 0.0, Color32::from_rgb(34, 36, 40));

        painter.line_segment(
            [top_ruler.left_bottom(), top_ruler.right_bottom()],
            Stroke::new(1.0_f32, border_col),
        );
        painter.line_segment(
            [left_ruler.right_top(), left_ruler.right_bottom()],
            Stroke::new(1.0_f32, border_col),
        );
        painter.text(
            corner_box.center(),
            Align2::CENTER_CENTER,
            "px",
            FontId::monospace(8.5),
            Color32::from_gray(145),
        );

        let half_w = doc_w as f32 * 0.5;
        let half_h = doc_h as f32 * 0.5;

        let major_step = if self.zoom < 0.25 {
            500
        } else if self.zoom < 0.7 {
            200
        } else if self.zoom < 1.8 {
            100
        } else {
            50
        };
        let minor_step = (major_step / 5).max(10);

        // Top Horizontal Ruler ticks
        let mut px = -400_i32;
        while px <= (doc_w as i32 + 400) {
            let sx = self.canvas_to_screen(Pos2::new(px as f32 - half_w, 0.0)).x;
            if sx >= top_ruler.left() && sx <= top_ruler.right() {
                let is_major = px % major_step == 0;
                let h = if is_major { 11.0 } else { 4.5 };
                painter.line_segment(
                    [
                        Pos2::new(sx, top_ruler.bottom() - h),
                        Pos2::new(sx, top_ruler.bottom()),
                    ],
                    Stroke::new(1.0_f32, tick_col),
                );
                if is_major {
                    painter.text(
                        Pos2::new(sx + 2.5, top_ruler.top() + 5.5),
                        Align2::LEFT_CENTER,
                        format!("{px}"),
                        FontId::monospace(8.5),
                        tick_col,
                    );
                }
            }
            px += minor_step;
        }

        // Left Vertical Ruler ticks
        let mut py = -400_i32;
        while py <= (doc_h as i32 + 400) {
            let sy = self.canvas_to_screen(Pos2::new(0.0, py as f32 - half_h)).y;
            if sy >= left_ruler.top() && sy <= left_ruler.bottom() {
                let is_major = py % major_step == 0;
                let w = if is_major { 11.0 } else { 4.5 };
                painter.line_segment(
                    [
                        Pos2::new(left_ruler.right() - w, sy),
                        Pos2::new(left_ruler.right(), sy),
                    ],
                    Stroke::new(1.0_f32, tick_col),
                );
                if is_major {
                    painter.text(
                        Pos2::new(left_ruler.left() + 1.5, sy + 5.0),
                        Align2::LEFT_CENTER,
                        format!("{py}"),
                        FontId::monospace(7.5),
                        tick_col,
                    );
                }
            }
            py += minor_step;
        }

        // Live cursor position hairline on both rulers
        if let Some(ptr) = latest_ptr {
            let cursor_col = Color32::from_rgb(55, 165, 255);
            if ptr.x >= top_ruler.left() && ptr.x <= top_ruler.right() {
                painter.line_segment(
                    [
                        Pos2::new(ptr.x, top_ruler.top()),
                        Pos2::new(ptr.x, top_ruler.bottom()),
                    ],
                    Stroke::new(1.5_f32, cursor_col),
                );
            }
            if ptr.y >= left_ruler.top() && ptr.y <= left_ruler.bottom() {
                painter.line_segment(
                    [
                        Pos2::new(left_ruler.left(), ptr.y),
                        Pos2::new(left_ruler.right(), ptr.y),
                    ],
                    Stroke::new(1.5_f32, cursor_col),
                );
            }
        }
    }

    fn draw_csp_scrollbars(
        &self,
        painter: &egui::Painter,
        outer_rect: Rect,
        canvas_rect: Rect,
        doc_w: u32,
        doc_h: u32,
    ) {
        let track_col = Color32::from_rgb(33, 35, 39);
        let thumb_col = Color32::from_rgb(68, 72, 80);

        let h_track = Rect::from_min_max(
            Pos2::new(canvas_rect.left(), canvas_rect.bottom()),
            Pos2::new(canvas_rect.right(), outer_rect.bottom()),
        );
        let v_track = Rect::from_min_max(
            Pos2::new(canvas_rect.right(), canvas_rect.top()),
            Pos2::new(outer_rect.right(), canvas_rect.bottom()),
        );
        let corner = Rect::from_min_max(canvas_rect.right_bottom(), outer_rect.right_bottom());

        painter.rect_filled(h_track, 0.0, track_col);
        painter.rect_filled(v_track, 0.0, track_col);
        painter.rect_filled(corner, 0.0, Color32::from_rgb(36, 38, 42));

        let view_w = (doc_w as f32 * self.zoom).max(100.0);
        let view_h = (doc_h as f32 * self.zoom).max(100.0);

        let h_ratio = (h_track.width() / (view_w * 2.0)).clamp(0.15, 0.85);
        let h_thumb_w = h_track.width() * h_ratio;
        let h_offset = ((self.pan.x - canvas_rect.center().x) / view_w).clamp(-0.4, 0.4);
        let h_center = h_track.center().x - h_offset * (h_track.width() - h_thumb_w);
        let h_thumb = Rect::from_center_size(
            Pos2::new(h_center, h_track.center().y),
            Vec2::new(h_thumb_w, h_track.height() - 3.0),
        );
        painter.rect_filled(h_thumb, 3.0, thumb_col);

        let v_ratio = (v_track.height() / (view_h * 2.0)).clamp(0.15, 0.85);
        let v_thumb_h = v_track.height() * v_ratio;
        let v_offset = ((self.pan.y - canvas_rect.center().y) / view_h).clamp(-0.4, 0.4);
        let v_center = v_track.center().y - v_offset * (v_track.height() - v_thumb_h);
        let v_thumb = Rect::from_center_size(
            Pos2::new(v_track.center().x, v_center),
            Vec2::new(v_track.width() - 3.0, v_thumb_h),
        );
        painter.rect_filled(v_thumb, 3.0, thumb_col);
    }
}

fn layer_signature(doc: &Document) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    doc.width.hash(&mut h);
    doc.height.hash(&mut h);
    doc.layers.len().hash(&mut h);
    for l in &doc.layers {
        l.id.hash(&mut h);
        l.visible.hash(&mut h);
        l.opacity.to_bits().hash(&mut h);
        std::mem::discriminant(&l.blend_mode).hash(&mut h);
        l.clipped_to_below.hash(&mut h);
    }
    h.finish()
}

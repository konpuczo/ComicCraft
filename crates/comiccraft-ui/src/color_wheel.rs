use eframe::egui::{self, Color32, Pos2, Rect, Sense, Shape, Stroke, Vec2};

pub struct ColorWheelWidget<'a> {
    pub primary_color: &'a mut [u8; 4],
    pub secondary_color: &'a mut [u8; 4],
    pub is_transparent: &'a mut bool,
    pub triangle_mode: &'a mut bool,
}

impl<'a> ColorWheelWidget<'a> {
    pub fn new(
        primary: &'a mut [u8; 4],
        secondary: &'a mut [u8; 4],
        is_transparent: &'a mut bool,
        triangle_mode: &'a mut bool,
    ) -> Self {
        Self {
            primary_color: primary,
            secondary_color: secondary,
            is_transparent,
            triangle_mode,
        }
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let avail_w = ui.available_width().clamp(160.0, 360.0);
        let size = Vec2::new(avail_w, 182.0);
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click_and_drag());

        let painter = ui.painter_at(rect);
        // Dark background box matching CSP Color Wheel well
        painter.rect_filled(rect, 2.0, Color32::from_rgb(35, 37, 41));
        painter.rect_stroke(rect, 2.0, Stroke::new(1.0_f32, Color32::from_rgb(54, 57, 63)));

        let center = Pos2::new(rect.center().x + 8.0, rect.center().y);
        let outer_radius = (rect.height() * 0.45).min(80.0);
        let inner_radius = outer_radius - 15.0;

        let (mut cur_h, mut cur_s, mut cur_v) = rgb_to_hsv(
            self.primary_color[0],
            self.primary_color[1],
            self.primary_color[2],
        );

        // 1. Draw Smooth 144-Segment Hue Ring
        let segments = 144;
        for i in 0..segments {
            let a1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let a2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

            let hue = i as f32 / segments as f32;
            let color = hsv_to_rgb(hue, 1.0, 1.0);

            let p1 = center + Vec2::new(a1.cos(), a1.sin()) * outer_radius;
            let p2 = center + Vec2::new(a2.cos(), a2.sin()) * outer_radius;
            let p3 = center + Vec2::new(a2.cos(), a2.sin()) * inner_radius;
            let p4 = center + Vec2::new(a1.cos(), a1.sin()) * inner_radius;

            painter.add(Shape::convex_polygon(
                vec![p1, p2, p3, p4],
                Color32::from_rgb(color[0], color[1], color[2]),
                Stroke::NONE,
            ));
        }
        painter.circle_stroke(center, outer_radius, Stroke::new(1.0, Color32::from_gray(25)));
        painter.circle_stroke(center, inner_radius, Stroke::new(1.0, Color32::from_gray(25)));

        // Hue ring indicator
        let hue_angle = cur_h * std::f32::consts::TAU;
        let mid_r = (outer_radius + inner_radius) * 0.5;
        let hue_marker = center + Vec2::new(hue_angle.cos(), hue_angle.sin()) * mid_r;
        painter.circle_stroke(hue_marker, 5.5, Stroke::new(2.0, Color32::WHITE));
        painter.circle_stroke(hue_marker, 6.5, Stroke::new(1.0, Color32::BLACK));

        // 2. Inner Saturation/Value Square (or HSV Triangle)
        let box_half = (inner_radius * std::f32::consts::FRAC_1_SQRT_2 - 2.0).floor();
        let sv_rect = Rect::from_min_max(center - Vec2::splat(box_half), center + Vec2::splat(box_half));

        if !*self.triangle_mode {
            let sv_res = 28;
            let cell_w = sv_rect.width() / sv_res as f32;
            let cell_h = sv_rect.height() / sv_res as f32;

            for y in 0..sv_res {
                for x in 0..sv_res {
                    let s = x as f32 / (sv_res - 1) as f32;
                    let v = 1.0 - (y as f32 / (sv_res - 1) as f32);
                    let c = hsv_to_rgb(cur_h, s, v);

                    let cell_rect = Rect::from_min_size(
                        Pos2::new(
                            sv_rect.min.x + x as f32 * cell_w,
                            sv_rect.min.y + y as f32 * cell_h,
                        ),
                        Vec2::new(cell_w + 0.6, cell_h + 0.6),
                    );
                    painter.rect_filled(cell_rect, 0.0, Color32::from_rgb(c[0], c[1], c[2]));
                }
            }
            painter.rect_stroke(sv_rect, 0.0, Stroke::new(1.0, Color32::from_gray(30)));

            let sv_pos = Pos2::new(
                sv_rect.min.x + cur_s * sv_rect.width(),
                sv_rect.min.y + (1.0 - cur_v) * sv_rect.height(),
            );
            painter.circle_stroke(sv_pos, 4.5, Stroke::new(1.8, Color32::WHITE));
            painter.circle_stroke(sv_pos, 5.5, Stroke::new(1.0, Color32::BLACK));
        } else {
            // CSP Triangle HSV mode
            let r_tri = inner_radius - 3.0;
            let p_hue = center + Vec2::new(r_tri, 0.0);
            let a1 = 2.0 * std::f32::consts::PI / 3.0;
            let a2 = 4.0 * std::f32::consts::PI / 3.0;
            let p_white = center + Vec2::new(a2.cos() * r_tri, a2.sin() * r_tri);
            let p_black = center + Vec2::new(a1.cos() * r_tri, a1.sin() * r_tri);

            let steps = 22;
            for i in 0..steps {
                for j in 0..(steps - i) {
                    let u = i as f32 / steps as f32;
                    let v = j as f32 / steps as f32;
                    let w = (1.0 - u - v).max(0.0);
                    let pt = Pos2::new(
                        u * p_hue.x + v * p_white.x + w * p_black.x,
                        u * p_hue.y + v * p_white.y + w * p_black.y,
                    );
                    let val = (u + v).clamp(0.0, 1.0);
                    let sat = if val > 1e-4 { (u / val).clamp(0.0, 1.0) } else { 0.0 };
                    let c = hsv_to_rgb(cur_h, sat, val);
                    painter.circle_filled(pt, 3.8, Color32::from_rgb(c[0], c[1], c[2]));
                }
            }
            painter.add(Shape::closed_line(
                vec![p_hue, p_white, p_black],
                Stroke::new(1.0, Color32::from_gray(30)),
            ));
        }

        // 3. CSP Left-Side Stacked Swatches (Main / Sub / Transparent) inside the Color Wheel well
        let p_box = Rect::from_min_size(rect.left_top() + Vec2::new(6.0, 8.0), Vec2::splat(20.0));
        let s_box = Rect::from_min_size(rect.left_top() + Vec2::new(16.0, 18.0), Vec2::splat(20.0));
        let t_box = Rect::from_min_size(rect.left_top() + Vec2::new(6.0, 42.0), Vec2::new(30.0, 11.0));

        let s_col = Color32::from_rgb(
            self.secondary_color[0],
            self.secondary_color[1],
            self.secondary_color[2],
        );
        painter.rect_filled(s_box, 1.0, s_col);
        painter.rect_stroke(s_box, 1.0, Stroke::new(1.0, Color32::from_gray(80)));

        let p_col = Color32::from_rgb(
            self.primary_color[0],
            self.primary_color[1],
            self.primary_color[2],
        );
        painter.rect_filled(p_box, 1.0, p_col);
        let p_border = if !*self.is_transparent {
            Color32::from_rgb(65, 145, 255)
        } else {
            Color32::from_gray(90)
        };
        painter.rect_stroke(p_box, 1.0, Stroke::new(1.5, p_border));

        // Checkerboard transparent swatch
        for cy in 0..2 {
            for cx in 0..6 {
                let cr = Rect::from_min_size(
                    t_box.min + Vec2::new(cx as f32 * 5.0, cy as f32 * 5.5),
                    Vec2::new(5.0, 5.5),
                );
                let fill = if (cx + cy) % 2 == 0 {
                    Color32::WHITE
                } else {
                    Color32::from_gray(175)
                };
                painter.rect_filled(cr, 0.0, fill);
            }
        }
        let t_border = if *self.is_transparent {
            Color32::from_rgb(65, 145, 255)
        } else {
            Color32::from_gray(85)
        };
        painter.rect_stroke(t_box, 1.0, Stroke::new(1.2, t_border));

        // 4. Bottom-left HSV/RGB compact readout inside wheel well
        painter.text(
            rect.left_bottom() + Vec2::new(6.0, -34.0),
            egui::Align2::LEFT_CENTER,
            format!("H:{:>3.0}", cur_h * 360.0),
            egui::FontId::monospace(9.5),
            Color32::from_gray(175),
        );
        painter.text(
            rect.left_bottom() + Vec2::new(6.0, -22.0),
            egui::Align2::LEFT_CENTER,
            format!("S:{:>3.0}", cur_s * 100.0),
            egui::FontId::monospace(9.5),
            Color32::from_gray(175),
        );
        painter.text(
            rect.left_bottom() + Vec2::new(6.0, -10.0),
            egui::Align2::LEFT_CENTER,
            format!("V:{:>3.0}", cur_v * 100.0),
            egui::FontId::monospace(9.5),
            Color32::from_gray(175),
        );

        // 5. Bottom-right Square/Triangle Mode Toggle Button (authentic CSP corner icon)
        let mode_rect = Rect::from_min_size(rect.right_bottom() - Vec2::new(22.0, 22.0), Vec2::splat(18.0));
        painter.rect_filled(mode_rect, 2.0, Color32::from_rgb(48, 51, 57));
        painter.rect_stroke(mode_rect, 2.0, Stroke::new(1.0, Color32::from_gray(90)));
        if *self.triangle_mode {
            painter.rect_stroke(mode_rect.shrink(4.5), 0.5, Stroke::new(1.2, Color32::WHITE));
        } else {
            let mc = mode_rect.center();
            painter.add(Shape::closed_line(
                vec![
                    mc + Vec2::new(0.0, -4.5),
                    mc + Vec2::new(4.5, 4.0),
                    mc + Vec2::new(-4.5, 4.0),
                ],
                Stroke::new(1.2, Color32::WHITE),
            ));
        }

        // Interaction Handling
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if mode_rect.contains(pos) {
                    *self.triangle_mode = !*self.triangle_mode;
                    return response;
                } else if t_box.contains(pos) {
                    *self.is_transparent = true;
                    return response;
                } else if p_box.contains(pos) {
                    *self.is_transparent = false;
                    return response;
                } else if s_box.contains(pos) {
                    std::mem::swap(self.primary_color, self.secondary_color);
                    *self.is_transparent = false;
                    return response;
                }
            }
        }

        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let offset = pointer_pos - center;
            let dist = offset.length();

            if dist >= inner_radius - 2.0 && dist <= outer_radius + 8.0 {
                let mut angle = offset.y.atan2(offset.x);
                if angle < 0.0 {
                    angle += std::f32::consts::TAU;
                }
                cur_h = angle / std::f32::consts::TAU;
                let rgb = hsv_to_rgb(cur_h, cur_s, cur_v);
                self.primary_color[0] = rgb[0];
                self.primary_color[1] = rgb[1];
                self.primary_color[2] = rgb[2];
                *self.is_transparent = false;
                response.mark_changed();
            } else if sv_rect.contains(pointer_pos) {
                cur_s = ((pointer_pos.x - sv_rect.min.x) / sv_rect.width()).clamp(0.0, 1.0);
                cur_v = (1.0 - ((pointer_pos.y - sv_rect.min.y) / sv_rect.height())).clamp(0.0, 1.0);
                let rgb = hsv_to_rgb(cur_h, cur_s, cur_v);
                self.primary_color[0] = rgb[0];
                self.primary_color[1] = rgb[1];
                self.primary_color[2] = rgb[2];
                *self.is_transparent = false;
                response.mark_changed();
            }
        }

        response
    }
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match (h * 6.0).floor() as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    ]
}

pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let d = max - min;

    let h = if d == 0.0 {
        0.0
    } else if max == rf {
        (((gf - bf) / d) % 6.0 + 6.0) % 6.0 / 6.0
    } else if max == gf {
        ((bf - rf) / d + 2.0) / 6.0
    } else {
        ((rf - gf) / d + 4.0) / 6.0
    };

    let s = if max == 0.0 { 0.0 } else { d / max };
    let v = max;

    (h, s, v)
}

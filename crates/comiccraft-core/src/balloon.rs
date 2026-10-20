use crate::layer::Layer;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BalloonShape {
    #[default]
    Ellipse,
    RoundedRect,
    Cloud,
    ShoutSpike,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpeechBalloon {
    pub center_x: f32,
    pub center_y: f32,
    pub radius_x: f32,
    pub radius_y: f32,
    pub shape: BalloonShape,
    pub tail_x: Option<f32>,
    pub tail_y: Option<f32>,
    pub fill_color: [u8; 4],
    pub border_color: [u8; 4],
    pub border_width: f32,
}

impl Default for SpeechBalloon {
    fn default() -> Self {
        Self {
            center_x: 400.0,
            center_y: 300.0,
            radius_x: 120.0,
            radius_y: 80.0,
            shape: BalloonShape::Ellipse,
            tail_x: Some(450.0),
            tail_y: Some(420.0),
            fill_color: [255, 255, 255, 255], // Opaque white interior
            border_color: [20, 20, 25, 255],  // Dark inking line
            border_width: 3.0,
        }
    }
}

impl SpeechBalloon {
    pub fn render_to_layer(&self, layer: &mut Layer) {
        let min_x = ((self.center_x - self.radius_x - self.border_width - 10.0) as i32).max(0);
        let max_x = ((self.center_x + self.radius_x + self.border_width + 10.0) as i32)
            .min(layer.width as i32 - 1);
        let min_y = ((self.center_y - self.radius_y - self.border_width - 10.0) as i32).max(0);
        let max_y = ((self.center_y + self.radius_y + self.border_width + 10.0) as i32)
            .min(layer.height as i32 - 1);

        let rx = self.radius_x;
        let ry = self.radius_y;
        let bw = self.border_width;

        // Render balloon body
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = x as f32 - self.center_x;
                let dy = y as f32 - self.center_y;

                let norm_dist = (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry);

                if norm_dist <= 1.0 {
                    // Inside ellipse
                    let edge_dist = 1.0 - norm_dist;
                    let border_threshold = (bw * 2.0) / (rx.min(ry));

                    if edge_dist <= border_threshold {
                        layer.set_pixel(x as u32, y as u32, self.border_color);
                    } else {
                        layer.set_pixel(x as u32, y as u32, self.fill_color);
                    }
                }
            }
        }

        // Render balloon tail if present
        if let (Some(tx), Some(ty)) = (self.tail_x, self.tail_y) {
            let tail_base_x1 = self.center_x + rx * 0.2;
            let tail_base_y1 = self.center_y + ry * 0.7;
            let tail_base_x2 = self.center_x + rx * 0.5;
            let tail_base_y2 = self.center_y + ry * 0.6;

            // Fill tail triangle
            draw_triangle(
                layer,
                [tail_base_x1, tail_base_y1],
                [tail_base_x2, tail_base_y2],
                [tx, ty],
                self.fill_color,
                self.border_color,
                self.border_width,
            );
        }
    }
}

fn draw_triangle(
    layer: &mut Layer,
    p1: [f32; 2],
    p2: [f32; 2],
    p3: [f32; 2],
    fill: [u8; 4],
    border: [u8; 4],
    border_width: f32,
) {
    let min_x = (p1[0].min(p2[0]).min(p3[0]) - border_width) as i32;
    let max_x = (p1[0].max(p2[0]).max(p3[0]) + border_width) as i32;
    let min_y = (p1[1].min(p2[1]).min(p3[1]) - border_width) as i32;
    let max_y = (p1[1].max(p2[1]).max(p3[1]) + border_width) as i32;

    let w = layer.width as i32;
    let h = layer.height as i32;

    for y in min_y.max(0)..=max_y.min(h - 1) {
        for x in min_x.max(0)..=max_x.min(w - 1) {
            let pt = [x as f32, y as f32];
            if point_in_triangle(pt, p1, p2, p3) {
                layer.set_pixel(x as u32, y as u32, fill);
            }
        }
    }

    // Draw borders along the sides p1->p3 and p2->p3
    draw_line(layer, p1, p3, border, border_width);
    draw_line(layer, p2, p3, border, border_width);
}

fn point_in_triangle(pt: [f32; 2], v1: [f32; 2], v2: [f32; 2], v3: [f32; 2]) -> bool {
    let d1 = sign(pt, v1, v2);
    let d2 = sign(pt, v2, v3);
    let d3 = sign(pt, v3, v1);

    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);

    !(has_neg && has_pos)
}

fn sign(p1: [f32; 2], p2: [f32; 2], p3: [f32; 2]) -> f32 {
    (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])
}

fn draw_line(layer: &mut Layer, p1: [f32; 2], p2: [f32; 2], color: [u8; 4], width: f32) {
    let dx = p2[0] - p1[0];
    let dy = p2[1] - p1[1];
    let dist = (dx * dx + dy * dy).sqrt();
    let steps = (dist * 2.0).ceil() as usize;
    let half_w = (width * 0.5).max(0.5);

    let lw = layer.width as i32;
    let lh = layer.height as i32;

    for i in 0..=steps {
        let t = i as f32 / steps.max(1) as f32;
        let cx = p1[0] + dx * t;
        let cy = p1[1] + dy * t;

        for oy in -(half_w as i32)..=(half_w as i32) {
            for ox in -(half_w as i32)..=(half_w as i32) {
                let px = (cx as i32) + ox;
                let py = (cy as i32) + oy;
                if px >= 0 && px < lw && py >= 0 && py < lh {
                    layer.set_pixel(px as u32, py as u32, color);
                }
            }
        }
    }
}

use crate::layer::Layer;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PanelRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl PanelRect {
    pub fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min_x: min_x.min(max_x),
            min_y: min_y.min(max_y),
            max_x: min_x.max(max_x),
            max_y: min_y.max(max_y),
        }
    }

    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComicFrameManager {
    pub border_width: f32, // Default 3.0 px
    pub gutter_h: f32,     // Horizontal gutter between panels (e.g. 20.0 px)
    pub gutter_v: f32,     // Vertical gutter between panels (e.g. 30.0 px)
    pub panels: Vec<PanelRect>,
}

impl Default for ComicFrameManager {
    fn default() -> Self {
        Self {
            border_width: 3.0,
            gutter_h: 20.0,
            gutter_v: 30.0,
            panels: Vec::new(),
        }
    }
}

impl ComicFrameManager {
    pub fn new(page_width: u32, page_height: u32, margin: f32) -> Self {
        let base_panel = PanelRect::new(
            margin,
            margin,
            page_width as f32 - margin,
            page_height as f32 - margin,
        );
        Self {
            border_width: 3.0,
            gutter_h: 20.0,
            gutter_v: 30.0,
            panels: vec![base_panel],
        }
    }

    /// Splits a panel horizontally into two panels with gutter spacing
    pub fn split_horizontal(&mut self, panel_idx: usize, split_y: f32) -> bool {
        if panel_idx >= self.panels.len() {
            return false;
        }
        let p = self.panels[panel_idx].clone();
        let half_gutter = self.gutter_v * 0.5;

        if split_y <= p.min_y + self.gutter_v || split_y >= p.max_y - self.gutter_v {
            return false;
        }

        let top = PanelRect::new(p.min_x, p.min_y, p.max_x, split_y - half_gutter);
        let bottom = PanelRect::new(p.min_x, split_y + half_gutter, p.max_x, p.max_y);

        self.panels[panel_idx] = top;
        self.panels.push(bottom);
        true
    }

    /// Splits a panel vertically into two panels with gutter spacing
    pub fn split_vertical(&mut self, panel_idx: usize, split_x: f32) -> bool {
        if panel_idx >= self.panels.len() {
            return false;
        }
        let p = self.panels[panel_idx].clone();
        let half_gutter = self.gutter_h * 0.5;

        if split_x <= p.min_x + self.gutter_h || split_x >= p.max_x - self.gutter_h {
            return false;
        }

        let left = PanelRect::new(p.min_x, p.min_y, split_x - half_gutter, p.max_y);
        let right = PanelRect::new(split_x + half_gutter, p.min_y, p.max_x, p.max_y);

        self.panels[panel_idx] = left;
        self.panels.push(right);
        true
    }

    /// Renders all panel borders onto a target layer
    pub fn render_to_layer(&self, layer: &mut Layer, border_color: [u8; 4]) {
        let half_b = (self.border_width * 0.5).max(0.5);
        let w = layer.width as i32;
        let h = layer.height as i32;

        for panel in &self.panels {
            let x0 = panel.min_x as i32;
            let y0 = panel.min_y as i32;
            let x1 = panel.max_x as i32;
            let y1 = panel.max_y as i32;

            // Draw top and bottom borders
            for x in (x0 - half_b as i32)..=(x1 + half_b as i32) {
                for dy in -(half_b as i32)..=(half_b as i32) {
                    let py_top = y0 + dy;
                    let py_bot = y1 + dy;
                    if x >= 0 && x < w {
                        if py_top >= 0 && py_top < h {
                            layer.set_pixel(x as u32, py_top as u32, border_color);
                        }
                        if py_bot >= 0 && py_bot < h {
                            layer.set_pixel(x as u32, py_bot as u32, border_color);
                        }
                    }
                }
            }

            // Draw left and right borders
            for y in (y0 - half_b as i32)..=(y1 + half_b as i32) {
                for dx in -(half_b as i32)..=(half_b as i32) {
                    let px_left = x0 + dx;
                    let px_right = x1 + dx;
                    if y >= 0 && y < h {
                        if px_left >= 0 && px_left < w {
                            layer.set_pixel(px_left as u32, y as u32, border_color);
                        }
                        if px_right >= 0 && px_right < w {
                            layer.set_pixel(px_right as u32, y as u32, border_color);
                        }
                    }
                }
            }
        }
    }
}

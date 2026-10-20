use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Selection {
    pub width: u32,
    pub height: u32,
    /// Bitmask or boolean array. When empty or None, entire canvas is active.
    #[serde(skip)]
    pub mask: Option<Vec<bool>>,
    pub bounds: Option<(u32, u32, u32, u32)>, // min_x, min_y, max_x, max_y
}


impl Selection {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            mask: None,
            bounds: None,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.mask = None;
        self.bounds = None;
    }

    pub fn is_empty(&self) -> bool {
        self.mask.is_none()
    }

    #[inline(always)]
    pub fn contains(&self, x: u32, y: u32) -> bool {
        match &self.mask {
            Some(mask) => {
                if x >= self.width || y >= self.height {
                    false
                } else {
                    mask[(y * self.width + x) as usize]
                }
            }
            None => true, // No selection means everything is editable
        }
    }

    pub fn deselect(&mut self) {
        self.mask = None;
        self.bounds = None;
    }

    pub fn select_all(&mut self) {
        let total = (self.width * self.height) as usize;
        self.mask = Some(vec![true; total]);
        self.bounds = Some((0, 0, self.width.saturating_sub(1), self.height.saturating_sub(1)));
    }

    pub fn invert(&mut self) {
        let total = (self.width * self.height) as usize;
        let new_mask = match self.mask.take() {
            Some(mut m) => {
                for b in &mut m {
                    *b = !*b;
                }
                m
            }
            None => vec![false; total],
        };


        // Recalculate bounds
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut has_selected = false;

        for y in 0..self.height {
            for x in 0..self.width {
                if new_mask[(y * self.width + x) as usize] {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    has_selected = true;
                }
            }
        }

        if has_selected {
            self.bounds = Some((min_x, min_y, max_x, max_y));
            self.mask = Some(new_mask);
        } else {
            self.mask = None;
            self.bounds = None;
        }
    }

    pub fn select_rect(&mut self, x1: u32, y1: u32, x2: u32, y2: u32) {
        let min_x = x1.min(x2).min(self.width.saturating_sub(1));
        let max_x = x1.max(x2).min(self.width.saturating_sub(1));
        let min_y = y1.min(y2).min(self.height.saturating_sub(1));
        let max_y = y1.max(y2).min(self.height.saturating_sub(1));

        let total = (self.width * self.height) as usize;
        let mut mask = vec![false; total];

        for y in min_y..=max_y {
            let row_start = (y * self.width) as usize;
            for x in min_x..=max_x {
                mask[row_start + x as usize] = true;
            }
        }

        self.mask = Some(mask);
        self.bounds = Some((min_x, min_y, max_x, max_y));
    }

    pub fn select_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, op: SelectionOp) {
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        let min_x = (cx - rx).max(0.0).floor() as u32;
        let max_x = (cx + rx).min(self.width.saturating_sub(1) as f32).ceil() as u32;
        let min_y = (cy - ry).max(0.0).floor() as u32;
        let max_y = (cy + ry).min(self.height.saturating_sub(1) as f32).ceil() as u32;

        let total = (self.width * self.height) as usize;
        let mut mask = vec![false; total];
        let rx_sq = rx * rx;
        let ry_sq = ry * ry;

        for y in min_y..=max_y {
            let fy = y as f32 + 0.5;
            let dy_sq = (fy - cy) * (fy - cy);
            let row_start = (y * self.width) as usize;
            for x in min_x..=max_x {
                let fx = x as f32 + 0.5;
                let dx_sq = (fx - cx) * (fx - cx);
                if (dx_sq / rx_sq) + (dy_sq / ry_sq) <= 1.0 {
                    mask[row_start + x as usize] = true;
                }
            }
        }
        self.combine(mask, op);
    }

    pub fn select_lasso(&mut self, poly: &[(f32, f32)]) {
        if poly.len() < 3 {
            return;
        }

        let mut p_min_x = poly[0].0;
        let mut p_max_x = poly[0].0;
        let mut p_min_y = poly[0].1;
        let mut p_max_y = poly[0].1;

        for pt in poly {
            p_min_x = p_min_x.min(pt.0);
            p_max_x = p_max_x.max(pt.0);
            p_min_y = p_min_y.min(pt.1);
            p_max_y = p_max_y.max(pt.1);
        }

        let min_x = (p_min_x.floor() as i32).max(0) as u32;
        let max_x = (p_max_x.ceil() as i32).min(self.width as i32 - 1).max(0) as u32;
        let min_y = (p_min_y.floor() as i32).max(0) as u32;
        let max_y = (p_max_y.ceil() as i32).min(self.height as i32 - 1).max(0) as u32;

        let total = (self.width * self.height) as usize;
        let mut mask = vec![false; total];
        let mut has_points = false;

        for y in min_y..=max_y {
            let fy = y as f32 + 0.5;
            let row_start = (y * self.width) as usize;
            for x in min_x..=max_x {
                let fx = x as f32 + 0.5;
                if point_in_polygon(fx, fy, poly) {
                    mask[row_start + x as usize] = true;
                    has_points = true;
                }
            }
        }

        if has_points {
            self.mask = Some(mask);
            self.bounds = Some((min_x, min_y, max_x, max_y));
        } else {
            self.mask = None;
            self.bounds = None;
        }
    }

    /// Returns the selection as a dense mask (`true` everywhere when nothing is selected).
    pub fn to_bool_mask(&self) -> Vec<bool> {
        match &self.mask {
            Some(m) => m.clone(),
            None => vec![true; (self.width * self.height) as usize],
        }
    }

    /// Combines `new_mask` with the current selection using `op` (CSP selection modes).
    pub fn combine(&mut self, new_mask: Vec<bool>, op: SelectionOp) {
        let total = (self.width * self.height) as usize;
        if new_mask.len() != total {
            return;
        }
        let merged = match (op, self.mask.take()) {
            (SelectionOp::Replace, _) | (_, None) if op != SelectionOp::Subtract && op != SelectionOp::Intersect => new_mask,
            (SelectionOp::Add, Some(mut cur)) => {
                for (c, n) in cur.iter_mut().zip(new_mask.iter()) {
                    *c |= *n;
                }
                cur
            }
            (SelectionOp::Subtract, cur) => {
                let mut cur = cur.unwrap_or_else(|| vec![false; total]);
                for (c, n) in cur.iter_mut().zip(new_mask.iter()) {
                    *c &= !*n;
                }
                cur
            }
            (SelectionOp::Intersect, cur) => {
                let mut cur = cur.unwrap_or_else(|| vec![false; total]);
                for (c, n) in cur.iter_mut().zip(new_mask.iter()) {
                    *c &= *n;
                }
                cur
            }
            (_, _) => new_mask,
        };
        self.mask = Some(merged);
        self.recompute_bounds();
    }

    /// Expands (+) or shrinks (-) the selection by `px` pixels (Selection > Expand / Shrink).
    pub fn grow(&mut self, px: i32) {
        if let Some(m) = &self.mask {
            let out = crate::fill::scale_mask(m, self.width as usize, self.height as usize, px);
            self.mask = Some(out);
            self.recompute_bounds();
        }
    }

    /// Computes a boolean mask representing a stroke along the perimeter of the selection (CSP Border Selected Area).
    pub fn compute_border_mask(&self, width_px: u32, position: SelectionBorderPosition) -> Option<Vec<bool>> {
        let mask = self.mask.as_ref()?;
        let w = self.width as usize;
        let h = self.height as usize;
        let radius = width_px.max(1) as i32;

        let border: Vec<bool> = match position {
            SelectionBorderPosition::Inside => {
                let shrunk = crate::fill::scale_mask(mask, w, h, -radius);
                mask.iter().zip(shrunk.iter()).map(|(&m, &s)| m && !s).collect()
            }
            SelectionBorderPosition::Outside => {
                let expanded = crate::fill::scale_mask(mask, w, h, radius);
                expanded.iter().zip(mask.iter()).map(|(&e, &m)| e && !m).collect()
            }
            SelectionBorderPosition::Centered => {
                let half_in = (radius / 2).max(1);
                let half_out = radius - half_in;
                let expanded = crate::fill::scale_mask(mask, w, h, half_out.max(1));
                let shrunk = crate::fill::scale_mask(mask, w, h, -half_in);
                expanded.iter().zip(shrunk.iter()).map(|(&e, &s)| e && !s).collect()
            }
        };
        Some(border)
    }

    /// Recomputes `bounds`; clears the selection if it became empty.
    pub fn recompute_bounds(&mut self) {
        let Some(m) = &self.mask else {
            self.bounds = None;
            return;
        };
        let (w, h) = (self.width, self.height);
        let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0u32, 0u32);
        let mut any = false;
        for y in 0..h {
            let row = (y * w) as usize;
            for x in 0..w {
                if m[row + x as usize] {
                    any = true;
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                    y0 = y0.min(y);
                    y1 = y1.max(y);
                }
            }
        }
        if any {
            self.bounds = Some((x0, y0, x1, y1));
        } else {
            self.mask = None;
            self.bounds = None;
        }
    }
}

/// How a new selection combines with the existing one (CSP: New / Add (Shift) / Subtract (Alt) / Intersect).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SelectionOp {
    #[default]
    Replace,
    Add,
    Subtract,
    Intersect,
}

/// Border positioning relative to selection perimeter (CSP: Inside, Centered, Outside).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SelectionBorderPosition {
    Inside,
    #[default]
    Centered,
    Outside,
}

fn point_in_polygon(x: f32, y: f32, poly: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        let intersect = ((yi > y) != (yj > y))
            && (x < (xj - xi) * (y - yi) / (yj - yi + 1e-7) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

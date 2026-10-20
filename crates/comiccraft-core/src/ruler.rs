//! Clip Studio Paint-style Symmetrical Ruler & Drawing Guides.
//!
//! Supports 2-way bilateral mirror drawing (characters, faces, weapons, vehicles)
//! and multi-way radial kaleidoscope symmetry (mandalas, magic circles, snowflakes).

use crate::stroke::StrokePoint;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SymmetryRuler {
    pub enabled: bool,
    pub center_x: f32,
    pub center_y: f32,
    /// Angle of the primary symmetry axis in degrees (0 = vertical axis, 90 = horizontal axis).
    pub angle_deg: f32,
    /// Number of symmetry sectors / lines (2 = bilateral mirror, 4, 6, 8 = radial kaleidoscope).
    pub lines: u32,
    /// If true, reflects across each axis line; if false, performs rotational repetition.
    pub line_symmetric: bool,
}

impl Default for SymmetryRuler {
    fn default() -> Self {
        Self {
            enabled: false,
            center_x: 800.0,
            center_y: 1200.0,
            angle_deg: 0.0,
            lines: 2,
            line_symmetric: true,
        }
    }
}

impl SymmetryRuler {
    pub fn new(cx: f32, cy: f32, lines: u32) -> Self {
        Self {
            enabled: true,
            center_x: cx,
            center_y: cy,
            angle_deg: 0.0,
            lines: lines.clamp(2, 16),
            line_symmetric: true,
        }
    }

    /// Computes all symmetrical reflected/rotated points for a stroke input point.
    /// The original point is NOT included in the returned list; callers render the original
    /// point first, then each reflected point in sequence.
    pub fn symmetrical_counterparts(&self, pt: StrokePoint) -> Vec<StrokePoint> {
        if !self.enabled || self.lines < 2 {
            return Vec::new();
        }

        let n = self.lines;
        let mut out = Vec::with_capacity((n.saturating_sub(1)) as usize);

        let dx = pt.x - self.center_x;
        let dy = pt.y - self.center_y;
        let base_rad = self.angle_deg.to_radians();

        // Rotate into ruler frame
        let cos0 = base_rad.cos();
        let sin0 = base_rad.sin();
        let lx = dx * cos0 + dy * sin0;
        let ly = -dx * sin0 + dy * cos0;

        let sector_rad = std::f32::consts::TAU / n as f32;

        if self.line_symmetric && n == 2 {
            // Fast bilateral mirror reflection across the ruler line (X -> -X)
            let rx = -lx;
            let ry = ly;
            let wx = self.center_x + (rx * cos0 - ry * sin0);
            let wy = self.center_y + (rx * sin0 + ry * cos0);
            out.push(StrokePoint::new(wx, wy, pt.pressure));
            return out;
        }

        for i in 1..n {
            let theta = i as f32 * sector_rad;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            let (rx, ry) = if self.line_symmetric && (i % 2 == 1) {
                // Alternating mirror flip
                (-lx * cos_t - ly * sin_t, -lx * sin_t + ly * cos_t)
            } else {
                // Pure rotation
                (lx * cos_t - ly * sin_t, lx * sin_t + ly * cos_t)
            };

            let wx = self.center_x + (rx * cos0 - ry * sin0);
            let wy = self.center_y + (rx * sin0 + ry * cos0);
            out.push(StrokePoint::new(wx, wy, pt.pressure));
        }

        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PerspectiveKind {
    OnePoint,
    #[default]
    TwoPoint,
    ThreePoint,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PerspectiveRuler {
    pub enabled: bool,
    pub kind: PerspectiveKind,
    pub eye_level_y: f32,
    pub vp1_x: f32,
    pub vp1_y: f32,
    pub vp2_x: f32,
    pub vp2_y: f32,
    pub vp3_x: f32,
    pub vp3_y: f32,
    pub snap_to_perspective: bool,
    pub guide_divisions: u32,
}

impl Default for PerspectiveRuler {
    fn default() -> Self {
        Self {
            enabled: false,
            kind: PerspectiveKind::TwoPoint,
            eye_level_y: 600.0,
            vp1_x: -300.0,
            vp1_y: 600.0,
            vp2_x: 1900.0,
            vp2_y: 600.0,
            vp3_x: 800.0,
            vp3_y: -1200.0,
            snap_to_perspective: true,
            guide_divisions: 8,
        }
    }
}

impl PerspectiveRuler {
    pub fn new_one_point(cx: f32, cy: f32) -> Self {
        Self {
            enabled: true,
            kind: PerspectiveKind::OnePoint,
            eye_level_y: cy,
            vp1_x: cx,
            vp1_y: cy,
            vp2_x: cx + 2000.0,
            vp2_y: cy,
            vp3_x: cx,
            vp3_y: -1000.0,
            snap_to_perspective: true,
            guide_divisions: 8,
        }
    }

    pub fn new_two_point(w: f32, h: f32) -> Self {
        let cy = h * 0.5;
        Self {
            enabled: true,
            kind: PerspectiveKind::TwoPoint,
            eye_level_y: cy,
            vp1_x: -w * 0.25,
            vp1_y: cy,
            vp2_x: w * 1.25,
            vp2_y: cy,
            vp3_x: w * 0.5,
            vp3_y: -h * 0.8,
            snap_to_perspective: true,
            guide_divisions: 8,
        }
    }

    /// Snaps a stroke point `pt` originating from `start` to the nearest perspective guideline.
    pub fn snap_point(&self, start: StrokePoint, pt: StrokePoint) -> StrokePoint {
        if !self.enabled || !self.snap_to_perspective {
            return pt;
        }

        let dx = pt.x - start.x;
        let dy = pt.y - start.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 2.0 {
            return pt;
        }

        let mut candidate_dirs: Vec<[f32; 2]> = Vec::with_capacity(4);

        // VP1 direction
        let v1x = self.vp1_x - start.x;
        let v1y = self.vp1_y - start.y;
        let l1 = (v1x * v1x + v1y * v1y).sqrt();
        if l1 > 1e-4 {
            candidate_dirs.push([v1x / l1, v1y / l1]);
        }

        match self.kind {
            PerspectiveKind::OnePoint => {
                candidate_dirs.push([1.0, 0.0]); // Horizontal
                candidate_dirs.push([0.0, 1.0]); // Vertical
            }
            PerspectiveKind::TwoPoint => {
                let v2x = self.vp2_x - start.x;
                let v2y = self.vp2_y - start.y;
                let l2 = (v2x * v2x + v2y * v2y).sqrt();
                if l2 > 1e-4 {
                    candidate_dirs.push([v2x / l2, v2y / l2]);
                }
                candidate_dirs.push([0.0, 1.0]); // Vertical
            }
            PerspectiveKind::ThreePoint => {
                let v2x = self.vp2_x - start.x;
                let v2y = self.vp2_y - start.y;
                let l2 = (v2x * v2x + v2y * v2y).sqrt();
                if l2 > 1e-4 {
                    candidate_dirs.push([v2x / l2, v2y / l2]);
                }
                let v3x = self.vp3_x - start.x;
                let v3y = self.vp3_y - start.y;
                let l3 = (v3x * v3x + v3y * v3y).sqrt();
                if l3 > 1e-4 {
                    candidate_dirs.push([v3x / l3, v3y / l3]);
                }
            }
        }

        if candidate_dirs.is_empty() {
            return pt;
        }

        // Find candidate unit vector with maximum absolute alignment
        let (nx, ny) = (dx / dist, dy / dist);
        let mut best_dir = candidate_dirs[0];
        let mut max_dot = -1.0_f32;

        for dir in candidate_dirs {
            let dot = (nx * dir[0] + ny * dir[1]).abs();
            if dot > max_dot {
                max_dot = dot;
                best_dir = dir;
            }
        }

        // Project displacement onto best_dir
        let proj = dx * best_dir[0] + dy * best_dir[1];
        let sx = start.x + proj * best_dir[0];
        let sy = start.y + proj * best_dir[1];

        StrokePoint::new(sx, sy, pt.pressure)
    }
}

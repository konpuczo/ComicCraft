#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
    pub pressure: f32, // 0.0 to 1.0
}

impl StrokePoint {
    pub fn new(x: f32, y: f32, pressure: f32) -> Self {
        Self {
            x,
            y,
            pressure: pressure.clamp(0.01, 1.0),
        }
    }

    pub fn lerp(a: StrokePoint, b: StrokePoint, t: f32) -> StrokePoint {
        StrokePoint {
            x: a.x + (b.x - a.x) * t,
            y: a.y + (b.y - a.y) * t,
            pressure: a.pressure + (b.pressure - a.pressure) * t,
        }
    }

    pub fn dist_to(&self, other: &StrokePoint) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Clip Studio Paint-style stroke stabilizer.
/// Smooths raw tablet / mouse input using weighted moving averages and interpolation.
#[derive(Clone, Debug)]
pub struct StrokeStabilizer {
    pub level: u32, // 0 to 50
    history: Vec<StrokePoint>,
}

impl StrokeStabilizer {
    pub fn new(level: u32) -> Self {
        Self {
            level: level.clamp(0, 50),
            history: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.history.clear();
    }

    pub fn push(&mut self, raw: StrokePoint) -> StrokePoint {
        if self.level == 0 {
            return raw;
        }

        self.history.push(raw);
        let max_samples = (self.level as usize).clamp(2, 25);
        if self.history.len() > max_samples {
            self.history.remove(0);
        }

        // Weighted moving average giving higher weight to recent points
        let mut total_weight = 0.0f32;
        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        let mut sum_pressure = 0.0f32;

        for (i, p) in self.history.iter().enumerate() {
            let weight = (i + 1) as f32;
            total_weight += weight;
            sum_x += p.x * weight;
            sum_y += p.y * weight;
            sum_pressure += p.pressure * weight;
        }

        StrokePoint {
            x: sum_x / total_weight,
            y: sum_y / total_weight,
            pressure: sum_pressure / total_weight,
        }
    }
}

/// Generates interpolated points along a stroke path for continuous stamping
pub fn interpolate_stroke(p1: StrokePoint, p2: StrokePoint, step_dist: f32) -> Vec<StrokePoint> {
    let dist = p1.dist_to(&p2);
    if dist < step_dist {
        return vec![p2];
    }

    let steps = (dist / step_dist).ceil() as usize;
    let mut result = Vec::with_capacity(steps);
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        result.push(StrokePoint::lerp(p1, p2, t));
    }
    result
}

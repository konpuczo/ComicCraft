use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
}

impl BlendMode {
    pub fn all() -> &'static [BlendMode] {
        &[
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::Darken,
            BlendMode::Lighten,
            BlendMode::ColorDodge,
            BlendMode::ColorBurn,
            BlendMode::HardLight,
            BlendMode::SoftLight,
            BlendMode::Difference,
            BlendMode::Exclusion,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::Darken => "Darken",
            BlendMode::Lighten => "Lighten",
            BlendMode::ColorDodge => "Color Dodge",
            BlendMode::ColorBurn => "Color Burn",
            BlendMode::HardLight => "Hard Light",
            BlendMode::SoftLight => "Soft Light",
            BlendMode::Difference => "Difference",
            BlendMode::Exclusion => "Exclusion",
        }
    }

    /// Blends top channel (s) over bottom channel (d), both in range [0.0, 1.0]
    #[inline(always)]
    pub fn blend_channel(&self, d: f32, s: f32) -> f32 {
        match self {
            BlendMode::Normal => s,
            BlendMode::Multiply => d * s,
            BlendMode::Screen => d + s - d * s,
            BlendMode::Overlay => {
                if d < 0.5 {
                    2.0 * d * s
                } else {
                    1.0 - 2.0 * (1.0 - d) * (1.0 - s)
                }
            }
            BlendMode::Darken => d.min(s),
            BlendMode::Lighten => d.max(s),
            BlendMode::ColorDodge => {
                if s >= 1.0 {
                    1.0
                } else {
                    (d / (1.0 - s)).min(1.0)
                }
            }
            BlendMode::ColorBurn => {
                if s <= 0.0 {
                    0.0
                } else {
                    (1.0 - (1.0 - d) / s).max(0.0)
                }
            }
            BlendMode::HardLight => {
                if s < 0.5 {
                    2.0 * d * s
                } else {
                    1.0 - 2.0 * (1.0 - d) * (1.0 - s)
                }
            }
            BlendMode::SoftLight => {
                if s < 0.5 {
                    d - (1.0 - 2.0 * s) * d * (1.0 - d)
                } else {
                    let g = if d <= 0.25 {
                        ((16.0 * d - 12.0) * d + 4.0) * d
                    } else {
                        d.sqrt()
                    };
                    d + (2.0 * s - 1.0) * (g - d)
                }
            }
            BlendMode::Difference => (d - s).abs(),
            BlendMode::Exclusion => d + s - 2.0 * d * s,
        }
    }

    /// Blends source [r, g, b, a] over destination [r, g, b, a] (0..255)
    #[inline]
    pub fn blend_pixel(&self, dst: [u8; 4], src: [u8; 4], layer_opacity: f32) -> [u8; 4] {
        let src_a = (src[3] as f32 / 255.0) * layer_opacity;
        if src_a <= 0.001 {
            return dst;
        }

        let dst_a = dst[3] as f32 / 255.0;
        let out_a = src_a + dst_a * (1.0 - src_a);
        if out_a <= 0.001 {
            return [0, 0, 0, 0];
        }

        let mut out = [0u8; 4];
        for i in 0..3 {
            let d = dst[i] as f32 / 255.0;
            let s = src[i] as f32 / 255.0;
            let blended = self.blend_channel(d, s);
            
            // Standard alpha compositing with blend mode
            let comp = (blended * src_a + d * dst_a * (1.0 - src_a)) / out_a;
            out[i] = (comp.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        out[3] = (out_a.clamp(0.0, 1.0) * 255.0).round() as u8;
        out
    }
}

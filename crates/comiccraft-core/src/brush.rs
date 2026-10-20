//! Clip Studio Paint-style brush engine.
//!
//! A stroke is rendered in three stages:
//! 1. Input points are smoothed into a Catmull-Rom curve (no polyline corners).
//! 2. Dabs are placed along the curve at a spacing relative to the current brush diameter.
//! 3. Each dab writes anti-aliased coverage into a per-stroke mask. The layer is then
//!    re-composited from a snapshot taken at stroke start, so overlapping dabs never build
//!    up opacity for pens (CSP "non-accumulating" ink), while airbrushes accumulate flow.
//!
//! Paint-mixing brushes (watercolor, thick paint) instead paint directly and pick up the
//! colour already on the layer.

use crate::layer::Layer;
use crate::selection::Selection;
use crate::stroke::StrokePoint;
use crate::tools::{GrayMap, SubTool, ToolConfig, ToolKind};
use serde::{Deserialize, Serialize};

/// Surface texture applied to a brush tip.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TipTexture {
    None,
    /// Paper grain (pencils, pastels, charcoal). `scale` in pixels, `strength` 0..1.
    Grain { scale: f32, strength: f32 },
    /// Manga halftone dots at 45° with the given pitch in pixels.
    Halftone { pitch: f32 },
}

/// How a brush deposits paint.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InkMode {
    /// Coverage is the max over the stroke: overlapping dabs never darken (pens, pencils).
    Stroke,
    /// Coverage accumulates with `flow` per dab (airbrush, pastel).
    Buildup { flow: f32 },
    /// Paint directly, mixing with the colour already on the layer (watercolor, thick paint).
    Mix { flow: f32, pickup: f32 },
    /// Non-accumulating eraser.
    Erase,
    /// Accumulating eraser (kneaded eraser).
    EraseBuildup { flow: f32 },
}

impl InkMode {
    fn is_erase(&self) -> bool {
        matches!(self, InkMode::Erase | InkMode::EraseBuildup { .. })
    }
    fn uses_mask(&self) -> bool {
        !matches!(self, InkMode::Mix { .. })
    }
}

/// All the per-sub-tool behaviour of a Clip Studio Paint brush.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrushPreset {
    /// Tip aspect ratio (1 = round, < 1 = flattened nib).
    pub aspect: f32,
    /// Tip angle in degrees (for flattened nibs).
    pub angle_deg: f32,
    /// Random rotation per dab (0 = fixed angle, 1 = full 360° random rotation).
    #[serde(default)]
    pub random_angle: f32,
    /// Watercolor / wet-pencil edge accentuation (0..1).
    #[serde(default)]
    pub water_edge: f32,
    pub texture: TipTexture,
    pub mode: InkMode,
    /// Dab spacing as a fraction of the current diameter.
    pub spacing: f32,
    /// Diameter at zero pressure, as a fraction of brush size (1 = no size pressure).
    pub size_min: f32,
    /// Opacity at zero pressure, as a fraction (1 = no opacity pressure).
    pub opacity_min: f32,
    /// Pressure curve exponent (> 1 = needs more pressure, < 1 = reaches full size quickly).
    pub pressure_gamma: f32,
    /// "Starting and ending" taper lengths, in brush diameters.
    pub taper_in: f32,
    pub taper_out: f32,
    /// Ragged / dry edges 0..1.
    pub roughness: f32,
    /// > 0 turns the tip into a droplet spray with this many droplets per dab.
    pub droplets: u32,
    pub default_size: f32,
    pub default_opacity: f32,
    pub default_hardness: f32,
    pub default_stabilization: u32,
}

impl BrushPreset {
    pub const BASE: BrushPreset = BrushPreset {
        aspect: 1.0,
        angle_deg: 0.0,
        random_angle: 0.0,
        water_edge: 0.0,
        texture: TipTexture::None,
        mode: InkMode::Stroke,
        spacing: 0.08,
        size_min: 1.0,
        opacity_min: 1.0,
        pressure_gamma: 1.0,
        taper_in: 0.0,
        taper_out: 0.0,
        roughness: 0.0,
        droplets: 0,
        default_size: 8.0,
        default_opacity: 1.0,
        default_hardness: 1.0,
        default_stabilization: 8,
    };

    /// Returns the preset for a tool + sub tool, matching CSP's default sub tools.
    pub fn for_tool(tool: ToolKind, sub: SubTool) -> BrushPreset {
        let b = Self::BASE;
        if tool == ToolKind::Screentone {
            return BrushPreset {
                texture: TipTexture::Halftone { pitch: 6.0 },
                default_size: 40.0,
                default_hardness: 1.0,
                default_stabilization: 4,
                ..b
            };
        }
        match sub {
            // ---------------- Pen ----------------
            SubTool::GPen => BrushPreset {
                size_min: 0.08,
                taper_in: 2.0,
                taper_out: 3.0,
                default_size: 8.0,
                default_stabilization: 10,
                ..b
            },
            SubTool::RealGPen => BrushPreset {
                size_min: 0.15,
                opacity_min: 0.55,
                taper_in: 1.5,
                taper_out: 2.5,
                roughness: 0.12,
                default_size: 9.0,
                default_stabilization: 10,
                ..b
            },
            SubTool::MappingPen => BrushPreset {
                size_min: 0.02,
                pressure_gamma: 1.6,
                taper_in: 3.0,
                taper_out: 4.5,
                default_size: 3.0,
                default_stabilization: 12,
                ..b
            },
            SubTool::KaburaPen => BrushPreset {
                size_min: 0.3,
                pressure_gamma: 0.7,
                taper_in: 1.2,
                taper_out: 1.8,
                default_size: 14.0,
                default_hardness: 0.85,
                default_stabilization: 8,
                ..b
            },
            SubTool::Calligraphy => BrushPreset {
                aspect: 0.22,
                angle_deg: -45.0,
                size_min: 0.7,
                spacing: 0.04,
                default_size: 18.0,
                default_stabilization: 10,
                ..b
            },
            SubTool::TexturedPen => BrushPreset {
                size_min: 0.25,
                texture: TipTexture::Grain { scale: 2.2, strength: 0.45 },
                taper_in: 1.0,
                taper_out: 1.5,
                default_size: 10.0,
                ..b
            },
            // ---------------- Marker ----------------
            SubTool::FeltPen => BrushPreset {
                size_min: 0.85,
                default_size: 12.0,
                default_opacity: 0.9,
                default_hardness: 0.8,
                default_stabilization: 4,
                ..b
            },
            SubTool::Millipen => BrushPreset {
                default_size: 4.0,
                default_stabilization: 6,
                ..b
            },
            SubTool::SignPen => BrushPreset {
                size_min: 0.75,
                pressure_gamma: 0.8,
                roughness: 0.08,
                default_size: 6.0,
                default_hardness: 0.9,
                default_stabilization: 5,
                ..b
            },
            // ---------------- Pencil ----------------
            SubTool::RealPencil => BrushPreset {
                texture: TipTexture::Grain { scale: 1.6, strength: 0.6 },
                size_min: 0.6,
                opacity_min: 0.2,
                pressure_gamma: 1.2,
                spacing: 0.05,
                default_size: 5.0,
                default_opacity: 0.9,
                default_hardness: 0.8,
                default_stabilization: 4,
                ..b
            },
            SubTool::DesignPencil => BrushPreset {
                texture: TipTexture::Grain { scale: 1.2, strength: 0.3 },
                size_min: 0.45,
                opacity_min: 0.4,
                default_size: 4.0,
                default_hardness: 0.95,
                default_stabilization: 6,
                ..b
            },
            SubTool::RoughPencil => BrushPreset {
                texture: TipTexture::Grain { scale: 1.8, strength: 0.5 },
                size_min: 0.55,
                opacity_min: 0.45,
                roughness: 0.3,
                default_size: 7.0,
                default_hardness: 0.75,
                default_stabilization: 3,
                ..b
            },
            SubTool::MechanicalPencil => BrushPreset {
                texture: TipTexture::Grain { scale: 1.0, strength: 0.35 },
                size_min: 0.9,
                opacity_min: 0.3,
                default_size: 2.5,
                default_opacity: 0.85,
                default_stabilization: 6,
                ..b
            },
            SubTool::ColoredPencil => BrushPreset {
                texture: TipTexture::Grain { scale: 2.0, strength: 0.65 },
                size_min: 0.7,
                opacity_min: 0.15,
                mode: InkMode::Buildup { flow: 0.55 },
                default_size: 8.0,
                default_opacity: 0.8,
                default_hardness: 0.7,
                default_stabilization: 3,
                ..b
            },
            // ---------------- Pastel ----------------
            SubTool::Pastel => BrushPreset {
                texture: TipTexture::Grain { scale: 3.0, strength: 0.75 },
                mode: InkMode::Buildup { flow: 0.35 },
                size_min: 0.8,
                opacity_min: 0.3,
                default_size: 30.0,
                default_hardness: 0.55,
                default_stabilization: 2,
                ..b
            },
            SubTool::Charcoal => BrushPreset {
                aspect: 0.6,
                angle_deg: 30.0,
                texture: TipTexture::Grain { scale: 4.0, strength: 0.85 },
                mode: InkMode::Buildup { flow: 0.45 },
                size_min: 0.8,
                opacity_min: 0.25,
                roughness: 0.25,
                default_size: 28.0,
                default_hardness: 0.6,
                default_stabilization: 2,
                ..b
            },
            SubTool::OilPastel => BrushPreset {
                texture: TipTexture::Grain { scale: 2.5, strength: 0.4 },
                mode: InkMode::Mix { flow: 0.85, pickup: 0.3 },
                size_min: 0.85,
                default_size: 24.0,
                default_hardness: 0.75,
                default_stabilization: 2,
                ..b
            },
            SubTool::Chalk => BrushPreset {
                texture: TipTexture::Grain { scale: 3.5, strength: 0.9 },
                mode: InkMode::Buildup { flow: 0.5 },
                roughness: 0.35,
                default_size: 20.0,
                default_hardness: 0.7,
                default_stabilization: 2,
                ..b
            },
            // ---------------- Brush ----------------
            SubTool::DenseWatercolor => BrushPreset {
                mode: InkMode::Mix { flow: 0.35, pickup: 0.55 },
                opacity_min: 0.3,
                size_min: 0.6,
                spacing: 0.1,
                default_size: 40.0,
                default_hardness: 0.35,
                default_stabilization: 3,
                ..b
            },
            SubTool::TransparentWatercolor => BrushPreset {
                mode: InkMode::Mix { flow: 0.18, pickup: 0.7 },
                opacity_min: 0.2,
                size_min: 0.7,
                spacing: 0.1,
                default_size: 60.0,
                default_opacity: 0.8,
                default_hardness: 0.15,
                default_stabilization: 3,
                ..b
            },
            SubTool::OpaqueWatercolor => BrushPreset {
                mode: InkMode::Mix { flow: 0.75, pickup: 0.35 },
                size_min: 0.6,
                spacing: 0.08,
                default_size: 36.0,
                default_hardness: 0.7,
                default_stabilization: 3,
                ..b
            },
            SubTool::OilPaint => BrushPreset {
                aspect: 0.7,
                angle_deg: 20.0,
                texture: TipTexture::Grain { scale: 1.4, strength: 0.25 },
                mode: InkMode::Mix { flow: 0.95, pickup: 0.45 },
                size_min: 0.7,
                spacing: 0.05,
                default_size: 40.0,
                default_hardness: 0.85,
                default_stabilization: 2,
                ..b
            },
            SubTool::Gouache => BrushPreset {
                mode: InkMode::Mix { flow: 0.9, pickup: 0.15 },
                size_min: 0.75,
                default_size: 30.0,
                default_hardness: 0.9,
                default_stabilization: 3,
                ..b
            },
            SubTool::IndiaInk => BrushPreset {
                aspect: 0.85,
                angle_deg: -30.0,
                size_min: 0.05,
                pressure_gamma: 1.2,
                taper_in: 2.0,
                taper_out: 3.5,
                roughness: 0.15,
                default_size: 20.0,
                default_stabilization: 6,
                ..b
            },
            SubTool::DryBrush => BrushPreset {
                aspect: 0.5,
                angle_deg: -20.0,
                size_min: 0.4,
                roughness: 0.6,
                texture: TipTexture::Grain { scale: 1.2, strength: 0.4 },
                taper_out: 2.0,
                default_size: 26.0,
                default_hardness: 0.9,
                default_stabilization: 4,
                ..b
            },
            // ---------------- Airbrush ----------------
            SubTool::AirbrushSoft => BrushPreset {
                mode: InkMode::Buildup { flow: 0.06 },
                opacity_min: 0.2,
                spacing: 0.06,
                default_size: 120.0,
                default_hardness: 0.0,
                default_stabilization: 0,
                ..b
            },
            SubTool::AirbrushHard => BrushPreset {
                mode: InkMode::Buildup { flow: 0.14 },
                opacity_min: 0.3,
                spacing: 0.06,
                default_size: 60.0,
                default_hardness: 0.65,
                default_stabilization: 0,
                ..b
            },
            SubTool::AirbrushSpray => BrushPreset {
                mode: InkMode::Buildup { flow: 0.9 },
                droplets: 18,
                spacing: 0.25,
                default_size: 80.0,
                default_hardness: 0.9,
                default_stabilization: 0,
                ..b
            },
            // ---------------- Eraser ----------------
            SubTool::HardEraser => BrushPreset {
                mode: InkMode::Erase,
                default_size: 30.0,
                default_stabilization: 2,
                ..b
            },
            SubTool::SoftEraser => BrushPreset {
                mode: InkMode::Erase,
                default_size: 60.0,
                default_hardness: 0.2,
                default_stabilization: 2,
                ..b
            },
            SubTool::KneadedEraser => BrushPreset {
                mode: InkMode::EraseBuildup { flow: 0.12 },
                opacity_min: 0.3,
                default_size: 50.0,
                default_hardness: 0.3,
                default_stabilization: 2,
                ..b
            },
            SubTool::VectorEraser => BrushPreset {
                mode: InkMode::Erase,
                default_size: 10.0,
                default_stabilization: 2,
                ..b
            },
            SubTool::RoughEraser => BrushPreset {
                mode: InkMode::Erase,
                texture: TipTexture::Grain { scale: 2.5, strength: 0.7 },
                default_size: 40.0,
                default_hardness: 0.8,
                default_stabilization: 2,
                ..b
            },
            SubTool::FocusLines
            | SubTool::StreamLines
            | SubTool::FigureRect
            | SubTool::FigureEllipse
            | SubTool::FigureLine
            | SubTool::SymmetryRuler
            | SubTool::LinearRuler
            | SubTool::PerspectiveRuler
            | SubTool::SelectionEllipseSub
            | SubTool::AutoSelectReferAll
            | SubTool::AutoSelectReferEditing
            | SubTool::CloseAndFillTool => BrushPreset {
                default_size: 4.0,
                default_hardness: 1.0,
                default_stabilization: 0,
                ..b
            },
            SubTool::Custom(_) => BrushPreset {
                texture: TipTexture::Grain { scale: 1.5, strength: 0.55 },
                size_min: 0.45,
                opacity_min: 0.25,
                default_size: 12.0,
                default_opacity: 0.95,
                default_hardness: 0.85,
                default_stabilization: 6,
                ..b
            },
        }
    }

    #[inline]
    fn size_factor(&self, p: f32) -> f32 {
        self.size_min + (1.0 - self.size_min) * p.clamp(0.0, 1.0).powf(self.pressure_gamma)
    }

    #[inline]
    fn opacity_factor(&self, p: f32) -> f32 {
        self.opacity_min + (1.0 - self.opacity_min) * p.clamp(0.0, 1.0).powf(self.pressure_gamma)
    }
}

// ---------------------------------------------------------------------------------------
// Procedural noise (deterministic in canvas space, so the paper grain never "swims").
// ---------------------------------------------------------------------------------------

#[inline]
fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    h
}

#[inline]
fn hashf(x: i32, y: i32, seed: u32) -> f32 {
    (hash(x, y, seed) & 0x00ff_ffff) as f32 / 16_777_215.0
}

#[inline]
fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let fx = x - xi;
    let fy = y - yi;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let (ix, iy) = (xi as i32, yi as i32);
    let a = hashf(ix, iy, seed);
    let b = hashf(ix + 1, iy, seed);
    let c = hashf(ix, iy + 1, seed);
    let d = hashf(ix + 1, iy + 1, seed);
    let top = a + (b - a) * sx;
    let bot = c + (d - c) * sx;
    top + (bot - top) * sy
}

/// Paper grain value in 0..1 at canvas pixel (x, y).
#[inline]
pub fn paper_grain(x: f32, y: f32, scale: f32) -> f32 {
    let s = scale.max(0.5);
    let n = 0.55 * value_noise(x / s, y / s, 11)
        + 0.30 * value_noise(x / (s * 0.47), y / (s * 0.47), 23)
        + 0.15 * hashf(x as i32, y as i32, 7);
    smoothstep(0.2, 0.85, n)
}

#[inline]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn xorshift(state: &mut u32) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x & 0x00ff_ffff) as f32 / 16_777_215.0
}

/// Anti-aliasing edge width in pixels for CSP's None / Weak / Mid / Strong setting.
#[inline]
pub fn aa_width(level: u8) -> f32 {
    match level {
        0 => 0.0,
        1 => 0.7,
        2 => 1.0,
        _ => 1.5,
    }
}

// ---------------------------------------------------------------------------------------
// Stroke state
// ---------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Dab {
    x: f32,
    y: f32,
    pressure: f32,
    dist: f32,
}

/// The state of an in-progress stroke.
pub struct StrokeState {
    pub layer_idx: usize,
    preset: BrushPreset,
    tip_stamp: Option<GrayMap>,
    paper_texture: Option<GrayMap>,
    color: [u8; 4],
    size: f32,
    opacity: f32,
    hardness: f32,
    aa: f32,
    w: usize,
    h: usize,
    /// Layer pixels when the stroke began (mask modes only).
    backup: Vec<u8>,
    /// Per-pixel stroke coverage 0..=65535 (mask modes only).
    mask: Vec<u16>,
    pts: Vec<StrokePoint>,
    dabs: Vec<Dab>,
    carry: f32,
    travelled: f32,
    last_sample: Option<StrokePoint>,
    /// Region touched since the last apply, [x0, y0, x1, y1) in pixels.
    pending: Option<[i32; 4]>,
    /// Region touched over the whole stroke.
    total: Option<[i32; 4]>,
    rng: u32,
    /// Colour currently loaded on a mixing brush (straight RGB 0..1).
    loaded: [f32; 3],
}

/// Holder for the active stroke that is never cloned into undo history.
#[derive(Default)]
pub struct StrokeSlot(pub Option<Box<StrokeState>>);

impl Clone for StrokeSlot {
    fn clone(&self) -> Self {
        StrokeSlot(None)
    }
}

impl std::fmt::Debug for StrokeSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StrokeSlot(active={})", self.0.is_some())
    }
}

/// Holder for mirror strokes used in symmetrical drawing.
#[derive(Default)]
pub struct MirrorStrokesSlot(pub Vec<Box<StrokeState>>);

impl Clone for MirrorStrokesSlot {
    fn clone(&self) -> Self {
        MirrorStrokesSlot(Vec::new())
    }
}

impl std::fmt::Debug for MirrorStrokesSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MirrorStrokesSlot(count={})", self.0.len())
    }
}


#[inline]
fn union(a: Option<[i32; 4]>, b: [i32; 4]) -> Option<[i32; 4]> {
    Some(match a {
        Some(a) => [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])],
        None => b,
    })
}

impl StrokeState {
    pub fn new(layer: &Layer, layer_idx: usize, cfg: &ToolConfig) -> Self {
        let preset = cfg.active_preset();
        let (tip_stamp, paper_texture) = if let SubTool::Custom(id) = cfg.sub_tool {
            if let Some(c) = cfg.custom_sub_tools.iter().find(|c| c.id == id) {
                (c.tip_stamp.clone(), c.paper_texture.clone())
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        let w = layer.width as usize;
        let h = layer.height as usize;
        let (backup, mask) = if preset.mode.uses_mask() {
            (layer.pixels.clone(), vec![0u16; w * h])
        } else {
            (Vec::new(), Vec::new())
        };
        let c = cfg.primary_color;
        StrokeState {
            layer_idx,
            preset,
            tip_stamp,
            paper_texture,
            color: c,
            size: cfg.size.max(0.3),
            opacity: cfg.opacity.clamp(0.0, 1.0),
            hardness: cfg.hardness.clamp(0.0, 1.0),
            aa: aa_width(cfg.antialiasing),
            w,
            h,
            backup,
            mask,
            pts: Vec::new(),
            dabs: Vec::new(),
            carry: 0.0,
            travelled: 0.0,
            last_sample: None,
            pending: None,
            total: None,
            rng: 0x9e37_79b9 ^ (cfg.size.to_bits()),
            loaded: [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0],
        }
    }

    /// Adds an input point. Returns the dirty pixel region written to the layer.
    pub fn push(&mut self, pt: StrokePoint, layer: &mut Layer, sel: &Selection) -> Option<[u32; 4]> {
        if let Some(last) = self.pts.last() {
            if last.dist_to(&pt) < 0.25 {
                return None;
            }
        }
        self.pts.push(pt);
        let n = self.pts.len();
        if n == 1 {
            self.place_dab(pt, layer, sel);
            self.last_sample = Some(pt);
        } else if n >= 3 {
            // Render the curve segment pts[n-3] -> pts[n-2] (one point of lag for tangents).
            let p0 = self.pts[n.saturating_sub(4).max(0)];
            let p0 = if n >= 4 { p0 } else { self.pts[n - 3] };
            let (p1, p2, p3) = (self.pts[n - 3], self.pts[n - 2], self.pts[n - 1]);
            self.render_curve(p0, p1, p2, p3, layer, sel);
        }
        self.apply(layer, sel)
    }

    /// Finishes the stroke: renders the final segment and applies end taper.
    pub fn finish(&mut self, layer: &mut Layer, sel: &Selection) -> Option<[u32; 4]> {
        let n = self.pts.len();
        if n >= 2 {
            let p0 = if n >= 3 { self.pts[n - 3] } else { self.pts[n - 2] };
            let (p1, p2) = (self.pts[n - 2], self.pts[n - 1]);
            self.render_curve(p0, p1, p2, p2, layer, sel);
        }

        let tapered = (self.preset.taper_in > 0.0 || self.preset.taper_out > 0.0)
            && self.preset.mode.uses_mask()
            && !self.dabs.is_empty();
        if tapered {
            // Re-render the whole stroke with the final "Starting and ending" taper.
            if let Some(t) = self.total {
                self.clear_mask(t);
            }
            let total_len = self.travelled;
            let dabs = std::mem::take(&mut self.dabs);
            for d in &dabs {
                let f = self.taper_factor(d.dist, Some(total_len));
                self.render_dab(d.x, d.y, d.pressure, f, layer, sel);
            }
            self.dabs = dabs;
            self.pending = self.total;
        }
        self.apply(layer, sel)
    }

    fn clear_mask(&mut self, r: [i32; 4]) {
        for y in r[1].max(0) as usize..(r[3].max(0) as usize).min(self.h) {
            let row = y * self.w;
            let x0 = r[0].max(0) as usize;
            let x1 = (r[2].max(0) as usize).min(self.w);
            if x1 > x0 {
                self.mask[row + x0..row + x1].fill(0);
            }
        }
    }

    /// Taper multiplier for a dab at arc length `dist`. When `total` is known (stroke end),
    /// both taper lengths are clamped so short strokes and dots still look right.
    fn taper_factor(&self, dist: f32, total: Option<f32>) -> f32 {
        let diam = self.size;
        let mut f = 1.0f32;
        let (mut tin, mut tout) = (self.preset.taper_in * diam, self.preset.taper_out * diam);
        if let Some(total) = total {
            if total < 1.0 {
                return 1.0; // a single click: no taper
            }
            tin = tin.min(total * 0.4);
            tout = tout.min(total * 0.45);
            if tout > 0.0 {
                let t = ((total - dist) / tout).clamp(0.0, 1.0);
                f = f.min(t.powf(0.75));
            }
        }
        if tin > 0.0 {
            let t = (dist / tin).clamp(0.0, 1.0);
            f = f.min(t.powf(0.75));
        }
        f.max(0.06)
    }

    fn render_curve(
        &mut self,
        p0: StrokePoint,
        p1: StrokePoint,
        p2: StrokePoint,
        p3: StrokePoint,
        layer: &mut Layer,
        sel: &Selection,
    ) {
        let chord = p1.dist_to(&p2);
        let steps = (chord / 0.75).ceil().max(1.0) as usize;
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let s = catmull_rom(p0, p1, p2, p3, t);
            self.walk_to(s, layer, sel);
        }
    }

    /// Advances along a straight micro-segment, placing dabs at the brush spacing.
    fn walk_to(&mut self, s: StrokePoint, layer: &mut Layer, sel: &Selection) {
        let Some(mut prev) = self.last_sample else {
            self.last_sample = Some(s);
            return;
        };
        let mut seg = prev.dist_to(&s);
        while seg > 1e-4 {
            let diam = (self.size * self.preset.size_factor(prev.pressure)).max(0.6);
            let spacing = (diam * self.preset.spacing).clamp(0.35, 64.0);
            let need = spacing - self.carry;
            if seg >= need {
                let t = need / seg;
                let d = StrokePoint::lerp(prev, s, t);
                self.travelled += need;
                self.carry = 0.0;
                self.place_dab(d, layer, sel);
                prev = d;
                seg -= need;
            } else {
                self.carry += seg;
                self.travelled += seg;
                break;
            }
        }
        self.last_sample = Some(s);
    }

    fn place_dab(&mut self, p: StrokePoint, layer: &mut Layer, sel: &Selection) {
        let dist = self.travelled;
        self.dabs.push(Dab { x: p.x, y: p.y, pressure: p.pressure, dist });
        let f = self.taper_factor(dist, None);
        self.render_dab(p.x, p.y, p.pressure, f, layer, sel);
    }

    /// Renders one dab into the mask (or directly onto the layer for mixing brushes).
    fn render_dab(&mut self, cx: f32, cy: f32, pressure: f32, taper: f32, layer: &mut Layer, sel: &Selection) {
        let pr = self.preset;
        let radius = 0.5 * self.size * pr.size_factor(pressure) * taper;
        let alpha = pr.opacity_factor(pressure);

        if pr.droplets > 0 {
            let r = radius.max(1.0);
            for _ in 0..pr.droplets {
                let a = xorshift(&mut self.rng) * std::f32::consts::TAU;
                let d = xorshift(&mut self.rng).sqrt() * r;
                let dr = (r * 0.045 * (0.5 + xorshift(&mut self.rng))).max(0.6);
                let fade = 1.0 - (d / r).powi(2) * 0.6;
                self.stamp(cx + a.cos() * d, cy + a.sin() * d, dr, 1.0, 0.0, 0.9, alpha * fade, layer, sel);
            }
            return;
        }

        let mut angle_deg = pr.angle_deg;
        if pr.random_angle > 0.0 {
            angle_deg += (xorshift(&mut self.rng) - 0.5) * 360.0 * pr.random_angle;
        }
        let (sin, cos) = angle_deg.to_radians().sin_cos();
        self.stamp(cx, cy, radius, pr.aspect, sin, cos, alpha, layer, sel);
    }

    #[allow(clippy::too_many_arguments)]
    fn stamp(
        &mut self,
        cx: f32,
        cy: f32,
        radius: f32,
        aspect: f32,
        sin: f32,
        cos: f32,
        alpha: f32,
        layer: &mut Layer,
        sel: &Selection,
    ) {
        let pr = self.preset;
        // Sub-pixel tips: keep a minimum footprint and fade intensity by area instead.
        let (r, intensity) = if radius < 0.6 { (0.6, (radius / 0.6).powi(2)) } else { (radius, 1.0) };
        let alpha = (alpha * intensity).clamp(0.0, 1.0);
        if alpha <= 0.0005 {
            return;
        }
        let aspect = aspect.clamp(0.05, 1.0);
        let hardness = if pr.droplets > 0 { 0.9 } else { self.hardness };
        let aa = self.aa;
        let reach = r + aa + 1.0;
        let x0 = ((cx - reach).floor() as i32).max(0);
        let y0 = ((cy - reach).floor() as i32).max(0);
        let x1 = ((cx + reach).ceil() as i32).min(self.w as i32 - 1);
        let y1 = ((cy + reach).ceil() as i32).min(self.h as i32 - 1);
        if x1 < x0 || y1 < y0 {
            return;
        }
        let rect = [x0, y0, x1 + 1, y1 + 1];
        self.pending = union(self.pending, rect);
        self.total = union(self.total, rect);

        let rough = pr.roughness;
        let texture = pr.texture;

        // Mixing brushes: pick up colour under the dab, then paint directly.
        if let InkMode::Mix { flow, pickup } = pr.mode {
            let (sr, sg, sb, sa) = sample_average(layer, cx, cy, r * 0.6);
            if sa > 0.01 {
                let k = pickup * sa;
                for i in 0..3 {
                    let s = [sr, sg, sb][i];
                    self.loaded[i] = self.loaded[i] * (1.0 - k) + s * k;
                }
            }
            // Gradually reload the original colour.
            let base = [
                self.color[0] as f32 / 255.0,
                self.color[1] as f32 / 255.0,
                self.color[2] as f32 / 255.0,
            ];
            for i in 0..3 {
                self.loaded[i] = self.loaded[i] * 0.85 + base[i] * 0.15;
            }
            let col = [
                (self.loaded[0] * 255.0).round() as u8,
                (self.loaded[1] * 255.0).round() as u8,
                (self.loaded[2] * 255.0).round() as u8,
            ];
            let a_scale = alpha * flow * self.opacity;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if !sel.contains(x as u32, y as u32) {
                        continue;
                    }
                    let dx = x as f32 + 0.5 - cx;
                    let dy = y as f32 + 0.5 - cy;
                    let mut c = sample_tip(self.tip_stamp.as_ref(), dx, dy, r, aspect, sin, cos, hardness, aa);
                    if c <= 0.0 {
                        continue;
                    }
                    c *= sample_paper_or_proc(self.paper_texture.as_ref(), texture, x as f32, y as f32, c);
                    if rough > 0.0 {
                        c *= rough_factor(rough, x as f32, y as f32);
                    }
                    let a = c * a_scale;
                    if a > 0.0005 {
                        let idx = (y as usize * self.w + x as usize) * 4;
                        let dst = [
                            layer.pixels[idx],
                            layer.pixels[idx + 1],
                            layer.pixels[idx + 2],
                            layer.pixels[idx + 3],
                        ];
                        let out = paint_over(dst, col, a, layer.alpha_locked);
                        layer.pixels[idx..idx + 4].copy_from_slice(&out);
                    }
                }
            }
            return;
        }

        let buildup = match pr.mode {
            InkMode::Buildup { flow } | InkMode::EraseBuildup { flow } => Some(flow),
            _ => None,
        };
        for y in y0..=y1 {
            let row = y as usize * self.w;
            for x in x0..=x1 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let mut c = sample_tip(self.tip_stamp.as_ref(), dx, dy, r, aspect, sin, cos, hardness, aa);
                if c <= 0.0 {
                    continue;
                }
                c *= sample_paper_or_proc(self.paper_texture.as_ref(), texture, x as f32, y as f32, c);
                if rough > 0.0 {
                    c *= rough_factor(rough, x as f32, y as f32);
                }
                let a = (c * alpha).clamp(0.0, 1.0);
                if a <= 0.0 {
                    continue;
                }
                let m = &mut self.mask[row + x as usize];
                let cur = *m as f32 / 65535.0;
                let next = match buildup {
                    Some(flow) => cur + (1.0 - cur) * a * flow,
                    None => cur.max(a),
                };
                *m = (next.clamp(0.0, 1.0) * 65535.0).round() as u16;
            }
        }
    }

    /// Composites the pending mask region from the snapshot onto the layer.
    fn apply(&mut self, layer: &mut Layer, sel: &Selection) -> Option<[u32; 4]> {
        let r = self.pending.take()?;
        let r = [r[0].max(0) as u32, r[1].max(0) as u32, r[2].max(0) as u32, r[3].max(0) as u32];
        if !self.preset.mode.uses_mask() {
            return Some(r);
        }
        let erase = self.preset.mode.is_erase();
        let col = [self.color[0], self.color[1], self.color[2]];
        let op = self.opacity;
        let water_edge = self.preset.water_edge;
        let alpha_locked = layer.alpha_locked;
        for y in r[1] as usize..(r[3] as usize).min(self.h) {
            let row = y * self.w;
            for x in r[0] as usize..(r[2] as usize).min(self.w) {
                let i = row + x;
                let idx = i * 4;
                let base = [self.backup[idx], self.backup[idx + 1], self.backup[idx + 2], self.backup[idx + 3]];
                let m = self.mask[i];
                let out = if m == 0 || !sel.contains(x as u32, y as u32) {
                    base
                } else {
                    let mut cov = m as f32 / 65535.0;
                    if water_edge > 0.0 && cov > 0.03 && cov < 0.92 {
                        let rim = 4.0 * cov * (1.0 - cov);
                        cov = (cov + water_edge * rim * 0.45).clamp(0.0, 1.0);
                    }
                    let a = cov * op;
                    if erase {
                        if alpha_locked {
                            base
                        } else {
                            let na = (base[3] as f32 * (1.0 - a)).round() as u8;
                            [base[0], base[1], base[2], na]
                        }
                    } else {
                        paint_over(base, col, a, alpha_locked)
                    }
                };
                layer.pixels[idx..idx + 4].copy_from_slice(&out);
            }
        }
        Some(r)
    }
}

/// Uniform Catmull-Rom interpolation between p1 and p2.
fn catmull_rom(p0: StrokePoint, p1: StrokePoint, p2: StrokePoint, p3: StrokePoint, t: f32) -> StrokePoint {
    let t2 = t * t;
    let t3 = t2 * t;
    let f = |a: f32, b: f32, c: f32, d: f32| {
        0.5 * ((2.0 * b) + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (-a + 3.0 * b - 3.0 * c + d) * t3)
    };
    StrokePoint {
        x: f(p0.x, p1.x, p2.x, p3.x),
        y: f(p0.y, p1.y, p2.y, p3.y),
        pressure: (p1.pressure + (p2.pressure - p1.pressure) * t).clamp(0.01, 1.0),
    }
}

/// Samples either a custom bitmap brush tip (`.sut` `BrushPattern` or imported `.png`)
/// or the analytical elliptical brush tip.
#[inline]
#[allow(clippy::too_many_arguments)]
fn sample_tip(
    tip: Option<&GrayMap>,
    dx: f32,
    dy: f32,
    r: f32,
    aspect: f32,
    sin: f32,
    cos: f32,
    hardness: f32,
    aa: f32,
) -> f32 {
    if let Some(tmap) = tip {
        let inv_r = 1.0 / r.max(0.5);
        let u = (dx * cos + dy * sin) * inv_r;
        let v = ((-dx * sin + dy * cos) / aspect) * inv_r;
        let tu = u * 0.5 + 0.5;
        let tv = v * 0.5 + 0.5;
        let s = tmap.sample_bilinear(tu, tv);
        if s <= 0.0 {
            return 0.0;
        }
        if hardness < 0.98 {
            let d = (u * u + v * v).sqrt().clamp(0.0, 1.0);
            let falloff = if d <= hardness {
                1.0
            } else {
                let x = (d - hardness) / (1.0 - hardness).max(1e-3);
                1.0 - x * x * (3.0 - 2.0 * x)
            };
            return s * falloff;
        }
        return s;
    }
    tip_coverage(dx, dy, r, aspect, sin, cos, hardness, aa)
}

/// Samples either a custom tileable paper texture (`.sut` `PaperTexture` or imported `.png`)
/// or the procedural paper grain / halftone screen.
#[inline]
fn sample_paper_or_proc(
    paper: Option<&GrayMap>,
    tex: TipTexture,
    x: f32,
    y: f32,
    coverage: f32,
) -> f32 {
    if let Some(pmap) = paper {
        let (scale, strength) = match tex {
            TipTexture::Grain { scale, strength } => (scale, strength),
            _ => (1.0, 0.65),
        };
        let g = pmap.sample_tiled(x, y, scale);
        return (1.0 - strength) + strength * g;
    }
    texture_factor(tex, x, y, coverage)
}

/// Anti-aliased coverage of an (optionally elliptical, rotated) tip at offset (dx, dy).
#[inline]
#[allow(clippy::too_many_arguments)]
fn tip_coverage(dx: f32, dy: f32, r: f32, aspect: f32, sin: f32, cos: f32, hardness: f32, aa: f32) -> f32 {
    let (d, scale) = if aspect >= 0.999 {
        ((dx * dx + dy * dy).sqrt(), 1.0)
    } else {
        let u = dx * cos + dy * sin;
        let v = (-dx * sin + dy * cos) / aspect;
        let d = (u * u + v * v).sqrt();
        let real = (dx * dx + dy * dy).sqrt();
        (d, if d > 1e-4 { real / d } else { aspect })
    };
    let edge_px = (r - d) * scale;
    let edge = if aa <= 0.0 {
        if edge_px >= 0.0 { 1.0 } else { 0.0 }
    } else {
        (edge_px / aa + 0.5).clamp(0.0, 1.0)
    };
    if edge <= 0.0 {
        return 0.0;
    }
    if hardness >= 0.999 {
        return edge;
    }
    let t = (d / r.max(1e-3)).clamp(0.0, 1.0);
    let soft = if hardness <= 0.001 {
        // Gaussian-like airbrush falloff.
        let q = 1.0 - t * t;
        q * q
    } else if t <= hardness {
        1.0
    } else {
        let x = (t - hardness) / (1.0 - hardness);
        1.0 - x * x * (3.0 - 2.0 * x)
    };
    edge * soft
}

#[inline]
fn texture_factor(tex: TipTexture, x: f32, y: f32, coverage: f32) -> f32 {
    match tex {
        TipTexture::None => 1.0,
        TipTexture::Grain { scale, strength } => {
            let g = paper_grain(x, y, scale);
            (1.0 - strength) + strength * g
        }
        TipTexture::Halftone { pitch } => {
            // 45° screen: rotate, take distance to cell centre, grow dots with coverage.
            let k = std::f32::consts::FRAC_1_SQRT_2;
            let u = (x + y) * k / pitch;
            let v = (y - x) * k / pitch;
            let fu = u - u.floor() - 0.5;
            let fv = v - v.floor() - 0.5;
            let dcell = (fu * fu + fv * fv).sqrt();
            let dot_r = coverage.sqrt() * 0.56;
            (((dot_r - dcell) * pitch + 0.5).clamp(0.0, 1.0)) / coverage.max(1e-3)
        }
    }
}

#[inline]
fn rough_factor(rough: f32, x: f32, y: f32) -> f32 {
    let n = 0.6 * value_noise(x * 0.45, y * 0.45, 91) + 0.4 * hashf(x as i32, y as i32, 93);
    smoothstep(rough * 0.7, rough * 0.7 + 0.25, n)
}

/// Straight-alpha "source over" of a solid colour with alpha `a` onto `dst`.
#[inline]
pub fn paint_over(dst: [u8; 4], col: [u8; 3], a: f32, alpha_locked: bool) -> [u8; 4] {
    let a = a.clamp(0.0, 1.0);
    let da = dst[3] as f32 / 255.0;
    if alpha_locked {
        if dst[3] == 0 {
            return dst;
        }
        let mut out = dst;
        for i in 0..3 {
            out[i] = (dst[i] as f32 + (col[i] as f32 - dst[i] as f32) * a).round() as u8;
        }
        return out;
    }
    let oa = a + da * (1.0 - a);
    if oa <= 0.0001 {
        return [0, 0, 0, 0];
    }
    let mut out = [0u8; 4];
    for i in 0..3 {
        let c = (col[i] as f32 * a + dst[i] as f32 * da * (1.0 - a)) / oa;
        out[i] = c.round().clamp(0.0, 255.0) as u8;
    }
    out[3] = (oa * 255.0).round() as u8;
    out
}

/// Alpha-weighted average colour (RGB 0..1) and mean alpha around (cx, cy).
fn sample_average(layer: &Layer, cx: f32, cy: f32, r: f32) -> (f32, f32, f32, f32) {
    let (mut sr, mut sg, mut sb, mut sa, mut n) = (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let offs = [(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (0.7, 0.7), (-0.7, 0.7), (0.7, -0.7), (-0.7, -0.7)];
    for (ox, oy) in offs {
        let x = (cx + ox * r).round() as i64;
        let y = (cy + oy * r).round() as i64;
        if x < 0 || y < 0 || x >= layer.width as i64 || y >= layer.height as i64 {
            continue;
        }
        let idx = ((y as usize) * layer.width as usize + x as usize) * 4;
        let a = layer.pixels[idx + 3] as f32 / 255.0;
        sr += layer.pixels[idx] as f32 / 255.0 * a;
        sg += layer.pixels[idx + 1] as f32 / 255.0 * a;
        sb += layer.pixels[idx + 2] as f32 / 255.0 * a;
        sa += a;
        n += 1.0;
    }
    if sa <= 0.0001 || n == 0.0 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    (sr / sa, sg / sa, sb / sa, sa / n)
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ToolKind {
    #[default]
    Pen,
    Pencil,
    Brush,
    Airbrush,
    Eraser,
    Fill,
    ComicFrame,
    SpeechBalloon,
    Screentone,
    Eyedropper,
    SelectionRect,
    SelectionEllipse,
    SelectionLasso,
    AutoSelect,
    CloseAndFill,
    Hand,
    Zoom,
    Figure,
    Ruler,
}

impl ToolKind {
    pub fn name(&self) -> &'static str {
        match self {
            ToolKind::Pen => "Pen",
            ToolKind::Pencil => "Pencil",
            ToolKind::Brush => "Brush",
            ToolKind::Airbrush => "Airbrush",
            ToolKind::Eraser => "Eraser",
            ToolKind::Fill => "Fill (Bucket)",
            ToolKind::ComicFrame => "Comic Frame Border",
            ToolKind::SpeechBalloon => "Speech Balloon",
            ToolKind::Screentone => "Screentone",
            ToolKind::Eyedropper => "Eyedropper",
            ToolKind::SelectionRect => "Rectangle Selection",
            ToolKind::SelectionEllipse => "Ellipse Selection",
            ToolKind::SelectionLasso => "Lasso Selection",
            ToolKind::AutoSelect => "Auto Select (Magic Wand)",
            ToolKind::CloseAndFill => "Close and Fill",
            ToolKind::Hand => "Hand (Pan)",
            ToolKind::Zoom => "Zoom",
            ToolKind::Figure => "Figure / Speed Lines",
            ToolKind::Ruler => "Ruler / Symmetry",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SubTool {
    #[default]
    GPen,
    MappingPen,
    KaburaPen,
    RealGPen,
    Calligraphy,
    TexturedPen,
    RoughPencil,
    RealPencil,
    DesignPencil,
    MechanicalPencil,
    ColoredPencil,
    DenseWatercolor,
    OpaqueWatercolor,
    TransparentWatercolor,
    OilPaint,
    Gouache,
    AirbrushSoft,
    AirbrushHard,
    AirbrushSpray,
    HardEraser,
    SoftEraser,
    KneadedEraser,
    VectorEraser,
    RoughEraser,
    FeltPen,
    Millipen,
    SignPen,
    Pastel,
    Charcoal,
    OilPastel,
    Chalk,
    IndiaInk,
    DryBrush,
    FocusLines,
    StreamLines,
    FigureRect,
    FigureEllipse,
    FigureLine,
    SymmetryRuler,
    LinearRuler,
    PerspectiveRuler,
    SelectionEllipseSub,
    AutoSelectReferAll,
    AutoSelectReferEditing,
    CloseAndFillTool,
    Custom(u32),
}

impl SubTool {
    pub fn name(&self) -> &'static str {
        match self {
            SubTool::GPen => "G-Pen",
            SubTool::MappingPen => "Mapping Pen (Maru)",
            SubTool::KaburaPen => "Turnip Pen (Kabura)",
            SubTool::RealGPen => "Real G-Pen",
            SubTool::Calligraphy => "Calligraphy Nib (45°)",
            SubTool::TexturedPen => "Textured Pen",
            SubTool::FeltPen => "Felt Pen (Marker)",
            SubTool::Millipen => "Millipen (Technical)",
            SubTool::SignPen => "Sign Pen",
            SubTool::RoughPencil => "Darker Pencil",
            SubTool::RealPencil => "Real Pencil (Textured)",
            SubTool::DesignPencil => "Design Pencil",
            SubTool::MechanicalPencil => "Mechanical Pencil",
            SubTool::ColoredPencil => "Colored Pencil",
            SubTool::Pastel => "Soft Pastel",
            SubTool::Charcoal => "Charcoal Stick",
            SubTool::OilPastel => "Oil Pastel",
            SubTool::Chalk => "Chalk",
            SubTool::DenseWatercolor => "Dense Watercolor",
            SubTool::OpaqueWatercolor => "Opaque Watercolor",
            SubTool::TransparentWatercolor => "Transparent Watercolor",
            SubTool::OilPaint => "Oil Paint",
            SubTool::Gouache => "Gouache",
            SubTool::IndiaInk => "India Ink (Smooth)",
            SubTool::DryBrush => "Dry Ink Brush (Kasure)",
            SubTool::AirbrushSoft => "Soft Airbrush",
            SubTool::AirbrushHard => "Hard Airbrush",
            SubTool::AirbrushSpray => "Droplet Spray Airbrush",
            SubTool::HardEraser => "Hard Eraser",
            SubTool::SoftEraser => "Soft Eraser",
            SubTool::KneadedEraser => "Kneaded Eraser",
            SubTool::VectorEraser => "Vector Line Eraser",
            SubTool::RoughEraser => "Rough Eraser",
            SubTool::FocusLines => "Radiation Focus Lines (Sunburst)",
            SubTool::StreamLines => "Directional Speed Lines (Stream)",
            SubTool::FigureRect => "Direct Draw: Rectangle",
            SubTool::FigureEllipse => "Direct Draw: Ellipse",
            SubTool::FigureLine => "Direct Draw: Straight Line",
            SubTool::SymmetryRuler => "Symmetrical Ruler",
            SubTool::LinearRuler => "Linear Ruler",
            SubTool::PerspectiveRuler => "Perspective Ruler",
            SubTool::SelectionEllipseSub => "Ellipse Selection",
            SubTool::AutoSelectReferAll => "Refer All Layers (Auto Select)",
            SubTool::AutoSelectReferEditing => "Refer Editing Layer (Auto Select)",
            SubTool::CloseAndFillTool => "Close and Fill (Lasso Fill)",
            SubTool::Custom(_) => "Custom Sub Tool",
        }
    }
}

/// 8-bit grayscale coverage or paper texture bitmap (`0..=255`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrayMap {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl GrayMap {
    /// Builds a centered, square, normalized brush tip coverage map from an RGBA image.
    pub fn from_rgba_tip(width: u32, height: u32, rgba: &[u8]) -> Option<Self> {
        if width == 0 || height == 0 || rgba.len() < (width * height * 4) as usize {
            return None;
        }
        let w = width as usize;
        let h = height as usize;
        let mut min_a = 255u8;
        let mut max_a = 0u8;
        for px in rgba.chunks_exact(4) {
            min_a = min_a.min(px[3]);
            max_a = max_a.max(px[3]);
        }
        let use_alpha = max_a > 10 && min_a < 245;
        let mut raw = vec![0u8; w * h];
        for (i, px) in rgba.chunks_exact(4).enumerate() {
            let cov = if use_alpha {
                // If RGB is dark ink on transparent, alpha is coverage; if RGB is grayscale with alpha, combine
                px[3]
            } else {
                let luma = ((77 * px[0] as u32 + 150 * px[1] as u32 + 29 * px[2] as u32) >> 8) as u8;
                255u8.saturating_sub(luma)
            };
            raw[i] = cov;
        }

        // Find tight bounding box of non-zero coverage
        let mut x0 = w;
        let mut y0 = h;
        let mut x1 = 0usize;
        let mut y1 = 0usize;
        let mut peak = 0u8;
        for y in 0..h {
            for x in 0..w {
                let c = raw[y * w + x];
                if c > 6 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                    peak = peak.max(c);
                }
            }
        }
        if x1 < x0 || y1 < y0 || peak == 0 {
            return None;
        }

        let cw = x1 - x0 + 1;
        let ch = y1 - y0 + 1;
        let side = cw.max(ch);
        let pad_x = (side - cw) / 2;
        let pad_y = (side - ch) / 2;
        let mut sq = vec![0u8; side * side];
        for y in 0..ch {
            for x in 0..cw {
                let c = raw[(y0 + y) * w + (x0 + x)];
                let norm = ((c as u32 * 255) / (peak as u32)).min(255) as u8;
                sq[(pad_y + y) * side + (pad_x + x)] = norm;
            }
        }

        // Downscale if larger than 128x128 to keep stamping cache-hot
        let target = side.min(128);
        if target == side {
            Some(Self {
                width: side as u32,
                height: side as u32,
                data: sq,
            })
        } else {
            let mut out = vec![0u8; target * target];
            for ty in 0..target {
                let sy = (ty * side) / target;
                for tx in 0..target {
                    let sx = (tx * side) / target;
                    out[ty * target + tx] = sq[sy * side + sx];
                }
            }
            Some(Self {
                width: target as u32,
                height: target as u32,
                data: out,
            })
        }
    }

    /// Builds a tileable paper grain map from an RGBA image.
    pub fn from_rgba_paper(width: u32, height: u32, rgba: &[u8], invert: bool) -> Option<Self> {
        if width == 0 || height == 0 || rgba.len() < (width * height * 4) as usize {
            return None;
        }
        let w = width as usize;
        let h = height as usize;
        let mut min_a = 255u8;
        let mut max_a = 0u8;
        let mut min_r = 255u8;
        let mut max_r = 0u8;
        for px in rgba.chunks_exact(4) {
            min_a = min_a.min(px[3]);
            max_a = max_a.max(px[3]);
            min_r = min_r.min(px[0]);
            max_r = max_r.max(px[0]);
        }
        // CSP paper textures in .sut store grain in Alpha with RGB=0
        let use_alpha = (max_a.saturating_sub(min_a) > 10) && (max_r.saturating_sub(min_r) <= 10);
        let mut raw = vec![0u8; w * h];
        let mut lo = 255u8;
        let mut hi = 0u8;
        for (i, px) in rgba.chunks_exact(4).enumerate() {
            let v = if use_alpha {
                px[3]
            } else {
                ((77 * px[0] as u32 + 150 * px[1] as u32 + 29 * px[2] as u32) >> 8) as u8
            };
            let v = if invert { 255u8.saturating_sub(v) } else { v };
            lo = lo.min(v);
            hi = hi.max(v);
            raw[i] = v;
        }
        let span = (hi.saturating_sub(lo)).max(1) as u32;
        for v in &mut raw {
            *v = (((*v).saturating_sub(lo) as u32 * 255) / span).min(255) as u8;
        }
        Some(Self {
            width,
            height,
            data: raw,
        })
    }

    /// Bilinear sample at normalized coordinates `(u, v)` in `[0.0, 1.0]`. Returns `0.0..=1.0`.
    #[inline]
    pub fn sample_bilinear(&self, u: f32, v: f32) -> f32 {
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) || self.data.is_empty() {
            return 0.0;
        }
        let wf = (self.width.saturating_sub(1)) as f32;
        let hf = (self.height.saturating_sub(1)) as f32;
        let gx = u * wf;
        let gy = v * hf;
        let x0 = gx.floor() as usize;
        let y0 = gy.floor() as usize;
        let x1 = (x0 + 1).min(self.width as usize - 1);
        let y1 = (y0 + 1).min(self.height as usize - 1);
        let fx = gx - x0 as f32;
        let fy = gy - y0 as f32;
        let w = self.width as usize;
        let c00 = self.data[y0 * w + x0] as f32;
        let c10 = self.data[y0 * w + x1] as f32;
        let c01 = self.data[y1 * w + x0] as f32;
        let c11 = self.data[y1 * w + x1] as f32;
        let top = c00 + (c10 - c00) * fx;
        let bot = c01 + (c11 - c01) * fx;
        (top + (bot - top) * fy) / 255.0
    }

    /// Tiled paper grain lookup at canvas coordinate `(x, y)` with scale factor. Returns `0.0..=1.0`.
    #[inline]
    pub fn sample_tiled(&self, x: f32, y: f32, scale: f32) -> f32 {
        if self.width == 0 || self.height == 0 || self.data.is_empty() {
            return 1.0;
        }
        let s = scale.max(0.25);
        let tx = (x / s).rem_euclid(self.width as f32) as usize;
        let ty = (y / s).rem_euclid(self.height as f32) as usize;
        let ix = tx.min(self.width as usize - 1);
        let iy = ty.min(self.height as usize - 1);
        self.data[iy * self.width as usize + ix] as f32 / 255.0
    }
}

/// User-imported or duplicated custom Sub Tool (`.sut`, `.png` tip, or custom preset).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomSubTool {
    pub id: u32,
    pub name: String,
    pub tool_kind: ToolKind,
    pub preset: crate::brush::BrushPreset,
    #[serde(default = "default_antialiasing")]
    pub antialiasing: u8,
    pub tip_stamp: Option<GrayMap>,
    pub paper_texture: Option<GrayMap>,
    #[serde(default)]
    pub source_path: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FillReferenceMode {
    #[default]
    ReferActiveLayer,
    ReferAllLayers, // Clip Studio Paint smart coloring beneath lineart
    ReferReferenceLayer, // Clip Studio Paint Lighthouse reference layer
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolConfig {
    pub tool: ToolKind,
    pub sub_tool: SubTool,
    pub size: f32,          // Brush diameter in pixels (1.0 to 500.0)
    pub opacity: f32,       // 0.0 to 1.0
    pub stabilization: u32, // 0 to 50
    pub hardness: f32,      // 0.0 (soft edge) to 1.0 (hard edge)
    #[serde(default = "default_antialiasing")]
    pub antialiasing: u8,   // 0 = None, 1 = Weak, 2 = Mid, 3 = Strong
    pub fill_mode: FillReferenceMode,
    pub fill_tolerance: u8,
    #[serde(default)]
    pub fill_close_gap: u32,
    #[serde(default)]
    pub fill_area_scaling: i32,
    #[serde(default)]
    pub selection_op: crate::selection::SelectionOp,
    #[serde(default = "default_wand_tolerance")]
    pub wand_tolerance: u8,
    #[serde(default = "default_true")]
    pub wand_contiguous: bool,
    #[serde(default)]
    pub wand_close_gap: u32,
    #[serde(default)]
    pub wand_area_scaling: i32,
    pub primary_color: [u8; 4],
    pub secondary_color: [u8; 4],
    #[serde(default)]
    pub custom_sub_tools: Vec<CustomSubTool>,
    #[serde(default)]
    pub preset_override: Option<crate::brush::BrushPreset>,
    #[serde(default)]
    pub focus_lines: crate::speed_lines::FocusLinesConfig,
    #[serde(default)]
    pub stream_lines: crate::speed_lines::StreamLinesConfig,
}

fn default_antialiasing() -> u8 {
    2
}

fn default_wand_tolerance() -> u8 {
    15
}

fn default_true() -> bool {
    true
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            tool: ToolKind::Pen,
            sub_tool: SubTool::GPen,
            size: 8.0,
            opacity: 1.0,
            stabilization: 10,
            hardness: 1.0,
            antialiasing: 2,
            fill_mode: FillReferenceMode::ReferAllLayers,
            fill_tolerance: 15,
            fill_close_gap: 0,
            fill_area_scaling: 0,
            selection_op: crate::selection::SelectionOp::Replace,
            wand_tolerance: 15,
            wand_contiguous: true,
            wand_close_gap: 0,
            wand_area_scaling: 0,
            primary_color: [20, 20, 24, 255],     // Dark ink
            secondary_color: [255, 255, 255, 255], // White
            custom_sub_tools: Vec::new(),
            preset_override: None,
            focus_lines: crate::speed_lines::FocusLinesConfig::default(),
            stream_lines: crate::speed_lines::StreamLinesConfig::default(),
        }
    }
}

impl ToolConfig {
    /// Returns the display name of any `SubTool` (including imported custom sub-tools).
    pub fn sub_tool_display_name(&self, sub: SubTool) -> String {
        if let SubTool::Custom(id) = sub {
            if let Some(c) = self.custom_sub_tools.iter().find(|c| c.id == id) {
                return c.name.clone();
            }
        }
        sub.name().to_string()
    }

    /// Returns the display name of the currently active sub-tool.
    pub fn active_sub_tool_name(&self) -> String {
        self.sub_tool_display_name(self.sub_tool)
    }

    /// Returns the `BrushPreset` for a given `SubTool`.
    pub fn preset_for(&self, tool: ToolKind, sub: SubTool) -> crate::brush::BrushPreset {
        if let SubTool::Custom(id) = sub {
            if let Some(c) = self.custom_sub_tools.iter().find(|c| c.id == id) {
                return c.preset;
            }
        }
        crate::brush::BrushPreset::for_tool(tool, sub)
    }

    /// Returns the currently active `BrushPreset` (with any live Sub Tool Detail edits applied).
    pub fn active_preset(&self) -> crate::brush::BrushPreset {
        if let Some(ov) = self.preset_override {
            return ov;
        }
        self.preset_for(self.tool, self.sub_tool)
    }

    /// Selects a sub-tool and applies its default Clip Studio Paint preset parameters.
    pub fn select_sub_tool(&mut self, sub: SubTool) {
        self.sub_tool = sub;
        self.preset_override = None;
        if let SubTool::Custom(id) = sub {
            if let Some(c) = self.custom_sub_tools.iter().find(|c| c.id == id) {
                self.tool = c.tool_kind;
                self.size = c.preset.default_size;
                self.opacity = c.preset.default_opacity;
                self.hardness = c.preset.default_hardness;
                self.stabilization = c.preset.default_stabilization;
                self.antialiasing = c.antialiasing;
                return;
            }
        }
        let preset = crate::brush::BrushPreset::for_tool(self.tool, sub);
        self.size = preset.default_size;
        self.opacity = preset.default_opacity;
        self.hardness = preset.default_hardness;
        self.stabilization = preset.default_stabilization;
    }

    /// Registers a newly imported or duplicated custom sub-tool and selects it immediately.
    pub fn register_custom_sub_tool(&mut self, mut custom: CustomSubTool) -> SubTool {
        let next_id = self
            .custom_sub_tools
            .iter()
            .map(|c| c.id)
            .max()
            .unwrap_or(0)
            + 1;
        if custom.id == 0
            || self
                .custom_sub_tools
                .iter()
                .any(|c| c.id == custom.id)
        {
            custom.id = next_id;
        }
        let st = SubTool::Custom(custom.id);
        self.tool = custom.tool_kind;
        self.custom_sub_tools.push(custom);
        self.select_sub_tool(st);
        st
    }
}



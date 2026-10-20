use crate::blend::BlendMode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LayerColorTag {
    #[default]
    None,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Gray,
}

impl LayerColorTag {
    pub fn name(&self) -> &'static str {
        match self {
            LayerColorTag::None => "None",
            LayerColorTag::Red => "Red",
            LayerColorTag::Orange => "Orange",
            LayerColorTag::Yellow => "Yellow",
            LayerColorTag::Green => "Green",
            LayerColorTag::Blue => "Blue",
            LayerColorTag::Purple => "Purple",
            LayerColorTag::Gray => "Gray",
        }
    }

    pub fn rgb(&self) -> Option<[u8; 3]> {
        match self {
            LayerColorTag::None => None,
            LayerColorTag::Red => Some([220, 60, 60]),
            LayerColorTag::Orange => Some([235, 130, 45]),
            LayerColorTag::Yellow => Some([230, 190, 40]),
            LayerColorTag::Green => Some([60, 180, 80]),
            LayerColorTag::Blue => Some([50, 130, 230]),
            LayerColorTag::Purple => Some([150, 70, 220]),
            LayerColorTag::Gray => Some([130, 130, 130]),
        }
    }
}

/// What a layer row in the Layer palette represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LayerKind {
    /// Pixel layer.
    #[default]
    Raster,
    /// Vector layer: strokes are stored as editable paths and re-rasterized into `pixels`.
    Vector,
    /// Layer folder (group). Holds no pixels; children reference it through `parent`.
    Folder,
}

/// Brush settings captured with a vector stroke so it can be re-rasterized after edits.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorStyle {
    pub tool: crate::tools::ToolKind,
    pub sub_tool: crate::tools::SubTool,
    pub size: f32,
    pub opacity: f32,
    pub hardness: f32,
    pub antialiasing: u8,
    pub color: [u8; 4],
    pub preset: crate::brush::BrushPreset,
    #[serde(default)]
    pub custom: Option<crate::tools::CustomSubTool>,
}

impl VectorStyle {
    pub fn from_tool_config(cfg: &crate::tools::ToolConfig) -> Self {
        let custom = if let crate::tools::SubTool::Custom(id) = cfg.sub_tool {
            cfg.custom_sub_tools.iter().find(|c| c.id == id).cloned()
        } else {
            None
        };
        Self {
            tool: cfg.tool,
            sub_tool: cfg.sub_tool,
            size: cfg.size,
            opacity: cfg.opacity,
            hardness: cfg.hardness,
            antialiasing: cfg.antialiasing,
            color: cfg.primary_color,
            preset: cfg.active_preset(),
            custom,
        }
    }
}

/// One editable vector line (centerline points with pressure).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorStroke {
    pub style: VectorStyle,
    pub points: Vec<crate::stroke::StrokePoint>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer {
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub opacity: f32, // 0.0 to 1.0
    pub blend_mode: BlendMode,
    pub clipped_to_below: bool, // Clip Studio Paint Clipping Mask
    pub alpha_locked: bool,     // Lock Transparent Pixels
    pub color_tag: LayerColorTag,
    pub is_draft: bool,         // Clip Studio Paint Draft Sketch Layer (blue pencil)
    pub locked: bool,           // Lock Layer from edits
    pub width: u32,
    pub height: u32,
    #[serde(skip)]
    pub pixels: Vec<u8>, // RGBA, len = width * height * 4
    /// Raster, vector or folder.
    #[serde(default)]
    pub kind: LayerKind,
    /// Id of the folder this layer lives in (None = top level).
    #[serde(default)]
    pub parent: Option<u64>,
    /// Folder rows: whether children are shown in the Layer palette.
    #[serde(default = "default_true")]
    pub expanded: bool,
    /// Layer mask: one byte per pixel, 255 = shown, 0 = hidden.
    #[serde(skip)]
    pub mask: Option<Vec<u8>>,
    /// Whether the mask is applied when compositing.
    #[serde(default)]
    pub mask_enabled: bool,
    /// When true, brush strokes paint into the mask instead of the pixels
    /// (drawing reveals, erasing hides — same as CSP).
    #[serde(default)]
    pub editing_mask: bool,
    /// Editable vector strokes (vector layers only).
    #[serde(default)]
    pub vector_strokes: Vec<VectorStroke>,
    /// Reference layer (CSP "lighthouse"): fill / magic wand can refer only to these layers.
    #[serde(default)]
    pub is_reference: bool,
}

fn default_true() -> bool {
    true
}

impl Layer {
    pub fn new(id: u64, name: impl Into<String>, width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        Self {
            id,
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            clipped_to_below: false,
            alpha_locked: false,
            color_tag: LayerColorTag::None,
            is_draft: false,
            locked: false,
            width,
            height,
            pixels: vec![0; size],
            kind: LayerKind::Raster,
            parent: None,
            expanded: true,
            mask: None,
            mask_enabled: false,
            editing_mask: false,
            vector_strokes: Vec::new(),
            is_reference: false,
        }
    }

    /// A layer folder. Folders own no pixels (width/height 0) so every pixel accessor is a no-op.
    pub fn new_folder(id: u64, name: impl Into<String>) -> Self {
        let mut l = Self::new(id, name, 0, 0);
        l.kind = LayerKind::Folder;
        l
    }

    pub fn is_folder(&self) -> bool {
        self.kind == LayerKind::Folder
    }

    pub fn is_vector(&self) -> bool {
        self.kind == LayerKind::Vector
    }

    /// True if this layer holds pixels that can be painted on.
    pub fn is_drawable(&self) -> bool {
        !self.is_folder() && !self.pixels.is_empty()
    }

    /// Creates a fully-revealing mask (all 255) if none exists, and enables it.
    pub fn add_mask(&mut self) {
        if self.is_folder() {
            return;
        }
        if self.mask.is_none() {
            self.mask = Some(vec![255u8; (self.width * self.height) as usize]);
        }
        self.mask_enabled = true;
    }

    /// Bakes the mask into the alpha channel and removes it.
    pub fn apply_mask(&mut self) {
        if let Some(mask) = self.mask.take() {
            for (px, m) in self.pixels.chunks_exact_mut(4).zip(mask.iter()) {
                px[3] = ((px[3] as u32 * *m as u32) / 255) as u8;
            }
        }
        self.mask_enabled = false;
        self.editing_mask = false;
    }

    pub fn delete_mask(&mut self) {
        self.mask = None;
        self.mask_enabled = false;
        self.editing_mask = false;
    }

    pub fn new_with_color(
        id: u64,
        name: impl Into<String>,
        width: u32,
        height: u32,
        color: [u8; 4],
    ) -> Self {
        let mut layer = Self::new(id, name, width, height);
        for pixel in layer.pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
        layer
    }

    #[inline(always)]
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        Some([
            self.pixels[idx],
            self.pixels[idx + 1],
            self.pixels[idx + 2],
            self.pixels[idx + 3],
        ])
    }

    #[inline(always)]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        if self.alpha_locked {
            let cur_alpha = self.pixels[idx + 3];
            if cur_alpha == 0 {
                return; // Transparent pixel locked
            }
            self.pixels[idx] = color[0];
            self.pixels[idx + 1] = color[1];
            self.pixels[idx + 2] = color[2];
            // Preserve original alpha
        } else {
            self.pixels[idx..idx + 4].copy_from_slice(&color);
        }
    }

    /// Blends a stamp pixel directly onto this layer
    #[inline(always)]
    pub fn blend_pixel_direct(&mut self, x: u32, y: u32, src: [u8; 4], brush_opacity: f32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        let dst = [
            self.pixels[idx],
            self.pixels[idx + 1],
            self.pixels[idx + 2],
            self.pixels[idx + 3],
        ];

        if self.alpha_locked && dst[3] == 0 {
            return; // Locked transparent pixels cannot be painted on
        }

        let mut blended = BlendMode::Normal.blend_pixel(dst, src, brush_opacity);
        if self.alpha_locked {
            blended[3] = dst[3]; // Keep existing alpha unchanged
        }
        self.pixels[idx..idx + 4].copy_from_slice(&blended);
    }

    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }

    /// Smart flood fill referencing other layers (e.g. Lineart layer) for boundary detection
    pub fn flood_fill_refer_layers(
        &mut self,
        start_x: u32,
        start_y: u32,
        fill_color: [u8; 4],
        reference_canvas: Option<&[u8]>,
        tolerance: u8,
    ) {
        if start_x >= self.width || start_y >= self.height {
            return;
        }

        let w = self.width;
        let h = self.height;

        // If no external reference canvas provided, take snapshot of current layer pixels
        let owned_snapshot: Option<Vec<u8>> = if reference_canvas.is_none() {
            Some(self.pixels.clone())
        } else {
            None
        };

        let ref_bytes: &[u8] = match reference_canvas {
            Some(r) => r,
            None => owned_snapshot.as_deref().unwrap(),
        };

        let sample_source = |x: u32, y: u32| -> [u8; 4] {
            let idx = ((y * w + x) * 4) as usize;
            [ref_bytes[idx], ref_bytes[idx + 1], ref_bytes[idx + 2], ref_bytes[idx + 3]]
        };

        let target_color = sample_source(start_x, start_y);
        if color_diff(target_color, fill_color) == 0 {
            return;
        }

        let mut queue = std::collections::VecDeque::new();
        let mut visited = vec![false; (w * h) as usize];

        queue.push_back((start_x, start_y));
        let start_idx = (start_y * w + start_x) as usize;
        visited[start_idx] = true;

        while let Some((cx, cy)) = queue.pop_front() {
            self.set_pixel(cx, cy, fill_color);

            let neighbors = [
                (cx.wrapping_sub(1), cy),
                (cx + 1, cy),
                (cx, cy.wrapping_sub(1)),
                (cx, cy + 1),
            ];

            for (nx, ny) in neighbors {
                if nx < w && ny < h {
                    let n_idx = (ny * w + nx) as usize;
                    if !visited[n_idx] {
                        let c = sample_source(nx, ny);
                        if color_diff(c, target_color) <= tolerance as u32 {
                            visited[n_idx] = true;
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }
        }
    }
}

#[inline(always)]
fn color_diff(c1: [u8; 4], c2: [u8; 4]) -> u32 {
    let dr = (c1[0] as i32 - c2[0] as i32).abs() as u32;
    let dg = (c1[1] as i32 - c2[1] as i32).abs() as u32;
    let db = (c1[2] as i32 - c2[2] as i32).abs() as u32;
    let da = (c1[3] as i32 - c2[3] as i32).abs() as u32;
    (dr + dg + db + da) / 4
}

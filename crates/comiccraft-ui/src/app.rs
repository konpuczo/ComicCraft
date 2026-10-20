use crate::canvas_view::CanvasViewport;
use crate::color_wheel::{hsv_to_rgb, rgb_to_hsv, ColorWheelWidget};
use crate::icons::{
    csp_bar_slider, csp_bar_slider_reserved, csp_palette_tabs, draw_icon, draw_stroke_preview,
    icon_button, StudioIcon,
};
use comiccraft_clip::{import_custom_subtool_file, open_clip_file, save_clip_file};
use comiccraft_core::balloon::{BalloonShape, SpeechBalloon};
use comiccraft_core::blend::BlendMode;
use comiccraft_core::brush::TipTexture;
use comiccraft_core::document::Document;
use comiccraft_core::filter::{
    apply_brightness_contrast, apply_gaussian_blur, apply_hue_saturation_luminosity, apply_invert,
};
use comiccraft_core::history::HistoryManager;
use comiccraft_core::layer::LayerColorTag;
use comiccraft_core::screentone::{generate_screentone_tile, ScreentoneConfig};
use comiccraft_core::speed_lines::{render_focus_lines, render_stream_lines};
use comiccraft_core::tools::{CustomSubTool, FillReferenceMode, SubTool, ToolConfig, ToolKind};
use eframe::egui::{
    self, Align2, Color32, ColorImage, FontId, Key, Pos2, Rect, RichText, Rounding, Sense, Stroke,
    TextureOptions, Vec2,
};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopRightTab {
    Navigator,
    SubView,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidRightTab {
    ColorWheel,
    ColorSet,
    ColorSlider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubToolTab {
    SubTool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPropTab {
    ToolProperty,
    SubToolDetail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushSizeTab {
    BrushSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerPropTab {
    LayerProperty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerDockTab {
    Layer,
    SearchLayer,
}

#[derive(Default)]
pub struct BrightnessContrastDialog {
    pub open: bool,
    pub brightness: i32,
    pub contrast: i32,
    pub preview: bool,
    pub original_layer: Option<comiccraft_core::Layer>,
}

#[derive(Default)]
pub struct HueSatLumDialog {
    pub open: bool,
    pub hue: f32,
    pub sat: f32,
    pub lum: f32,
    pub preview: bool,
    pub original_layer: Option<comiccraft_core::Layer>,
}

#[derive(Default)]
pub struct GaussianBlurDialog {
    pub open: bool,
    pub radius: f32,
    pub preview: bool,
    pub original_layer: Option<comiccraft_core::Layer>,
}

#[derive(Default)]
pub struct BinarizationDialog {
    pub open: bool,
    pub threshold: u8,
    pub preview: bool,
    pub original_layer: Option<comiccraft_core::Layer>,
}

pub struct LevelsDialog {
    pub open: bool,
    pub in_black: u8,
    pub in_white: u8,
    pub gamma: f32,
    pub out_black: u8,
    pub out_white: u8,
    pub preview: bool,
    pub original_layer: Option<comiccraft_core::Layer>,
}

impl Default for LevelsDialog {
    fn default() -> Self {
        Self {
            open: false,
            in_black: 0,
            in_white: 255,
            gamma: 1.0,
            out_black: 0,
            out_white: 255,
            preview: true,
            original_layer: None,
        }
    }
}

pub struct GradientMapDialog {
    pub open: bool,
    pub selected_preset: usize,
    pub preview: bool,
    pub original_layer: Option<comiccraft_core::Layer>,
}

impl Default for GradientMapDialog {
    fn default() -> Self {
        Self {
            open: false,
            selected_preset: 0,
            preview: true,
            original_layer: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BorderSelectedDialog {
    pub open: bool,
    pub width_px: u32,
    pub position: comiccraft_core::SelectionBorderPosition,
    pub color: [u8; 4],
}

impl Default for BorderSelectedDialog {
    fn default() -> Self {
        Self {
            open: false,
            width_px: 2,
            position: comiccraft_core::SelectionBorderPosition::Centered,
            color: [20, 20, 24, 255],
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExpandSelectionDialog {
    pub open: bool,
    pub width_px: u32,
}

impl Default for ExpandSelectionDialog {
    fn default() -> Self {
        Self {
            open: false,
            width_px: 5,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShrinkSelectionDialog {
    pub open: bool,
    pub width_px: u32,
}

impl Default for ShrinkSelectionDialog {
    fn default() -> Self {
        Self {
            open: false,
            width_px: 5,
        }
    }
}

pub struct TabSession {
    pub doc: Document,
    pub history: HistoryManager,
    pub viewport: CanvasViewport,
    pub file_path: Option<PathBuf>,
}

pub struct TabMeta {
    #[allow(dead_code)]
    pub id: u64,
    pub session: Option<TabSession>,
    pub title: String,
}

pub struct ComicCraftApp {
    pub doc: Document,
    pub config: ToolConfig,
    pub history: HistoryManager,
    pub viewport: CanvasViewport,
    pub file_path: Option<PathBuf>,
    pub tabs: Vec<TabMeta>,
    pub active_tab: usize,
    pub next_tab_id: u64,
    pub status_message: String,
    pub screentone_config: ScreentoneConfig,
    pub new_dialog_open: bool,
    pub new_width: u32,
    pub new_height: u32,
    pub new_dpi: f32,
    pub new_preset_name: String,
    pub top_right_tab: TopRightTab,
    pub mid_right_tab: MidRightTab,
    pub tool_prop_tab: ToolPropTab,
    pub layer_dock_tab: LayerDockTab,
    pub is_transparent_color: bool,
    pub color_wheel_triangle: bool,
    pub sub_tool_category: usize,
    pub subview_path: Option<PathBuf>,
    pub subview_image: Option<egui::TextureHandle>,
    #[allow(dead_code)]
    pub custom_palette: Vec<[u8; 3]>,
    pub color_history: Vec<[u8; 3]>,
    pub hardness_step: usize,
    pub antialias_level: usize,
    pub color_set_category: usize,
    pub renaming_layer_idx: Option<usize>,
    pub renaming_layer_name: String,
    pub reference_layer_id: Option<u64>,
    pub layer_border_effect: bool,
    pub layer_tone_effect: bool,
    pub layer_color_effect: bool,
    pub show_material_drawer: bool,
    pub screenshot_target: Option<PathBuf>,
    pub frame_count: u32,
    pub bc_dialog: BrightnessContrastDialog,
    pub hsl_dialog: HueSatLumDialog,
    pub blur_dialog: GaussianBlurDialog,
    pub binarization_dialog: BinarizationDialog,
    pub levels_dialog: LevelsDialog,
    pub gradient_map_dialog: GradientMapDialog,
    pub border_dialog: BorderSelectedDialog,
    pub expand_dialog: ExpandSelectionDialog,
    pub shrink_dialog: ShrinkSelectionDialog,
    pub quick_mask_active: bool,
    pub pre_quick_mask_active_layer: usize,
}

impl Default for ComicCraftApp {
    fn default() -> Self {
        let doc = Document::new("Illustration_01.clip", 1600, 2400, 300.0);
        let mut viewport = CanvasViewport::default();
        viewport.pan = Vec2::new(750.0, 520.0);

        let mut app = Self {
            doc,
            config: ToolConfig::default(),
            history: HistoryManager::new(25),
            viewport,
            file_path: None,
            tabs: vec![TabMeta {
                id: 1,
                session: None,
                title: "Illustration_01.clip".to_string(),
            }],
            active_tab: 0,
            next_tab_id: 2,
            status_message: "CLIP STUDIO PAINT / ComicCraft Studio — Ready".to_string(),
            screentone_config: ScreentoneConfig::default(),
            new_dialog_open: false,
            new_width: 1600,
            new_height: 2400,
            new_dpi: 300.0,
            new_preset_name: "Standard Manga B5".to_string(),
            top_right_tab: TopRightTab::Navigator,
            mid_right_tab: MidRightTab::ColorWheel,
            tool_prop_tab: ToolPropTab::ToolProperty,
            layer_dock_tab: LayerDockTab::Layer,
            is_transparent_color: false,
            color_wheel_triangle: false,
            sub_tool_category: 0,
            subview_path: None,
            subview_image: None,
            custom_palette: Vec::new(),
            color_history: vec![
                [20, 20, 24],
                [255, 255, 255],
                [225, 55, 55],
                [45, 125, 235],
                [245, 195, 55],
                [65, 185, 95],
                [249, 201, 176],
                [140, 75, 215],
            ],
            hardness_step: 4,
            antialias_level: 2,
            color_set_category: 0,
            renaming_layer_idx: None,
            renaming_layer_name: String::new(),
            reference_layer_id: None,
            layer_border_effect: false,
            layer_tone_effect: false,
            layer_color_effect: false,
            show_material_drawer: false,
            screenshot_target: None,
            frame_count: 0,
            bc_dialog: BrightnessContrastDialog::default(),
            hsl_dialog: HueSatLumDialog::default(),
            blur_dialog: GaussianBlurDialog::default(),
            binarization_dialog: BinarizationDialog { threshold: 128, preview: true, ..Default::default() },
            levels_dialog: LevelsDialog::default(),
            gradient_map_dialog: GradientMapDialog::default(),
            border_dialog: BorderSelectedDialog::default(),
            expand_dialog: ExpandSelectionDialog::default(),
            shrink_dialog: ShrinkSelectionDialog::default(),
            quick_mask_active: false,
            pre_quick_mask_active_layer: 1,
        };
        app.load_persisted_custom_subtools();
        app
    }
}

impl eframe::App for ComicCraftApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [43.0 / 255.0, 45.0 / 255.0, 50.0 / 255.0, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.handle_screenshot(ctx);

        // =========================================================================
        // 1. Top Menu Bar (Authentic Clip Studio Paint EX/PRO Menu Hierarchy)
        // =========================================================================
        egui::TopBottomPanel::top("top_menu_bar")
            .exact_height(22.0)
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("New... (Ctrl+N)").clicked() {
                            self.new_dialog_open = true;
                            ui.close_menu();
                        }
                        if ui.button("New from Clipboard").clicked() {
                            self.new_canvas_from_clipboard();
                            ui.close_menu();
                        }
                        if ui.button("Open... (Ctrl+O)").clicked() {
                            self.open_file_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Import Sub Tool (.sut / Brush Tip)...").clicked() {
                            self.open_import_subtool_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Export Active Sub Tool (.ccbrush)...").clicked() {
                            self.export_active_subtool_dialog();
                            ui.close_menu();
                        }

                        ui.separator();
                        if ui.button("Save (Ctrl+S)").clicked() {
                            self.save_current_file();
                            ui.close_menu();
                        }
                        if ui.button("Save As... (Ctrl+Shift+S)").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("CLIP STUDIO FORMAT (*.clip)", &["clip"])
                                .set_file_name(&self.doc.name)
                                .save_file()
                            {
                                self.file_path = Some(path);
                                self.save_current_file();
                            }
                            ui.close_menu();
                        }
                        if ui.button("Close Canvas (Ctrl+W)").clicked() {
                            self.close_tab(self.active_tab);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Export (Single Layer) > .png (PNG)...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PNG (*.png)", &["png"])
                                .set_file_name("export.png")
                                .save_file()
                            {
                                let img = self.doc.to_rgba_image();
                                if let Err(e) = img.save(&path) {
                                    self.status_message = format!("Export error: {}", e);
                                } else {
                                    self.status_message = format!("Exported to {}", path.display());
                                }
                            }
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Edit", |ui| {
                        if ui
                            .add_enabled(self.history.can_undo(), egui::Button::new("Undo (Ctrl+Z)"))
                            .clicked()
                        {
                            self.history.undo(&mut self.doc);
                            self.status_message = "Undo".into();
                            ui.close_menu();
                        }
                        if ui
                            .add_enabled(self.history.can_redo(), egui::Button::new("Redo (Ctrl+Y)"))
                            .clicked()
                        {
                            self.history.redo(&mut self.doc);
                            self.status_message = "Redo".into();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Cut (Ctrl+X)").clicked() {
                            self.cut_to_clipboard();
                            ui.close_menu();
                        }
                        if ui.button("Copy (Ctrl+C)").clicked() {
                            self.copy_to_clipboard(false);
                            ui.close_menu();
                        }
                        if ui.button("Copy Merged (Ctrl+Shift+C)").clicked() {
                            self.copy_to_clipboard(true);
                            ui.close_menu();
                        }
                        if ui.button("Paste (Ctrl+V)").clicked() {
                            self.paste_from_clipboard();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Clear (Delete)").clicked() {
                            self.history.push_state("Clear", &self.doc);
                            self.doc.clear_active_selection_or_layer();
                            ui.close_menu();
                        }
                        if ui.button("Fill (Alt+Delete)").clicked() {
                            self.history.push_state("Fill", &self.doc);
                            self.doc
                                .flood_fill(self.doc.width / 2, self.doc.height / 2, &self.config);
                            ui.close_menu();
                        }
                        if ui.button("Convert Brightness to Opacity").clicked() {
                            self.history.push_state("Convert Brightness to Opacity", &self.doc);
                            if self.doc.convert_brightness_to_opacity() {
                                self.status_message = "Converted layer brightness to opacity (transparent lineart).".to_string();
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button("Transform", |ui| {
                            if ui.button("Scale / Rotate / Free Transform (Ctrl+T)").clicked() {
                                self.viewport.start_transform(&mut self.doc);
                                ui.close_menu();
                            }
                        });
                        ui.menu_button("Tonal Correction", |ui| {
                            if ui.button("Brightness / Contrast...").clicked() {
                                self.open_brightness_contrast_dialog();
                                ui.close_menu();
                            }
                            if ui.button("Hue / Saturation / Luminosity...").clicked() {
                                self.open_hue_sat_lum_dialog();
                                ui.close_menu();
                            }
                            if ui.button("Level Correction (Levels)...").clicked() {
                                self.open_levels_dialog();
                                ui.close_menu();
                            }
                            if ui.button("Binarization (Threshold)...").clicked() {
                                self.open_binarization_dialog();
                                ui.close_menu();
                            }
                            ui.menu_button("Gradient Map", |ui| {
                                if ui.button("Gradient Map Settings...").clicked() {
                                    self.open_gradient_map_dialog();
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Preset: Sunset Glow").clicked() {
                                    self.apply_gradient_preset(0);
                                    ui.close_menu();
                                }
                                if ui.button("Preset: Sepia Tone").clicked() {
                                    self.apply_gradient_preset(1);
                                    ui.close_menu();
                                }
                                if ui.button("Preset: Anime Sky").clicked() {
                                    self.apply_gradient_preset(2);
                                    ui.close_menu();
                                }
                                if ui.button("Preset: Cyberpunk Neon").clicked() {
                                    self.apply_gradient_preset(3);
                                    ui.close_menu();
                                }
                            });
                            ui.separator();
                            if ui.button("Invert Colors (Ctrl+I)").clicked() {
                                self.apply_invert_colors();
                                ui.close_menu();
                            }
                        });
                    });

                    ui.menu_button("Story", |ui| {
                        if ui.button("Split Frame Horizontally").clicked() {
                            self.history.push_state("Split Frame H", &self.doc);
                            let cy = self.doc.height as f32 * 0.5;
                            self.doc.frame_manager.split_horizontal(0, cy);
                            self.doc.render_comic_panels([20, 20, 25, 255]);
                            ui.close_menu();
                        }
                        if ui.button("Split Frame Vertically").clicked() {
                            self.history.push_state("Split Frame V", &self.doc);
                            let cx = self.doc.width as f32 * 0.5;
                            self.doc.frame_manager.split_vertical(0, cx);
                            self.doc.render_comic_panels([20, 20, 25, 255]);
                            ui.close_menu();
                        }
                        if ui.button("Render Frame Borders to Active Layer").clicked() {
                            self.history.push_state("Render Frame Borders", &self.doc);
                            self.doc.render_comic_panels([20, 20, 25, 255]);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Add Speech Balloon").clicked() {
                            self.history.push_state("Add Speech Balloon", &self.doc);
                            let balloon = SpeechBalloon::default();
                            self.doc.add_speech_balloon(&balloon);
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Layer", |ui| {
                        if ui.button("New Raster Layer (Ctrl+Shift+N)").clicked() {
                            self.history.push_state("New Layer", &self.doc);
                            self.doc
                                .add_layer(format!("Layer {}", self.doc.layers.len() + 1));
                            ui.close_menu();
                        }
                        if ui.button("New Vector Layer").clicked() {
                            self.history.push_state("New Vector Layer", &self.doc);
                            self.doc
                                .add_vector_layer(format!("Vector {}", self.doc.layers.len() + 1));
                            ui.close_menu();
                        }
                        if ui.button("New Layer Folder").clicked() {
                            self.history.push_state("New Folder", &self.doc);
                            self.doc.add_folder(format!("Folder {}", self.doc.layers.len() + 1));
                            ui.close_menu();
                        }
                        if ui.button("Paste as New Layer (Ctrl+V)").clicked() {
                            self.paste_from_clipboard();
                            ui.close_menu();
                        }
                        if ui.button("New Layer > Tone...").clicked() {
                            self.history.push_state("New Tone Layer", &self.doc);
                            let tiles = generate_screentone_tile(
                                self.doc.width,
                                self.doc.height,
                                &self.screentone_config,
                            );
                            let l_idx = self.doc.add_layer(format!(
                                "Tone {:.0}L {:.0}%",
                                self.screentone_config.frequency,
                                self.screentone_config.density * 100.0
                            ));
                            if let Some(l) = self.doc.layers.get_mut(l_idx) {
                                l.pixels = tiles;
                                l.blend_mode = BlendMode::Multiply;
                            }
                            self.doc.mark_full_dirty();
                            ui.close_menu();
                        }
                        if ui.button("Duplicate Layer (Ctrl+J)").clicked() {
                            self.duplicate_active_layer();
                            ui.close_menu();
                        }
                        if ui.button("Merge with Layer Below (Ctrl+E)").clicked() {
                            self.merge_active_layer_down();
                            ui.close_menu();
                        }
                        if ui.button("Merge Visible Layers").clicked() {
                            self.merge_visible_layers();
                            ui.close_menu();
                        }
                        if ui.button("Flatten Image").clicked() {
                            self.flatten_image();
                            ui.close_menu();
                        }
                        if ui.button("Delete Layer").clicked() {
                            self.history.push_state("Delete Layer", &self.doc);
                            self.doc.delete_layer(self.doc.active_layer_idx);
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button("Layer Mask", |ui| {
                            let has_mask = self.doc.layers.get(self.doc.active_layer_idx).map_or(false, |l| l.mask.is_some());
                            if !has_mask {
                                if ui.button("Mask Outside Selection / Create Mask").clicked() {
                                    self.history.push_state("Create Layer Mask", &self.doc);
                                    self.doc.add_layer_mask(self.doc.active_layer_idx);
                                    ui.close_menu();
                                }
                            } else {
                                if ui.button("Apply Layer Mask to Layer").clicked() {
                                    self.history.push_state("Apply Layer Mask", &self.doc);
                                    self.doc.apply_layer_mask(self.doc.active_layer_idx);
                                    ui.close_menu();
                                }
                                if ui.button("Delete Layer Mask").clicked() {
                                    self.history.push_state("Delete Layer Mask", &self.doc);
                                    self.doc.delete_layer_mask(self.doc.active_layer_idx);
                                    ui.close_menu();
                                }
                                let mask_en = self.doc.layers.get(self.doc.active_layer_idx).map_or(true, |l| l.mask_enabled);
                                if ui.button(if mask_en { "Disable Layer Mask" } else { "Enable Layer Mask" }).clicked() {
                                    if let Some(l) = self.doc.layers.get_mut(self.doc.active_layer_idx) {
                                        l.mask_enabled = !l.mask_enabled;
                                    }
                                    self.doc.mark_full_dirty();
                                    ui.close_menu();
                                }
                            }
                        });
                        ui.separator();
                        if ui.button("Layer Settings > Clip to Layer Below (Ctrl+Alt+G)").clicked() {
                            if let Some(layer) = self.doc.active_layer_mut() {
                                layer.clipped_to_below = !layer.clipped_to_below;
                            }
                            ui.close_menu();
                        }
                        if ui.button("Layer Settings > Set as Reference Layer").clicked() {
                            if let Some(layer) = self.doc.active_layer_mut() {
                                layer.is_reference = !layer.is_reference;
                            }
                            ui.close_menu();
                        }
                        if ui.button("Layer Settings > Lock Transparent Pixels (/)").clicked() {
                            if let Some(layer) = self.doc.active_layer_mut() {
                                layer.alpha_locked = !layer.alpha_locked;
                            }
                            ui.close_menu();
                        }
                        if ui.button("Layer Settings > Set as Draft Layer").clicked() {
                            if let Some(layer) = self.doc.active_layer_mut() {
                                layer.is_draft = !layer.is_draft;
                            }
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Selection", |ui| {
                        if ui.button("Select All (Ctrl+A)").clicked() {
                            self.doc.selection.select_all();
                            ui.close_menu();
                        }
                        if ui.button("Deselect (Ctrl+D)").clicked() {
                            self.doc.selection.deselect();
                            ui.close_menu();
                        }
                        if ui.button("Invert Selected Area (Ctrl+Shift+I)").clicked() {
                            self.doc.selection.invert();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Border Selected Area...").clicked() {
                            self.border_dialog.open = true;
                            ui.close_menu();
                        }
                        if ui.button("Expand Selected Area...").clicked() {
                            self.expand_dialog.open = true;
                            ui.close_menu();
                        }
                        if ui.button("Shrink Selected Area...").clicked() {
                            self.shrink_dialog.open = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        let qm_label = if self.quick_mask_active {
                            "Exit Quick Mask (Ctrl+Q)"
                        } else {
                            "Quick Mask (Ctrl+Q)"
                        };
                        if ui.button(qm_label).clicked() {
                            self.toggle_quick_mask();
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("View", |ui| {
                        if ui.button("Fit to Navigator / Screen (Ctrl+0)").clicked() {
                            self.viewport.reset_view(
                                ui.available_size(),
                                self.doc.width,
                                self.doc.height,
                            );
                            ui.close_menu();
                        }
                        if ui.button("100% (Ctrl+1)").clicked() {
                            self.viewport.zoom = 1.0;
                            ui.close_menu();
                        }
                        if ui.button("Reset Rotation").clicked() {
                            self.viewport.rotation = 0.0;
                            ui.close_menu();
                        }
                        if ui.button("Rotate/Flip > Flip Horizontal").clicked() {
                            self.viewport.flip_h = !self.viewport.flip_h;
                            ui.close_menu();
                        }
                        if ui.button("Rotate/Flip > Flip Vertical").clicked() {
                            self.viewport.flip_v = !self.viewport.flip_v;
                            ui.close_menu();
                        }
                        ui.separator();
                        let ruler_lbl = if self.viewport.show_rulers {
                            "✓ Ruler Bar (Ctrl+R)"
                        } else {
                            "  Ruler Bar (Ctrl+R)"
                        };
                        if ui.button(ruler_lbl).clicked() {
                            self.viewport.show_rulers = !self.viewport.show_rulers;
                            ui.close_menu();
                        }
                        let guide_lbl = if self.viewport.show_manga_guides {
                            "✓ Crop Mark / Default Border (内枠・トンボ)"
                        } else {
                            "  Crop Mark / Default Border (内枠・トンボ)"
                        };
                        if ui.button(guide_lbl).clicked() {
                            self.viewport.show_manga_guides = !self.viewport.show_manga_guides;
                            ui.close_menu();
                        }
                        let grid_lbl = if self.viewport.show_grid {
                            "✓ Grid (G)"
                        } else {
                            "  Grid (G)"
                        };
                        if ui.button(grid_lbl).clicked() {
                            self.viewport.show_grid = !self.viewport.show_grid;
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Filter", |ui| {
                        ui.menu_button("Blur", |ui| {
                            if ui.button("Gaussian Blur...").clicked() {
                                self.open_gaussian_blur_dialog();
                                ui.close_menu();
                            }
                        });
                        ui.menu_button("Manga Effect", |ui| {
                            if ui.button("Radiation Focus Lines (Sunburst)...").clicked() {
                                self.config.tool = ToolKind::Figure;
                                self.config.select_sub_tool(SubTool::FocusLines);
                                ui.close_menu();
                            }
                            if ui.button("Directional Speed Lines (Stream)...").clicked() {
                                self.config.tool = ToolKind::Figure;
                                self.config.select_sub_tool(SubTool::StreamLines);
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("Add Manga Screentone Overlay").clicked() {
                                self.history.push_state("Add Screentone", &self.doc);
                                let tiles = generate_screentone_tile(
                                    self.doc.width,
                                    self.doc.height,
                                    &self.screentone_config,
                                );
                                let l_idx = self.doc.add_layer("Screentone 60L");
                                if let Some(l) = self.doc.layers.get_mut(l_idx) {
                                    l.pixels = tiles;
                                    l.blend_mode = BlendMode::Multiply;
                                }
                                self.doc.mark_full_dirty();
                                ui.close_menu();
                            }
                        });
                    });

                    ui.menu_button("Window", |ui| {
                        if ui.button("Navigator").clicked() {
                            self.top_right_tab = TopRightTab::Navigator;
                            ui.close_menu();
                        }
                        if ui.button("Sub View").clicked() {
                            self.top_right_tab = TopRightTab::SubView;
                            ui.close_menu();
                        }
                        if ui.button("History").clicked() {
                            self.top_right_tab = TopRightTab::History;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Color Wheel").clicked() {
                            self.mid_right_tab = MidRightTab::ColorWheel;
                            ui.close_menu();
                        }
                        if ui.button("Color Set").clicked() {
                            self.mid_right_tab = MidRightTab::ColorSet;
                            ui.close_menu();
                        }
                        if ui.button("Color Slider").clicked() {
                            self.mid_right_tab = MidRightTab::ColorSlider;
                            ui.close_menu();
                        }
                        ui.separator();
                        let mat_lbl = if self.show_material_drawer {
                            "✓ Material [All materials]"
                        } else {
                            "  Material [All materials]"
                        };
                        if ui.button(mat_lbl).clicked() {
                            self.show_material_drawer = !self.show_material_drawer;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Workspace > Reset to Default").clicked() {
                            self.top_right_tab = TopRightTab::Navigator;
                            self.mid_right_tab = MidRightTab::ColorWheel;
                            self.show_material_drawer = false;
                            self.viewport.reset_view(
                                ui.available_size(),
                                self.doc.width,
                                self.doc.height,
                            );
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Help", |ui| {
                        ui.label(
                            RichText::new("ComicCraft — Clip Studio Paint 1:1 Studio Edition")
                                .strong(),
                        );
                        ui.label("Version 0.1.0 • Learning Machines LLC / ArtCraft Suite");
                    });
                });
            });

        // =========================================================================
        // 2. Command Bar (Authentic CSP 28px Icon Toolbar below Menu Bar)
        // =========================================================================
        egui::TopBottomPanel::top("command_bar")
            .exact_height(28.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let btn_sz = Vec2::splat(22.0);
                    let _ = icon_button(
                        ui,
                        StudioIcon::CspLogo,
                        btn_sz,
                        false,
                        "CLIP STUDIO / ArtCraft Hub",
                    );
                    ui.separator();

                    if icon_button(ui, StudioIcon::NewDoc, btn_sz, false, "New (Ctrl+N)").clicked() {
                        self.new_dialog_open = true;
                    }
                    if icon_button(ui, StudioIcon::OpenFolder, btn_sz, false, "Open (Ctrl+O)").clicked()
                    {
                        self.open_file_dialog();
                    }

                    if icon_button(ui, StudioIcon::SaveDisk, btn_sz, false, "Save (Ctrl+S)").clicked()
                    {
                        self.save_current_file();
                    }

                    ui.separator();

                    if icon_button(ui, StudioIcon::UndoArrow, btn_sz, false, "Undo (Ctrl+Z)").clicked()
                        && self.history.can_undo()
                    {
                        self.history.undo(&mut self.doc);
                    }
                    if icon_button(ui, StudioIcon::RedoArrow, btn_sz, false, "Redo (Ctrl+Y)").clicked()
                        && self.history.can_redo()
                    {
                        self.history.redo(&mut self.doc);
                    }

                    ui.separator();

                    if icon_button(
                        ui,
                        StudioIcon::ClearTrash,
                        btn_sz,
                        false,
                        "Delete / Clear (Delete)",
                    )
                    .clicked()
                    {
                        self.history.push_state("Clear", &self.doc);
                        self.doc.clear_active_selection_or_layer();
                    }
                    if icon_button(
                        ui,
                        StudioIcon::FillBucket,
                        btn_sz,
                        false,
                        "Fill (Alt+Delete)",
                    )
                    .clicked()
                    {
                        self.history.push_state("Fill", &self.doc);
                        self.doc
                            .flood_fill(self.doc.width / 2, self.doc.height / 2, &self.config);
                    }
                    if icon_button(
                        ui,
                        StudioIcon::TransformBox,
                        btn_sz,
                        self.viewport.active_transform.is_some(),
                        "Scale / Rotate / Free Transform (Ctrl+T)",
                    )
                    .clicked()
                    {
                        if self.viewport.active_transform.is_some() {
                            self.viewport.commit_transform(&mut self.doc, &mut self.history);
                        } else {
                            self.viewport.start_transform(&mut self.doc);
                        }
                    }

                    ui.separator();

                    if icon_button(
                        ui,
                        StudioIcon::DeselectBox,
                        btn_sz,
                        false,
                        "Deselect (Ctrl+D)",
                    )
                    .clicked()
                    {
                        self.doc.selection.deselect();
                    }
                    if icon_button(
                        ui,
                        StudioIcon::InvertSelection,
                        btn_sz,
                        false,
                        "Invert Selected Area (Ctrl+Shift+I)",
                    )
                    .clicked()
                    {
                        self.doc.selection.invert();
                    }

                    ui.separator();

                    if icon_button(
                        ui,
                        StudioIcon::ZoomOut,
                        btn_sz,
                        false,
                        "Zoom Out (Ctrl+-)",
                    )
                    .clicked()
                    {
                        self.viewport.zoom = (self.viewport.zoom * 0.85).clamp(0.05, 32.0);
                    }
                    if icon_button(ui, StudioIcon::ZoomIn, btn_sz, false, "Zoom In (Ctrl++)").clicked()
                    {
                        self.viewport.zoom = (self.viewport.zoom * 1.15).clamp(0.05, 32.0);
                    }
                    if icon_button(
                        ui,
                        StudioIcon::FitScreen,
                        btn_sz,
                        false,
                        "Fit to Navigator (Ctrl+0)",
                    )
                    .clicked()
                    {
                        self.viewport.reset_view(
                            ui.available_size(),
                            self.doc.width,
                            self.doc.height,
                        );
                    }
                    if icon_button(
                        ui,
                        StudioIcon::FlipH,
                        btn_sz,
                        self.viewport.flip_h,
                        "Flip Horizontal",
                    )
                    .clicked()
                    {
                        self.viewport.flip_h = !self.viewport.flip_h;
                    }
                    if icon_button(
                        ui,
                        StudioIcon::FlipV,
                        btn_sz,
                        self.viewport.flip_v,
                        "Flip Vertical",
                    )
                    .clicked()
                    {
                        self.viewport.flip_v = !self.viewport.flip_v;
                    }

                    ui.separator();

                    if icon_button(
                        ui,
                        StudioIcon::MangaGuides,
                        btn_sz,
                        self.viewport.show_manga_guides,
                        "Snap / Show Manga Manuscript Crop Marks (内枠・トンボ)",
                    )
                    .clicked()
                    {
                        self.viewport.show_manga_guides = !self.viewport.show_manga_guides;
                    }
                    if icon_button(
                        ui,
                        StudioIcon::CanvasGrid,
                        btn_sz,
                        self.viewport.show_grid,
                        "Snap / Show Grid",
                    )
                    .clicked()
                    {
                        self.viewport.show_grid = !self.viewport.show_grid;
                    }
                    if icon_button(
                        ui,
                        StudioIcon::RulerBar,
                        btn_sz,
                        self.viewport.show_rulers,
                        "Show Pixel Ruler Bar (Ctrl+R)",
                    )
                    .clicked()
                    {
                        self.viewport.show_rulers = !self.viewport.show_rulers;
                    }
                    if icon_button(
                        ui,
                        StudioIcon::ToolRuler,
                        btn_sz,
                        self.doc.symmetry_ruler.enabled,
                        "Toggle Symmetry Ruler (Mandala / Reflection)",
                    )
                    .clicked()
                    {
                        self.doc.symmetry_ruler.enabled = !self.doc.symmetry_ruler.enabled;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mat_btn = ui.selectable_label(
                            self.show_material_drawer,
                            RichText::new("◀ Material").size(11.0),
                        );
                        if mat_btn
                            .on_hover_text("Toggle CSP Material Palette Drawer (Screentones & Manga)")
                            .clicked()
                        {
                            self.show_material_drawer = !self.show_material_drawer;
                        }
                        ui.separator();
                        ui.label(
                            RichText::new(format!(
                                "{} ({}×{}px • {:.0}dpi)",
                                self.doc.name, self.doc.width, self.doc.height, self.doc.dpi
                            ))
                            .size(11.0)
                            .color(Color32::from_rgb(175, 185, 200)),
                        );
                    });
                });
            });

        // =========================================================================
        // 3. Bottom Status Bar
        // =========================================================================
        egui::TopBottomPanel::bottom("bottom_status_bar")
            .exact_height(20.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new(&self.status_message)
                            .size(10.5)
                            .color(Color32::from_gray(190)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{:.1}%", self.viewport.zoom * 100.0))
                                .size(10.5)
                                .monospace(),
                        );
                        ui.separator();
                        let (cx, cy) = self.viewport.last_cursor_canvas.unwrap_or((0, 0));
                        ui.label(
                            RichText::new(format!("X:{cx:>4} Y:{cy:>4} px"))
                                .size(10.5)
                                .monospace(),
                        );
                        ui.separator();
                        ui.label(
                            RichText::new(format!(
                                "{} × {} px ({:.0} dpi)",
                                self.doc.width, self.doc.height, self.doc.dpi
                            ))
                            .size(10.5),
                        );
                        ui.separator();
                        ui.label(
                            RichText::new(format!(
                                "{} [{}]",
                                self.config.tool.name(),
                                self.config.active_sub_tool_name()
                            ))
                            .size(10.5)
                            .color(Color32::from_rgb(165, 200, 245)),
                        );
                    });
                });
            });

        // =========================================================================
        // 4A. Leftmost 36px Clip Studio Paint Tool Strip (Separate Panel)
        // =========================================================================
        egui::SidePanel::left("csp_tool_strip")
            .exact_width(36.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(1.0);
                    let tool_btn_sz = Vec2::new(25.0, 23.0);

                    // All 18 Clip Studio Paint Tool Icons grouped with authentic separators
                    let tool_groups: &[&[(ToolKind, StudioIcon, &str)]] = &[
                        // Group 1: Navigation & Operation
                        &[
                            (ToolKind::Zoom, StudioIcon::ToolZoom, "Zoom (Z)"),
                            (ToolKind::Hand, StudioIcon::ToolHand, "Move / Hand (Space)"),
                            (
                                ToolKind::Hand,
                                StudioIcon::ToolOperation,
                                "Operation (Object / Select Layer)",
                            ),
                            (ToolKind::Hand, StudioIcon::ToolMoveLayer, "Move Layer (K)"),
                        ],
                        // Group 2: Selection & Eyedropper
                        &[
                            (
                                ToolKind::SelectionRect,
                                StudioIcon::ToolMarquee,
                                "Rectangle Selection (M)",
                            ),
                            (
                                ToolKind::SelectionEllipse,
                                StudioIcon::ToolEllipse,
                                "Ellipse Selection (M)",
                            ),
                            (
                                ToolKind::SelectionLasso,
                                StudioIcon::ToolLasso,
                                "Lasso Selection (L)",
                            ),
                            (
                                ToolKind::AutoSelect,
                                StudioIcon::ToolAutoSelect,
                                "Auto Select / Magic Wand (W)",
                            ),
                            (
                                ToolKind::Eyedropper,
                                StudioIcon::ToolEyedropper,
                                "Eyedropper (I / Alt)",
                            ),
                        ],
                        // Group 3: Brushes & Inking
                        &[
                            (ToolKind::Pen, StudioIcon::ToolPen, "Pen / Marker (P)"),
                            (ToolKind::Pencil, StudioIcon::ToolPencil, "Pencil / Pastel (P)"),
                            (
                                ToolKind::Brush,
                                StudioIcon::ToolBrush,
                                "Brush [Watercolor / Thick Paint / India Ink] (B)",
                            ),
                            (ToolKind::Airbrush, StudioIcon::ToolAirbrush, "Airbrush (B)"),
                            (
                                ToolKind::Screentone,
                                StudioIcon::ToolDecoration,
                                "Decoration / Screentone Brush (B)",
                            ),
                        ],
                        // Group 4: Eraser & Blend
                        &[
                            (ToolKind::Eraser, StudioIcon::ToolEraser, "Eraser (E)"),
                            (ToolKind::Brush, StudioIcon::ToolBlend, "Blend / Blur (J)"),
                        ],
                        // Group 5: Fill & Gradient
                        &[
                            (ToolKind::Fill, StudioIcon::ToolFill, "Fill Bucket (G)"),
                            (ToolKind::CloseAndFill, StudioIcon::ToolCloseAndFill, "Close and Fill (G)"),
                            (ToolKind::Fill, StudioIcon::ToolGradient, "Gradient (G)"),
                        ],
                        // Group 6: Figure, Comic Frame, Ruler, Balloon, Correct Line
                        &[
                            (
                                ToolKind::Figure,
                                StudioIcon::ToolFigure,
                                "Figure [Direct Draw / Speed Lines] (U)",
                            ),
                            (
                                ToolKind::ComicFrame,
                                StudioIcon::ToolComicFrame,
                                "Frame Border [Comic Panels] (U)",
                            ),
                            (
                                ToolKind::Ruler,
                                StudioIcon::ToolRuler,
                                "Ruler [Symmetry / Linear] (U)",
                            ),
                            (
                                ToolKind::SpeechBalloon,
                                StudioIcon::ToolBalloon,
                                "Text / Speech Balloon (T)",
                            ),
                            (
                                ToolKind::Pen,
                                StudioIcon::ToolCorrectLine,
                                "Correct Line (Y)",
                            ),
                        ],
                    ];

                    for group in tool_groups {
                        for &(tool, icon, tip) in *group {
                            // Highlight the canonical tool icon
                            let is_canonical = matches!(
                                (tool, icon),
                                (ToolKind::Zoom, StudioIcon::ToolZoom)
                                    | (ToolKind::Hand, StudioIcon::ToolHand)
                                    | (ToolKind::SelectionRect, StudioIcon::ToolMarquee)
                                    | (ToolKind::SelectionLasso, StudioIcon::ToolLasso)
                                    | (ToolKind::Eyedropper, StudioIcon::ToolEyedropper)
                                    | (ToolKind::Pen, StudioIcon::ToolPen)
                                    | (ToolKind::Pencil, StudioIcon::ToolPencil)
                                    | (ToolKind::Brush, StudioIcon::ToolBrush)
                                    | (ToolKind::Airbrush, StudioIcon::ToolAirbrush)
                                    | (ToolKind::Screentone, StudioIcon::ToolDecoration)
                                    | (ToolKind::Eraser, StudioIcon::ToolEraser)
                                    | (ToolKind::Fill, StudioIcon::ToolFill)
                                    | (ToolKind::Figure, StudioIcon::ToolFigure)
                                    | (ToolKind::ComicFrame, StudioIcon::ToolComicFrame)
                                    | (ToolKind::Ruler, StudioIcon::ToolRuler)
                                    | (ToolKind::SpeechBalloon, StudioIcon::ToolBalloon)
                            );
                            let selected = self.config.tool == tool
                                && is_canonical
                                && !self.is_transparent_color;
                            if icon_button(ui, icon, tool_btn_sz, selected, tip).clicked() {
                                self.config.tool = tool;
                                self.is_transparent_color = false;
                                self.sub_tool_category = 0;
                                let default_st = match tool {
                                    ToolKind::Pen => Some(SubTool::GPen),
                                    ToolKind::Pencil => Some(SubTool::RealPencil),
                                    ToolKind::Brush => {
                                        if icon == StudioIcon::ToolBlend {
                                            Some(SubTool::TransparentWatercolor)
                                        } else {
                                            Some(SubTool::DenseWatercolor)
                                        }
                                    }
                                    ToolKind::Airbrush => Some(SubTool::AirbrushSoft),
                                    ToolKind::Eraser => Some(SubTool::HardEraser),
                                    ToolKind::Figure => Some(SubTool::FigureRect),
                                    ToolKind::Ruler => Some(SubTool::SymmetryRuler),
                                    _ => None,
                                };
                                if let Some(st) = default_st {
                                    self.config.select_sub_tool(st);
                                    self.hardness_step = ((self.config.hardness * 5.0).round() as usize)
                                        .saturating_sub(1)
                                        .min(4);
                                }
                            }
                        }
                        ui.separator();
                    }

                    // Bottom CSP Dual Color & Transparent Swatch Module
                    let box_size = Vec2::new(28.0, 38.0);
                    let (rect, resp) = ui.allocate_exact_size(box_size, Sense::click());
                    let painter = ui.painter_at(rect);

                    let p_rect = Rect::from_min_size(rect.min, Vec2::splat(16.0));
                    let s_rect =
                        Rect::from_min_size(rect.min + Vec2::new(10.0, 7.0), Vec2::splat(16.0));
                    let t_rect = Rect::from_min_size(
                        rect.min + Vec2::new(0.0, 26.0),
                        Vec2::new(26.0, 10.0),
                    );

                    // Sub color
                    let s_col = Color32::from_rgb(
                        self.config.secondary_color[0],
                        self.config.secondary_color[1],
                        self.config.secondary_color[2],
                    );
                    painter.rect_filled(s_rect, 1.0, s_col);
                    painter.rect_stroke(s_rect, 1.0, Stroke::new(1.0_f32, Color32::from_gray(75)));

                    // Main color
                    let p_col = Color32::from_rgb(
                        self.config.primary_color[0],
                        self.config.primary_color[1],
                        self.config.primary_color[2],
                    );
                    painter.rect_filled(p_rect, 1.0, p_col);
                    let p_stroke = if !self.is_transparent_color {
                        Color32::from_rgb(55, 145, 255)
                    } else {
                        Color32::from_gray(90)
                    };
                    painter.rect_stroke(p_rect, 1.0, Stroke::new(1.5_f32, p_stroke));

                    // Transparent checkerboard swatch
                    for cy in 0..2 {
                        for cx in 0..6 {
                            let chk = Rect::from_min_size(
                                t_rect.min + Vec2::new(cx as f32 * 4.3, cy as f32 * 5.0),
                                Vec2::new(4.3, 5.0),
                            );
                            let fill = if (cx + cy) % 2 == 0 {
                                Color32::WHITE
                            } else {
                                Color32::from_gray(175)
                            };
                            painter.rect_filled(chk, 0.0, fill);
                        }
                    }
                    let t_stroke = if self.is_transparent_color {
                        Color32::from_rgb(55, 145, 255)
                    } else {
                        Color32::from_gray(90)
                    };
                    painter.rect_stroke(t_rect, 1.0, Stroke::new(1.2_f32, t_stroke));

                    if resp.clicked() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            if t_rect.contains(pos) {
                                self.is_transparent_color = true;
                                self.config.tool = ToolKind::Eraser;
                            } else if p_rect.contains(pos) {
                                self.is_transparent_color = false;
                                if self.config.tool == ToolKind::Eraser {
                                    self.config.tool = ToolKind::Pen;
                                }
                            } else if s_rect.contains(pos) {
                                std::mem::swap(
                                    &mut self.config.primary_color,
                                    &mut self.config.secondary_color,
                                );
                                self.is_transparent_color = false;
                            }
                        }
                    }
                });
            });

        // =========================================================================
        // 4B. Left Palette Dock: Sub Tool + Tool Property + Brush Size
        // =========================================================================
        egui::SidePanel::left("left_palettes_dock")
            .default_width(256.0)
            .width_range(220.0..=340.0)
            .show(ctx, |ui| {
                // --- PALETTE 1: SUB TOOL ---
                let mut st_tab = SubToolTab::SubTool;
                let st_title = format!("Sub Tool [{}]", self.config.tool.name());
                csp_palette_tabs(ui, &mut st_tab, &[(SubToolTab::SubTool, &st_title)]);

                let categories: &[(&str, &[SubTool])] = match self.config.tool {
                    ToolKind::Pen => &[
                        (
                            "Pen",
                            &[
                                SubTool::GPen,
                                SubTool::RealGPen,
                                SubTool::MappingPen,
                                SubTool::KaburaPen,
                                SubTool::Calligraphy,
                                SubTool::TexturedPen,
                            ],
                        ),
                        (
                            "Marker",
                            &[SubTool::FeltPen, SubTool::Millipen, SubTool::SignPen],
                        ),
                    ],
                    ToolKind::Pencil => &[
                        (
                            "Pencil",
                            &[
                                SubTool::RealPencil,
                                SubTool::DesignPencil,
                                SubTool::RoughPencil,
                                SubTool::MechanicalPencil,
                                SubTool::ColoredPencil,
                            ],
                        ),
                        (
                            "Pastel",
                            &[
                                SubTool::Pastel,
                                SubTool::Charcoal,
                                SubTool::OilPastel,
                                SubTool::Chalk,
                            ],
                        ),
                    ],
                    ToolKind::Brush => &[
                        (
                            "Watercolor",
                            &[
                                SubTool::DenseWatercolor,
                                SubTool::OpaqueWatercolor,
                                SubTool::TransparentWatercolor,
                            ],
                        ),
                        ("Thick paint", &[SubTool::OilPaint, SubTool::Gouache]),
                        ("India ink", &[SubTool::IndiaInk, SubTool::DryBrush]),
                    ],
                    ToolKind::Airbrush => &[(
                        "Airbrush",
                        &[
                            SubTool::AirbrushSoft,
                            SubTool::AirbrushHard,
                            SubTool::AirbrushSpray,
                        ],
                    )],
                    ToolKind::Eraser => &[(
                        "Eraser",
                        &[
                            SubTool::HardEraser,
                            SubTool::SoftEraser,
                            SubTool::KneadedEraser,
                            SubTool::RoughEraser,
                            SubTool::VectorEraser,
                        ],
                    )],
                    ToolKind::Figure => &[
                        (
                            "Direct draw",
                            &[
                                SubTool::FigureRect,
                                SubTool::FigureEllipse,
                                SubTool::FigureLine,
                            ],
                        ),
                        (
                            "Speed lines",
                            &[
                                SubTool::FocusLines,
                                SubTool::StreamLines,
                            ],
                        ),
                    ],
                    ToolKind::Ruler => &[(
                        "Ruler",
                        &[
                            SubTool::SymmetryRuler,
                            SubTool::PerspectiveRuler,
                            SubTool::LinearRuler,
                        ],
                    )],
                    _ => &[("Default", &[SubTool::GPen])],
                };

                // Category Sub-Tabs (CSP style)
                ui.horizontal(|ui| {
                    for (i, &(cat_name, cat_tools)) in categories.iter().enumerate() {
                        let active = self.sub_tool_category == i;
                        if ui
                            .selectable_label(active, RichText::new(cat_name).size(11.0))
                            .clicked()
                        {
                            self.sub_tool_category = i;
                            if let Some(&first_st) = cat_tools.first() {
                                self.config.select_sub_tool(first_st);
                                self.hardness_step = ((self.config.hardness * 5.0).round() as usize)
                                    .saturating_sub(1)
                                    .min(4);
                            }
                        }
                    }
                });

                let active_cat_idx = self
                    .sub_tool_category
                    .min(categories.len().saturating_sub(1));
                let mut sub_tools: Vec<SubTool> = categories[active_cat_idx].1.to_vec();
                for c in &self.config.custom_sub_tools {
                    if c.tool_kind == self.config.tool {
                        sub_tools.push(SubTool::Custom(c.id));
                    }
                }

                // Sub-tool list with live S-curve stroke previews
                let row_icon = match self.config.tool {
                    ToolKind::Pencil => StudioIcon::ToolPencil,
                    ToolKind::Brush => StudioIcon::ToolBrush,
                    ToolKind::Airbrush => StudioIcon::ToolAirbrush,
                    ToolKind::Eraser => StudioIcon::ToolEraser,
                    ToolKind::Figure => StudioIcon::ToolFigure,
                    ToolKind::Ruler => StudioIcon::ToolRuler,
                    _ => StudioIcon::ToolPen,
                };

                egui::ScrollArea::vertical()
                    .id_salt("sub_tool_scroll")
                    .max_height(198.0)
                    .show(ui, |ui| {
                        ui.push_id("sub_tool_rows", |ui| {
                            for st in sub_tools {
                                let selected = self.config.sub_tool == st;
                                let st_preset = self.config.preset_for(self.config.tool, st);
                                let disp_size = if selected {
                                    self.config.size
                                } else {
                                    st_preset.default_size
                                };
                                let st_label = self.config.sub_tool_display_name(st);
                                let is_custom = matches!(st, SubTool::Custom(_));

                                let row_w = ui.available_width();
                                let (r_rect, r_resp) =
                                    ui.allocate_exact_size(Vec2::new(row_w, 22.0), Sense::click());
                                let painter = ui.painter_at(r_rect);

                                let bg = if selected {
                                    Color32::from_rgb(38, 108, 212)
                                } else if r_resp.hovered() {
                                    Color32::from_rgb(52, 56, 63)
                                } else {
                                    Color32::from_rgb(35, 37, 41)
                                };
                                painter.rect_filled(r_rect, 1.5, bg);

                                let icon_r = Rect::from_min_size(
                                    r_rect.left_top() + Vec2::new(3.0, 2.0),
                                    Vec2::splat(18.0),
                                );
                                draw_icon(
                                    &painter,
                                    icon_r,
                                    row_icon,
                                    if selected {
                                        Color32::WHITE
                                    } else if is_custom {
                                        Color32::from_rgb(130, 200, 255)
                                    } else {
                                        Color32::from_gray(200)
                                    },
                                );

                                painter.text(
                                    r_rect.left_center() + Vec2::new(25.0, 0.0),
                                    Align2::LEFT_CENTER,
                                    &st_label,
                                    FontId::proportional(11.5),
                                    Color32::WHITE,
                                );

                                // Live S-curve stroke preview in the right half of the row
                                let prev_rect = Rect::from_min_max(
                                    Pos2::new(r_rect.right() - 78.0, r_rect.top() + 3.0),
                                    Pos2::new(r_rect.right() - 30.0, r_rect.bottom() - 3.0),
                                );
                                draw_stroke_preview(
                                    &painter,
                                    prev_rect,
                                    disp_size,
                                    if selected {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_gray(180)
                                    },
                                );

                                // Right-aligned brush size badge
                                let sz_txt = if disp_size < 3.0 {
                                    format!("{:.1}", disp_size)
                                } else {
                                    format!("{:.0}", disp_size)
                                };
                                painter.text(
                                    r_rect.right_center() - Vec2::new(5.0, 0.0),
                                    Align2::RIGHT_CENTER,
                                    sz_txt,
                                    FontId::monospace(10.0),
                                    Color32::from_gray(215),
                                );

                                if r_resp.clicked() {
                                    self.config.select_sub_tool(st);
                                    self.hardness_step =
                                        ((self.config.hardness * 5.0).round() as usize)
                                            .saturating_sub(1)
                                            .min(4);
                                }
                            }
                        });
                    });

                // Sub Tool bottom action strip (Import .sut / Tip, Duplicate, Reset / Delete)
                ui.horizontal(|ui| {
                    let b_sz = Vec2::splat(19.0);
                    if icon_button(
                        ui,
                        StudioIcon::OpenFolder,
                        b_sz,
                        false,
                        "Import Sub Tool (.sut / Brush Tip Image / .ccbrush)...",
                    )
                    .clicked()
                    {
                        self.open_import_subtool_dialog();
                    }
                    if icon_button(
                        ui,
                        StudioIcon::DuplicateLayer,
                        b_sz,
                        false,
                        "Create Custom Copy of Currently Selected Sub Tool",
                    )
                    .clicked()
                    {
                        self.duplicate_active_subtool();
                    }
                    if icon_button(
                        ui,
                        StudioIcon::DeleteTrash,
                        b_sz,
                        false,
                        "Reset Built-in Sub Tool or Delete Selected Custom Sub Tool",
                    )
                    .clicked()
                    {
                        if let SubTool::Custom(cid) = self.config.sub_tool {
                            self.config.custom_sub_tools.retain(|c| c.id != cid);
                            self.save_persisted_custom_subtools();
                            let fallback = match self.config.tool {
                                ToolKind::Pencil => SubTool::RealPencil,
                                ToolKind::Brush => SubTool::DenseWatercolor,
                                ToolKind::Airbrush => SubTool::AirbrushSoft,
                                ToolKind::Eraser => SubTool::HardEraser,
                                _ => SubTool::GPen,
                            };
                            self.config.select_sub_tool(fallback);
                            self.status_message = "Deleted custom sub tool".to_string();
                        } else {
                            self.config.select_sub_tool(self.config.sub_tool);
                            self.status_message = format!(
                                "Reset `{}` to default settings",
                                self.config.active_sub_tool_name()
                            );
                        }
                    }
                    if ui
                        .small_button(RichText::new("+ Import .sut").size(10.0))
                        .on_hover_text("Import Clip Studio Paint Sub Tool (.sut) or custom pencil tip image (.png)")
                        .clicked()
                    {
                        self.open_import_subtool_dialog();
                    }
                });

                ui.add_space(4.0);

                // --- PALETTE 2: TOOL PROPERTY ---
                let tp_title = format!("Tool Property [{}]", self.config.active_sub_tool_name());
                csp_palette_tabs(
                    ui,
                    &mut self.tool_prop_tab,
                    &[
                        (ToolPropTab::ToolProperty, &tp_title),
                        (ToolPropTab::SubToolDetail, "Sub Tool Detail"),
                    ],
                );

                ui.add_space(2.0);
                if self.tool_prop_tab == ToolPropTab::SubToolDetail {
                    let mut pr = self.config.active_preset();
                    let mut changed = false;

                    let mut thick_pct = pr.aspect * 100.0;
                    if csp_bar_slider(ui, "Thickness", &mut thick_pct, 5.0..=100.0, false, 0, " %") {
                        pr.aspect = (thick_pct / 100.0).clamp(0.05, 1.0);
                        changed = true;
                    }
                    if csp_bar_slider(ui, "Direction", &mut pr.angle_deg, 0.0..=360.0, false, 0, "°") {
                        changed = true;
                    }
                    let mut rand_rot = pr.random_angle * 100.0;
                    if csp_bar_slider(ui, "Random angle", &mut rand_rot, 0.0..=100.0, false, 0, " %") {
                        pr.random_angle = (rand_rot / 100.0).clamp(0.0, 1.0);
                        changed = true;
                    }
                    let mut gap_pct = pr.spacing * 100.0;
                    if csp_bar_slider(ui, "Brush interval", &mut gap_pct, 2.0..=50.0, false, 1, " %") {
                        pr.spacing = (gap_pct / 100.0).clamp(0.02, 0.50);
                        changed = true;
                    }
                    let mut sz_min_pct = pr.size_min * 100.0;
                    if csp_bar_slider(ui, "Min size (Pressure)", &mut sz_min_pct, 0.0..=100.0, false, 0, " %") {
                        pr.size_min = (sz_min_pct / 100.0).clamp(0.0, 1.0);
                        changed = true;
                    }
                    let mut op_min_pct = pr.opacity_min * 100.0;
                    if csp_bar_slider(ui, "Min opacity", &mut op_min_pct, 0.0..=100.0, false, 0, " %") {
                        pr.opacity_min = (op_min_pct / 100.0).clamp(0.0, 1.0);
                        changed = true;
                    }
                    let (g_scale, mut g_str) = match pr.texture {
                        TipTexture::Grain { scale, strength } => (scale, strength * 100.0),
                        _ => (1.5, 0.0),
                    };
                    if csp_bar_slider(ui, "Texture density", &mut g_str, 0.0..=100.0, false, 0, " %") {
                        pr.texture = if g_str <= 0.5 {
                            TipTexture::None
                        } else {
                            TipTexture::Grain {
                                scale: g_scale,
                                strength: (g_str / 100.0).clamp(0.01, 1.0),
                            }
                        };
                        changed = true;
                    }
                    let mut we_pct = pr.water_edge * 100.0;
                    if csp_bar_slider(ui, "Watercolor edge", &mut we_pct, 0.0..=100.0, false, 0, " %") {
                        pr.water_edge = (we_pct / 100.0).clamp(0.0, 1.0);
                        changed = true;
                    }

                    if changed {
                        if let SubTool::Custom(cid) = self.config.sub_tool {
                            if let Some(c) = self.config.custom_sub_tools.iter_mut().find(|c| c.id == cid) {
                                c.preset = pr;
                            }
                            self.save_persisted_custom_subtools();
                        } else {
                            self.config.preset_override = Some(pr);
                        }
                    }
                } else {
                match self.config.tool {
                    ToolKind::ComicFrame => {
                        csp_bar_slider(
                            ui,
                            "Horizontal gutter",
                            &mut self.doc.frame_manager.gutter_h,
                            5.0..=60.0,
                            false,
                            0,
                            " px",
                        );
                        csp_bar_slider(
                            ui,
                            "Vertical gutter",
                            &mut self.doc.frame_manager.gutter_v,
                            5.0..=80.0,
                            false,
                            0,
                            " px",
                        );
                        csp_bar_slider(
                            ui,
                            "Line width",
                            &mut self.doc.frame_manager.border_width,
                            1.0..=12.0,
                            false,
                            1,
                            " px",
                        );
                        ui.horizontal(|ui| {
                            if ui.button("Split H").clicked() {
                                self.history.push_state("Split Frame H", &self.doc);
                                let cy = self.doc.height as f32 * 0.5;
                                self.doc.frame_manager.split_horizontal(0, cy);
                                self.doc.render_comic_panels([20, 20, 25, 255]);
                            }
                            if ui.button("Split V").clicked() {
                                self.history.push_state("Split Frame V", &self.doc);
                                let cx = self.doc.width as f32 * 0.5;
                                self.doc.frame_manager.split_vertical(0, cx);
                                self.doc.render_comic_panels([20, 20, 25, 255]);
                            }
                            if ui.button("Render Borders").clicked() {
                                self.history.push_state("Render Borders", &self.doc);
                                self.doc.render_comic_panels([20, 20, 25, 255]);
                            }
                        });
                    }
                    ToolKind::SpeechBalloon => {
                        ui.horizontal(|ui| {
                            if ui.button("Oval Balloon").clicked() {
                                self.history.push_state("Add Balloon", &self.doc);
                                let mut b = SpeechBalloon::default();
                                b.shape = BalloonShape::Ellipse;
                                self.doc.add_speech_balloon(&b);
                            }
                            if ui.button("Rounded Rect Balloon").clicked() {
                                self.history.push_state("Add Balloon", &self.doc);
                                let mut b = SpeechBalloon::default();
                                b.shape = BalloonShape::RoundedRect;
                                self.doc.add_speech_balloon(&b);
                            }
                        });
                    }
                    ToolKind::Screentone => {
                        csp_bar_slider(
                            ui,
                            "Number of screen frequency",
                            &mut self.screentone_config.frequency,
                            20.0..=85.0,
                            false,
                            1,
                            " L",
                        );
                        let mut dens_pct = self.screentone_config.density * 100.0;
                        if csp_bar_slider(
                            ui,
                            "Density",
                            &mut dens_pct,
                            5.0..=90.0,
                            false,
                            0,
                            " %",
                        ) {
                            self.screentone_config.density = dens_pct / 100.0;
                        }
                        csp_bar_slider(
                            ui,
                            "Angle",
                            &mut self.screentone_config.angle_deg,
                            0.0..=90.0,
                            false,
                            0,
                            "°",
                        );
                        if ui.button("Apply Screentone Layer").clicked() {
                            self.history.push_state("Apply Screentone", &self.doc);
                            let tiles = generate_screentone_tile(
                                self.doc.width,
                                self.doc.height,
                                &self.screentone_config,
                            );
                            let l_idx = self.doc.add_layer(format!(
                                "Tone {:.0}L {:.0}%",
                                self.screentone_config.frequency,
                                self.screentone_config.density * 100.0
                            ));
                            if let Some(l) = self.doc.layers.get_mut(l_idx) {
                                l.pixels = tiles;
                                l.blend_mode = BlendMode::Multiply;
                            }
                            self.doc.mark_full_dirty();
                        }
                    }
                    ToolKind::Fill => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Refer multiple:").size(11.0));
                            ui.selectable_value(
                                &mut self.config.fill_mode,
                                FillReferenceMode::ReferAllLayers,
                                "All",
                            );
                            ui.selectable_value(
                                &mut self.config.fill_mode,
                                FillReferenceMode::ReferActiveLayer,
                                "Editing",
                            );
                            ui.selectable_value(
                                &mut self.config.fill_mode,
                                FillReferenceMode::ReferReferenceLayer,
                                "Ref (Lighthouse)",
                            );
                        });
                        let mut tol = self.config.fill_tolerance as f32;
                        if csp_bar_slider(ui, "Tolerance", &mut tol, 0.0..=100.0, false, 0, "") {
                            self.config.fill_tolerance = tol.round() as u8;
                        }
                        let mut gap = self.config.fill_close_gap as f32;
                        if csp_bar_slider(ui, "Close gap", &mut gap, 0.0..=30.0, false, 0, " px") {
                            self.config.fill_close_gap = gap.round() as u32;
                        }
                        let mut scale = self.config.fill_area_scaling as f32;
                        if csp_bar_slider(ui, "Area scaling", &mut scale, -20.0..=20.0, false, 0, " px") {
                            self.config.fill_area_scaling = scale.round() as i32;
                        }
                    }
                    ToolKind::CloseAndFill => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Target lines:").size(11.0));
                            ui.selectable_value(
                                &mut self.config.fill_mode,
                                FillReferenceMode::ReferAllLayers,
                                "All",
                            );
                            ui.selectable_value(
                                &mut self.config.fill_mode,
                                FillReferenceMode::ReferActiveLayer,
                                "Editing",
                            );
                            ui.selectable_value(
                                &mut self.config.fill_mode,
                                FillReferenceMode::ReferReferenceLayer,
                                "Ref (Lighthouse)",
                            );
                        });
                        let mut gap = self.config.fill_close_gap as f32;
                        if csp_bar_slider(ui, "Close gap", &mut gap, 0.0..=30.0, false, 0, " px") {
                            self.config.fill_close_gap = gap.round() as u32;
                        }
                        let mut scale = self.config.fill_area_scaling as f32;
                        if csp_bar_slider(ui, "Area scaling", &mut scale, -20.0..=20.0, false, 0, " px") {
                            self.config.fill_area_scaling = scale.round() as i32;
                        }
                        ui.label(
                            RichText::new("Draw a lasso around the lineart area to fill enclosed cavities.")
                                .size(10.5)
                                .color(Color32::from_gray(160)),
                        );
                    }
                    ToolKind::AutoSelect => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Refer multiple:").size(11.0));
                            ui.selectable_value(
                                &mut self.config.fill_mode,
                                FillReferenceMode::ReferAllLayers,
                                "All",
                            );
                            ui.selectable_value(
                                &mut self.config.fill_mode,
                                FillReferenceMode::ReferActiveLayer,
                                "Editing",
                            );
                            ui.selectable_value(
                                &mut self.config.fill_mode,
                                FillReferenceMode::ReferReferenceLayer,
                                "Ref (Lighthouse)",
                            );
                        });
                        let mut tol = self.config.wand_tolerance as f32;
                        if csp_bar_slider(ui, "Tolerance", &mut tol, 0.0..=100.0, false, 0, "") {
                            self.config.wand_tolerance = tol.round() as u8;
                        }
                        ui.checkbox(&mut self.config.wand_contiguous, "Contiguous (Follow adjacent)");
                        let mut gap = self.config.wand_close_gap as f32;
                        if csp_bar_slider(ui, "Close gap", &mut gap, 0.0..=30.0, false, 0, " px") {
                            self.config.wand_close_gap = gap.round() as u32;
                        }
                        let mut scale = self.config.wand_area_scaling as f32;
                        if csp_bar_slider(ui, "Area scaling", &mut scale, -20.0..=20.0, false, 0, " px") {
                            self.config.wand_area_scaling = scale.round() as i32;
                        }
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Selection mode:").size(11.0));
                            ui.selectable_value(&mut self.config.selection_op, comiccraft_core::SelectionOp::Replace, "New");
                            ui.selectable_value(&mut self.config.selection_op, comiccraft_core::SelectionOp::Add, "Add (+)");
                            ui.selectable_value(&mut self.config.selection_op, comiccraft_core::SelectionOp::Subtract, "Sub (-)");
                            ui.selectable_value(&mut self.config.selection_op, comiccraft_core::SelectionOp::Intersect, "Inter");
                        });
                    }
                    ToolKind::SelectionRect | ToolKind::SelectionEllipse | ToolKind::SelectionLasso => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Selection mode:").size(11.0));
                            ui.selectable_value(&mut self.config.selection_op, comiccraft_core::SelectionOp::Replace, "New");
                            ui.selectable_value(&mut self.config.selection_op, comiccraft_core::SelectionOp::Add, "Add (+)");
                            ui.selectable_value(&mut self.config.selection_op, comiccraft_core::SelectionOp::Subtract, "Sub (-)");
                            ui.selectable_value(&mut self.config.selection_op, comiccraft_core::SelectionOp::Intersect, "Inter");
                        });
                    }
                    ToolKind::Ruler => {
                        if self.config.sub_tool == SubTool::PerspectiveRuler {
                            ui.checkbox(&mut self.doc.perspective_ruler.enabled, "Enable Perspective Ruler");
                            ui.checkbox(&mut self.doc.perspective_ruler.snap_to_perspective, "Snap to Perspective Ruler");
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Perspective:").size(11.0));
                                ui.selectable_value(&mut self.doc.perspective_ruler.kind, comiccraft_core::PerspectiveKind::OnePoint, "1-Point");
                                ui.selectable_value(&mut self.doc.perspective_ruler.kind, comiccraft_core::PerspectiveKind::TwoPoint, "2-Point");
                                ui.selectable_value(&mut self.doc.perspective_ruler.kind, comiccraft_core::PerspectiveKind::ThreePoint, "3-Point");
                            });
                            csp_bar_slider(
                                ui,
                                "Eye level (Horizon)",
                                &mut self.doc.perspective_ruler.eye_level_y,
                                0.0..=(self.doc.height as f32),
                                false,
                                0,
                                " px",
                            );
                            csp_bar_slider(
                                ui,
                                "Vanishing Point 1 (X)",
                                &mut self.doc.perspective_ruler.vp1_x,
                                -1000.0..=(self.doc.width as f32 + 1000.0),
                                false,
                                0,
                                " px",
                            );
                            if matches!(self.doc.perspective_ruler.kind, comiccraft_core::PerspectiveKind::TwoPoint | comiccraft_core::PerspectiveKind::ThreePoint) {
                                csp_bar_slider(
                                    ui,
                                    "Vanishing Point 2 (X)",
                                    &mut self.doc.perspective_ruler.vp2_x,
                                    -1000.0..=(self.doc.width as f32 + 2000.0),
                                    false,
                                    0,
                                    " px",
                                );
                            }
                            let mut divs_f = self.doc.perspective_ruler.guide_divisions as f32;
                            if csp_bar_slider(ui, "Grid density", &mut divs_f, 4.0..=24.0, false, 0, " rays") {
                                self.doc.perspective_ruler.guide_divisions = divs_f.round() as u32;
                            }
                            ui.horizontal(|ui| {
                                if ui.button("Center Horizon").clicked() {
                                    let cy = self.doc.height as f32 * 0.5;
                                    self.doc.perspective_ruler.eye_level_y = cy;
                                    self.doc.perspective_ruler.vp1_y = cy;
                                    self.doc.perspective_ruler.vp2_y = cy;
                                    self.doc.perspective_ruler.enabled = true;
                                    self.doc.mark_full_dirty();
                                }
                            });
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Tip: Click canvas to set horizon.\nDraw with any Pen/Brush/Pencil to snap perspective lines!")
                                    .size(10.5)
                                    .color(Color32::from_gray(175)),
                            );
                        } else {
                            ui.checkbox(&mut self.doc.symmetry_ruler.enabled, "Enable Symmetry Ruler");
                            let mut num_lines_f = self.doc.symmetry_ruler.lines as f32;
                            if csp_bar_slider(
                                ui,
                                "Number of lines",
                                &mut num_lines_f,
                                2.0..=16.0,
                                false,
                                0,
                                "",
                            ) {
                                self.doc.symmetry_ruler.lines = num_lines_f.round() as u32;
                            }
                            csp_bar_slider(
                                ui,
                                "Angle",
                                &mut self.doc.symmetry_ruler.angle_deg,
                                0.0..=360.0,
                                false,
                                0,
                                "°",
                            );
                            ui.checkbox(&mut self.doc.symmetry_ruler.line_symmetric, "Line symmetry (Mirror)");
                            ui.horizontal(|ui| {
                                if ui.button("Center on Canvas").clicked() {
                                    self.doc.symmetry_ruler.center_x = self.doc.width as f32 * 0.5;
                                    self.doc.symmetry_ruler.center_y = self.doc.height as f32 * 0.5;
                                    self.doc.symmetry_ruler.enabled = true;
                                    self.doc.mark_full_dirty();
                                }
                            });
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Tip: Click canvas with Ruler tool to reposition center.\nDraw with any Pen/Brush to mirror across symmetry axes!")
                                    .size(10.5)
                                    .color(Color32::from_gray(175)),
                            );
                        }
                    }
                    ToolKind::Figure => {
                        match self.config.sub_tool {
                            SubTool::FocusLines => {
                                csp_bar_slider(
                                    ui,
                                    "Inner radius",
                                    &mut self.config.focus_lines.inner_radius,
                                    10.0..=1000.0,
                                    false,
                                    0,
                                    " px",
                                );
                                csp_bar_slider(
                                    ui,
                                    "Outer radius",
                                    &mut self.config.focus_lines.outer_radius,
                                    50.0..=3000.0,
                                    false,
                                    0,
                                    " px",
                                );
                                let mut count_f = self.config.focus_lines.count as f32;
                                if csp_bar_slider(
                                    ui,
                                    "Number of lines",
                                    &mut count_f,
                                    10.0..=400.0,
                                    false,
                                    0,
                                    "",
                                ) {
                                    self.config.focus_lines.count = count_f.round() as u32;
                                }
                                csp_bar_slider(
                                    ui,
                                    "Line thickness",
                                    &mut self.config.focus_lines.thickness,
                                    0.5..=25.0,
                                    false,
                                    1,
                                    " px",
                                );
                                let mut jit_pct = self.config.focus_lines.jitter * 100.0;
                                if csp_bar_slider(
                                    ui,
                                    "Disarray / Jitter",
                                    &mut jit_pct,
                                    0.0..=100.0,
                                    false,
                                    0,
                                    " %",
                                ) {
                                    self.config.focus_lines.jitter = jit_pct / 100.0;
                                }
                                if ui.button("Render Focus Lines to Layer").clicked() {
                                    self.history.push_state("Radiation Focus Lines", &self.doc);
                                    self.config.focus_lines.color = self.config.primary_color;
                                    let idx = self.doc.active_layer_idx;
                                    let sel = &self.doc.selection;
                                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                                        render_focus_lines(layer, sel, &self.config.focus_lines);
                                    }
                                    self.doc.mark_full_dirty();
                                }
                            }
                            SubTool::StreamLines => {
                                csp_bar_slider(
                                    ui,
                                    "Angle",
                                    &mut self.config.stream_lines.angle_deg,
                                    0.0..=360.0,
                                    false,
                                    0,
                                    "°",
                                );
                                csp_bar_slider(
                                    ui,
                                    "Line length",
                                    &mut self.config.stream_lines.length,
                                    50.0..=2000.0,
                                    false,
                                    0,
                                    " px",
                                );
                                let mut count_f = self.config.stream_lines.count as f32;
                                if csp_bar_slider(
                                    ui,
                                    "Number of lines",
                                    &mut count_f,
                                    10.0..=300.0,
                                    false,
                                    0,
                                    "",
                                ) {
                                    self.config.stream_lines.count = count_f.round() as u32;
                                }
                                csp_bar_slider(
                                    ui,
                                    "Line thickness",
                                    &mut self.config.stream_lines.thickness,
                                    0.5..=20.0,
                                    false,
                                    1,
                                    " px",
                                );
                                if ui.button("Render Speed Lines to Layer").clicked() {
                                    self.history.push_state("Directional Speed Lines", &self.doc);
                                    self.config.stream_lines.color = self.config.primary_color;
                                    let idx = self.doc.active_layer_idx;
                                    let sel = &self.doc.selection;
                                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                                        render_stream_lines(layer, sel, &self.config.stream_lines);
                                    }
                                    self.doc.mark_full_dirty();
                                }
                            }
                            _ => {
                                csp_bar_slider(
                                    ui,
                                    "Line width",
                                    &mut self.config.size,
                                    0.5..=100.0,
                                    true,
                                    1,
                                    " px",
                                );
                                let mut opac_pct = self.config.opacity * 100.0;
                                if csp_bar_slider(ui, "Opacity", &mut opac_pct, 1.0..=100.0, false, 0, "") {
                                    self.config.opacity = opac_pct / 100.0;
                                }
                                ui.label(
                                    RichText::new("Drag on canvas to place figure preview.\nRelease to draw stroke.")
                                        .size(10.5)
                                        .color(Color32::from_gray(175)),
                                );
                            }
                        }
                    }
                    _ => {
                        // Authentic CSP Bar Sliders for Brush Size, Opacity, Hardness, Anti-aliasing, Stabilization
                        csp_bar_slider(
                            ui,
                            "Brush size",
                            &mut self.config.size,
                            0.5..=300.0,
                            true,
                            1,
                            "",
                        );

                        let mut opac_pct = self.config.opacity * 100.0;
                        if csp_bar_slider(ui, "Opacity", &mut opac_pct, 1.0..=100.0, false, 0, "")
                        {
                            self.config.opacity = opac_pct / 100.0;
                        }

                        // Stepped 5-dot Hardness row (drawn with custom circles)
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Hardness").size(11.0));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let vals = [0.2_f32, 0.4, 0.6, 0.8, 1.0];
                                    for idx in (0..5).rev() {
                                        let sel = self.hardness_step == idx;
                                        let (d_rect, d_resp) =
                                            ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                                        let p = ui.painter_at(d_rect);
                                        if sel {
                                            p.rect_filled(
                                                d_rect,
                                                2.0,
                                                Color32::from_rgb(38, 108, 212),
                                            );
                                        }
                                        let r = 2.0 + idx as f32 * 0.9;
                                        p.circle_filled(
                                            d_rect.center(),
                                            r,
                                            if sel {
                                                Color32::WHITE
                                            } else {
                                                Color32::from_gray(185)
                                            },
                                        );
                                        if d_resp.clicked() {
                                            self.hardness_step = idx;
                                            self.config.hardness = vals[idx];
                                        }
                                    }
                                },
                            );
                        });

                        // Stepped 4-mode Anti-aliasing row (directly drives sub-pixel AA width in brush.rs)
                        self.antialias_level = self.config.antialiasing as usize;
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Anti-aliasing").size(11.0));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let labels = ["Strong", "Mid", "Weak", "None"];
                                    for (rev_i, lbl) in labels.iter().enumerate() {
                                        let lvl = 3 - rev_i;
                                        let sel = self.antialias_level == lvl;
                                        if ui
                                            .selectable_label(sel, RichText::new(*lbl).size(10.0))
                                            .clicked()
                                        {
                                            self.antialias_level = lvl;
                                            self.config.antialiasing = lvl as u8;
                                        }
                                    }
                                },
                            );
                        });

                        let mut stab_f = self.config.stabilization as f32;
                        if csp_bar_slider(
                            ui,
                            "Stabilization",
                            &mut stab_f,
                            0.0..=50.0,
                            false,
                            0,
                            "",
                        ) {
                            self.config.stabilization = stab_f.round() as u32;
                        }
                    }
                }
                }


                ui.add_space(4.0);

                // --- PALETTE 3: BRUSH SIZE (Visual Circle Matrix at Bottom-Left of CSP) ---
                let mut bs_tab = BrushSizeTab::BrushSize;
                csp_palette_tabs(ui, &mut bs_tab, &[(BrushSizeTab::BrushSize, "Brush Size")]);

                let presets: &[f32] = &[
                    0.7, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 12.0, 15.0, 20.0, 30.0, 50.0,
                    100.0, 200.0,
                ];
                let cols = 4;
                let cell_w = ((ui.available_width() - 12.0) / cols as f32).floor();
                let cell_h = 34.0;

                for row in presets.chunks(cols) {
                    ui.horizontal(|ui| {
                        for &sz in row {
                            let is_sel = (self.config.size - sz).abs() < 0.25;
                            let (c_rect, c_resp) =
                                ui.allocate_exact_size(Vec2::new(cell_w, cell_h), Sense::click());
                            let painter = ui.painter_at(c_rect);

                            let bg = if is_sel {
                                Color32::from_rgb(38, 108, 212)
                            } else if c_resp.hovered() {
                                Color32::from_rgb(52, 56, 64)
                            } else {
                                Color32::from_rgb(34, 36, 40)
                            };
                            painter.rect_filled(c_rect, 1.5, bg);
                            painter.rect_stroke(
                                c_rect,
                                1.5,
                                Stroke::new(1.0_f32, Color32::from_rgb(48, 51, 56)),
                            );

                            let circle_center = c_rect.center() - Vec2::new(0.0, 5.0);
                            let vis_r = (sz * 0.35).clamp(0.8, 9.5);
                            painter.circle_filled(circle_center, vis_r, Color32::WHITE);

                            let lbl = if sz < 2.0 {
                                format!("{:.1}", sz)
                            } else {
                                format!("{:.0}", sz)
                            };
                            painter.text(
                                Pos2::new(c_rect.center().x, c_rect.bottom() - 6.5),
                                Align2::CENTER_CENTER,
                                lbl,
                                FontId::monospace(9.5),
                                Color32::from_gray(215),
                            );

                            if c_resp.clicked() {
                                self.config.size = sz;
                            }
                        }
                    });
                }
            });

        // =========================================================================
        // 5A. Optional Collapsible Far-Right Material Drawer (Screentones & Manga)
        // =========================================================================
        if self.show_material_drawer {
            egui::SidePanel::right("csp_material_drawer")
                .default_width(185.0)
                .resizable(true)
                .show(ctx, |ui| {
                    let mut m_tab = 0_usize;
                    csp_palette_tabs(
                        ui,
                        &mut m_tab,
                        &[(0, "Material [Monochromatic]")],
                    );

                    ui.label(
                        RichText::new("Click to drop onto canvas:")
                            .size(10.5)
                            .color(Color32::from_gray(170)),
                    );

                    let tones = [
                        ("Basic 60L 10%", 60.0_f32, 0.10_f32, 45.0_f32),
                        ("Basic 60L 20%", 60.0, 0.20, 45.0),
                        ("Basic 60L 30%", 60.0, 0.30, 45.0),
                        ("Basic 60L 50%", 60.0, 0.50, 45.0),
                        ("Coarse 40L 25%", 40.0, 0.25, 45.0),
                        ("Fine 75L 20%", 75.0, 0.20, 45.0),
                    ];

                    for (name, freq, dens, ang) in tones {
                        if ui.button(RichText::new(format!("▓ {name}")).size(11.0)).clicked() {
                            self.history.push_state("Drop Tone Material", &self.doc);
                            let cfg = ScreentoneConfig {
                                frequency: freq,
                                density: dens,
                                angle_deg: ang,
                            };
                            let tiles =
                                generate_screentone_tile(self.doc.width, self.doc.height, &cfg);
                            let idx = self.doc.add_layer(name);
                            if let Some(l) = self.doc.layers.get_mut(idx) {
                                l.pixels = tiles;
                                l.blend_mode = BlendMode::Multiply;
                            }
                            self.doc.mark_full_dirty();
                        }
                    }

                    ui.separator();
                    ui.label(
                        RichText::new("Manga Balloons & Frames")
                            .size(10.5)
                            .color(Color32::from_gray(170)),
                    );
                    if ui.button("◯ Oval Dialogue Balloon").clicked() {
                        self.history.push_state("Drop Balloon", &self.doc);
                        self.doc.add_speech_balloon(&SpeechBalloon::default());
                    }
                    if ui.button("▭ Rounded Narration Box").clicked() {
                        self.history.push_state("Drop Narration Box", &self.doc);
                        let mut b = SpeechBalloon::default();
                        b.shape = BalloonShape::RoundedRect;
                        b.tail_x = None;
                        b.tail_y = None;
                        self.doc.add_speech_balloon(&b);
                    }
                    if ui.button("⊞ 4-Koma Panel Layout").clicked() {
                        self.history.push_state("4-Koma Panels", &self.doc);
                        let h = self.doc.height as f32;
                        self.doc.frame_manager.split_horizontal(0, h * 0.5);
                        self.doc.render_comic_panels([20, 20, 25, 255]);
                    }
                });
        }

        // =========================================================================
        // 5B. Right Dock (Navigator + Color Wheel + Layer Property + Layer Palette)
        // =========================================================================
        egui::SidePanel::right("right_dock")
            .default_width(292.0)
            .width_range(260.0..=400.0)
            .show(ctx, |ui| {
                let dock_w = ui.available_width();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                    ui.set_max_width(dock_w);
                    // --- PALETTE 1: NAVIGATOR / SUB VIEW / HISTORY ---
                    csp_palette_tabs(
                        ui,
                        &mut self.top_right_tab,
                        &[
                            (TopRightTab::Navigator, "Navigator"),
                            (TopRightTab::SubView, "Sub View"),
                            (TopRightTab::History, "History"),
                        ],
                    );

                    match self.top_right_tab {
                        TopRightTab::Navigator => {
                            let avail_w = ui.available_width().clamp(140.0, 340.0);
                            let nav_h = 116.0;
                            let (well_rect, _) =
                                ui.allocate_exact_size(Vec2::new(avail_w, nav_h), Sense::hover());
                            let well_painter = ui.painter_at(well_rect);
                            well_painter.rect_filled(well_rect, 1.5, Color32::from_rgb(28, 30, 34));

                            let aspect =
                                (self.doc.width as f32 / self.doc.height.max(1) as f32).clamp(0.4, 2.2);
                            let page_h = nav_h - 10.0;
                            let page_w = (page_h * aspect).min(avail_w - 16.0);
                            let p_rect = Rect::from_center_size(
                                well_rect.center(),
                                Vec2::new(page_w, page_h),
                            );
                            let p_resp = ui.interact(
                                p_rect,
                                ui.id().with("nav_page"),
                                Sense::click_and_drag(),
                            );
                            let painter = ui.painter_at(p_rect);

                            painter.rect_filled(p_rect, 1.0, Color32::WHITE);
                            if let Some(tex) = &self.viewport.texture {
                                painter.image(
                                    tex.id(),
                                    p_rect,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            }

                            // Red Viewport Rectangle
                            let half_w = self.doc.width as f32 * 0.5;
                            let half_h = self.doc.height as f32 * 0.5;
                            let tl_canvas =
                                self.viewport.screen_to_canvas(self.viewport.last_rect.left_top());
                            let br_canvas = self
                                .viewport
                                .screen_to_canvas(self.viewport.last_rect.right_bottom());

                            let min_cx =
                                (tl_canvas.x.min(br_canvas.x) + half_w) / self.doc.width as f32;
                            let max_cx =
                                (tl_canvas.x.max(br_canvas.x) + half_w) / self.doc.width as f32;
                            let min_cy =
                                (tl_canvas.y.min(br_canvas.y) + half_h) / self.doc.height as f32;
                            let max_cy =
                                (tl_canvas.y.max(br_canvas.y) + half_h) / self.doc.height as f32;

                            let red_tl = Pos2::new(
                                p_rect.min.x + (min_cx * p_rect.width()).clamp(0.0, p_rect.width()),
                                p_rect.min.y
                                    + (min_cy * p_rect.height()).clamp(0.0, p_rect.height()),
                            );
                            let red_br = Pos2::new(
                                p_rect.min.x + (max_cx * p_rect.width()).clamp(0.0, p_rect.width()),
                                p_rect.min.y
                                    + (max_cy * p_rect.height()).clamp(0.0, p_rect.height()),
                            );
                            painter.rect_stroke(
                                Rect::from_min_max(red_tl, red_br),
                                0.5,
                                Stroke::new(1.5_f32, Color32::from_rgb(235, 45, 45)),
                            );

                            if let Some(ptr) = p_resp.interact_pointer_pos() {
                                let norm_x =
                                    ((ptr.x - p_rect.min.x) / p_rect.width()).clamp(0.0, 1.0);
                                let norm_y =
                                    ((ptr.y - p_rect.min.y) / p_rect.height()).clamp(0.0, 1.0);
                                let target_canvas = Pos2::new(
                                    (norm_x - 0.5) * self.doc.width as f32,
                                    (norm_y - 0.5) * self.doc.height as f32,
                                );
                                let screen_target = self.viewport.canvas_to_screen(target_canvas);
                                let screen_center = self.viewport.last_rect.center();
                                self.viewport.pan +=
                                    screen_center.to_vec2() - screen_target.to_vec2();
                            }

                            // CSP Navigator Row 1: Zoom Bar
                            ui.horizontal(|ui| {
                                let b = Vec2::splat(18.0);
                                if icon_button(ui, StudioIcon::ZoomOut, b, false, "Zoom Out").clicked()
                                {
                                    self.viewport.zoom =
                                        (self.viewport.zoom * 0.85).clamp(0.05, 32.0);
                                }
                                let mut z_pct = self.viewport.zoom * 100.0;
                                if csp_bar_slider_reserved(
                                    ui,
                                    42.0,
                                    "Zoom",
                                    &mut z_pct,
                                    5.0..=800.0,
                                    true,
                                    1,
                                    "%",
                                ) {
                                    self.viewport.zoom = z_pct / 100.0;
                                }
                                if icon_button(ui, StudioIcon::ZoomIn, b, false, "Zoom In").clicked() {
                                    self.viewport.zoom =
                                        (self.viewport.zoom * 1.15).clamp(0.05, 32.0);
                                }
                                if icon_button(ui, StudioIcon::FitScreen, b, false, "Fit").clicked() {
                                    self.viewport.reset_view(
                                        ui.available_size(),
                                        self.doc.width,
                                        self.doc.height,
                                    );
                                }
                            });

                            // CSP Navigator Row 2: Rotation & Flip Bar
                            ui.horizontal(|ui| {
                                let b = Vec2::splat(18.0);
                                if icon_button(ui, StudioIcon::UndoArrow, b, false, "Rotate Left")
                                    .clicked()
                                {
                                    self.viewport.rotation -= 5.0_f32.to_radians();
                                }
                                let mut deg = self.viewport.rotation.to_degrees();
                                if csp_bar_slider_reserved(
                                    ui,
                                    42.0,
                                    "Angle",
                                    &mut deg,
                                    -180.0..=180.0,
                                    false,
                                    1,
                                    "°",
                                ) {
                                    self.viewport.rotation = deg.to_radians();
                                }
                                if icon_button(ui, StudioIcon::RedoArrow, b, false, "Rotate Right")
                                    .clicked()
                                {
                                    self.viewport.rotation += 5.0_f32.to_radians();
                                }
                                if icon_button(
                                    ui,
                                    StudioIcon::FlipH,
                                    b,
                                    self.viewport.flip_h,
                                    "Flip Horizontal",
                                )
                                .clicked()
                                {
                                    self.viewport.flip_h = !self.viewport.flip_h;
                                }
                            });
                        }
                        TopRightTab::SubView => {
                            ui.horizontal(|ui| {
                                if ui.button("Import Reference Image...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                                        .pick_file()
                                    {
                                        if let Ok(img) = image::open(&path) {
                                            let rgba = img.to_rgba8();
                                            let (w, h) = rgba.dimensions();
                                            let ci = ColorImage::from_rgba_unmultiplied(
                                                [w as usize, h as usize],
                                                rgba.as_raw(),
                                            );
                                            self.subview_image = Some(ui.ctx().load_texture(
                                                "subview_ref",
                                                ci,
                                                TextureOptions::LINEAR,
                                            ));
                                            self.subview_path = Some(path);
                                        }
                                    }
                                }
                                if self.subview_image.is_some() && ui.button("Clear").clicked() {
                                    self.subview_image = None;
                                    self.subview_path = None;
                                }
                            });
                            if let Some(tex) = &self.subview_image {
                                let max_w = (ui.available_width() - 8.0).clamp(120.0, 260.0);
                                let tex_sz = tex.size_vec2();
                                let aspect = tex_sz.y / tex_sz.x.max(1.0);
                                let h = (max_w * aspect).min(150.0);
                                let (ref_rect, _) =
                                    ui.allocate_exact_size(Vec2::new(max_w, h), Sense::click());
                                ui.painter_at(ref_rect).image(
                                    tex.id(),
                                    ref_rect,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            } else {
                                ui.label(
                                    RichText::new("No reference image loaded.")
                                        .size(11.0)
                                        .color(Color32::from_gray(145)),
                                );
                            }
                        }
                        TopRightTab::History => {
                            for entry in self.history.undo_entries() {
                                ui.label(
                                    RichText::new(format!("  • {}", entry))
                                        .size(11.0)
                                        .color(Color32::from_rgb(185, 205, 235)),
                                );
                            }
                            ui.label(
                                RichText::new("  ▸ Current State")
                                    .size(11.0)
                                    .strong()
                                    .color(Color32::from_rgb(65, 145, 255)),
                            );
                        }
                    }

                    ui.add_space(3.0);

                    // --- PALETTE 2: COLOR WHEEL / COLOR SET / COLOR SLIDER ---
                    csp_palette_tabs(
                        ui,
                        &mut self.mid_right_tab,
                        &[
                            (MidRightTab::ColorWheel, "Color Wheel"),
                            (MidRightTab::ColorSet, "Color Set"),
                            (MidRightTab::ColorSlider, "Color Slider"),
                        ],
                    );

                    match self.mid_right_tab {
                        MidRightTab::ColorWheel => {
                            ColorWheelWidget::new(
                                &mut self.config.primary_color,
                                &mut self.config.secondary_color,
                                &mut self.is_transparent_color,
                                &mut self.color_wheel_triangle,
                            )
                            .show(ui);

                            // Color History Swatch Row
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "#{:02X}{:02X}{:02X}",
                                        self.config.primary_color[0],
                                        self.config.primary_color[1],
                                        self.config.primary_color[2]
                                    ))
                                    .size(10.5)
                                    .monospace()
                                    .color(Color32::from_gray(200)),
                                );
                                ui.separator();
                                for &rgb in &self.color_history.clone() {
                                    let col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                    let (s_rect, s_resp) =
                                        ui.allocate_exact_size(Vec2::splat(14.0), Sense::click());
                                    ui.painter().rect_filled(s_rect, 1.0, col);
                                    ui.painter().rect_stroke(
                                        s_rect,
                                        1.0,
                                        Stroke::new(1.0_f32, Color32::from_gray(65)),
                                    );
                                    if s_resp.clicked() {
                                        self.config.primary_color = [rgb[0], rgb[1], rgb[2], 255];
                                        self.is_transparent_color = false;
                                    }
                                }
                            });
                        }
                        MidRightTab::ColorSet => {
                            let palettes: &[(&str, &[[u8; 3]])] = &[
                                (
                                    "Standard color set",
                                    &[
                                        [0, 0, 0],
                                        [64, 64, 64],
                                        [128, 128, 128],
                                        [192, 192, 192],
                                        [255, 255, 255],
                                        [128, 0, 0],
                                        [255, 0, 0],
                                        [255, 128, 0],
                                        [255, 255, 0],
                                        [0, 128, 0],
                                        [0, 255, 0],
                                        [0, 255, 255],
                                        [0, 128, 255],
                                        [0, 0, 255],
                                        [128, 0, 255],
                                        [255, 0, 255],
                                    ],
                                ),
                                (
                                    "Manga Monochrome & Inks",
                                    &[
                                        [10, 10, 14],
                                        [35, 35, 40],
                                        [60, 60, 65],
                                        [95, 95, 100],
                                        [140, 140, 145],
                                        [190, 190, 195],
                                        [230, 230, 235],
                                        [255, 255, 255],
                                    ],
                                ),
                                (
                                    "Skin Tones & Portrait",
                                    &[
                                        [255, 240, 230],
                                        [253, 224, 206],
                                        [249, 201, 176],
                                        [232, 168, 138],
                                        [198, 128, 96],
                                        [140, 78, 50],
                                        [90, 48, 30],
                                        [50, 25, 15],
                                    ],
                                ),
                            ];

                            egui::ComboBox::from_id_salt("color_set_cat")
                                .selected_text(
                                    palettes[self.color_set_category.min(palettes.len() - 1)].0,
                                )
                                .show_ui(ui, |ui| {
                                    for (idx, (name, _)) in palettes.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut self.color_set_category,
                                            idx,
                                            *name,
                                        );
                                    }
                                });

                            let swatches =
                                palettes[self.color_set_category.min(palettes.len() - 1)].1;
                            ui.horizontal_wrapped(|ui| {
                                for &rgb in swatches {
                                    let col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                    let (s_rect, resp) =
                                        ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                                    ui.painter().rect_filled(s_rect, 1.0, col);
                                    ui.painter().rect_stroke(
                                        s_rect,
                                        1.0,
                                        Stroke::new(1.0_f32, Color32::from_gray(60)),
                                    );
                                    if resp.clicked() {
                                        self.config.primary_color = [rgb[0], rgb[1], rgb[2], 255];
                                        self.is_transparent_color = false;
                                    }
                                }
                            });
                        }
                        MidRightTab::ColorSlider => {
                            let mut r = self.config.primary_color[0] as f32;
                            let mut g = self.config.primary_color[1] as f32;
                            let mut b = self.config.primary_color[2] as f32;
                            if csp_bar_slider(ui, "R", &mut r, 0.0..=255.0, false, 0, "") {
                                self.config.primary_color[0] = r.round() as u8;
                            }
                            if csp_bar_slider(ui, "G", &mut g, 0.0..=255.0, false, 0, "") {
                                self.config.primary_color[1] = g.round() as u8;
                            }
                            if csp_bar_slider(ui, "B", &mut b, 0.0..=255.0, false, 0, "") {
                                self.config.primary_color[2] = b.round() as u8;
                            }
                            let (mut h, mut s, mut v) = rgb_to_hsv(
                                self.config.primary_color[0],
                                self.config.primary_color[1],
                                self.config.primary_color[2],
                            );
                            let mut h_deg = h * 360.0;
                            let mut s_pct = s * 100.0;
                            let mut v_pct = v * 100.0;
                            if csp_bar_slider(ui, "H", &mut h_deg, 0.0..=360.0, false, 0, "°") {
                                h = h_deg / 360.0;
                                let rgb = hsv_to_rgb(h, s, v);
                                self.config.primary_color = [rgb[0], rgb[1], rgb[2], 255];
                            }
                            if csp_bar_slider(ui, "S", &mut s_pct, 0.0..=100.0, false, 0, "%") {
                                s = s_pct / 100.0;
                                let rgb = hsv_to_rgb(h, s, v);
                                self.config.primary_color = [rgb[0], rgb[1], rgb[2], 255];
                            }
                            if csp_bar_slider(ui, "V", &mut v_pct, 0.0..=100.0, false, 0, "%") {
                                v = v_pct / 100.0;
                                let rgb = hsv_to_rgb(h, s, v);
                                self.config.primary_color = [rgb[0], rgb[1], rgb[2], 255];
                            }
                        }
                    }

                    ui.add_space(3.0);

                    // --- PALETTE 3: LAYER PROPERTY (Authentic CSP Effect Bar above Layer Palette) ---
                    let mut lp_tab = LayerPropTab::LayerProperty;
                    csp_palette_tabs(
                        ui,
                        &mut lp_tab,
                        &[(LayerPropTab::LayerProperty, "Layer Property")],
                    );

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Effect:").size(10.5));
                        let b_sz = Vec2::splat(20.0);
                        if icon_button(
                            ui,
                            StudioIcon::BorderEffect,
                            b_sz,
                            self.layer_border_effect,
                            "Border Effect (Outline Stroke)",
                        )
                        .clicked()
                        {
                            self.layer_border_effect = !self.layer_border_effect;
                        }
                        if icon_button(
                            ui,
                            StudioIcon::ToneGrid,
                            b_sz,
                            self.layer_tone_effect,
                            "Tone (Convert Layer to Manga Halftone Screentone)",
                        )
                        .clicked()
                        {
                            self.layer_tone_effect = !self.layer_tone_effect;
                        }
                        if icon_button(
                            ui,
                            StudioIcon::LayerColorTint,
                            b_sz,
                            self.layer_color_effect,
                            "Layer Color (Tint Sketch Layer Blue)",
                        )
                        .clicked()
                        {
                            self.layer_color_effect = !self.layer_color_effect;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new("Color")
                                    .size(10.5)
                                    .color(Color32::from_gray(190)),
                            );
                            ui.label(
                                RichText::new("Expression:")
                                    .size(10.5)
                                    .color(Color32::from_gray(145)),
                            );
                        });
                    });

                    if self.layer_tone_effect {
                        csp_bar_slider(
                            ui,
                            "Frequency",
                            &mut self.screentone_config.frequency,
                            20.0..=85.0,
                            false,
                            1,
                            " L",
                        );
                    }

                    if let Some(active) = self.doc.layers.get(self.doc.active_layer_idx) {
                        if active.is_vector() {
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Vector:").size(10.5));
                                if ui.button("Simplify").on_hover_text("Simplify vector stroke points (Ramer-Douglas-Peucker)").clicked() {
                                    let idx = self.doc.active_layer_idx;
                                    self.history.push_state("Simplify Vector Line", &self.doc);
                                    self.doc.simplify_vector_layer(idx, 1.2);
                                    self.status_message = "Simplified vector line points.".to_string();
                                }
                                if ui.button("+1px").on_hover_text("Thicken vector lines by +1px").clicked() {
                                    let idx = self.doc.active_layer_idx;
                                    self.history.push_state("Alter Vector Width", &self.doc);
                                    self.doc.adjust_vector_line_width(idx, 1.0);
                                    self.status_message = "Thickened vector lines by +1px.".to_string();
                                }
                                if ui.button("-1px").on_hover_text("Narrow vector lines by -1px").clicked() {
                                    let idx = self.doc.active_layer_idx;
                                    self.history.push_state("Alter Vector Width", &self.doc);
                                    self.doc.adjust_vector_line_width(idx, -1.0);
                                    self.status_message = "Narrowed vector lines by -1px.".to_string();
                                }
                            });
                        }
                    }

                    ui.add_space(3.0);

                    // --- PALETTE 4: LAYER PALETTE ---
                    csp_palette_tabs(
                        ui,
                        &mut self.layer_dock_tab,
                        &[
                            (LayerDockTab::Layer, "Layer"),
                            (LayerDockTab::SearchLayer, "Search Layer"),
                        ],
                    );

                    // Layer Row 1: Blending Mode + Opacity Bar Slider
                    if let Some(active) = self.doc.layers.get_mut(self.doc.active_layer_idx) {
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("layer_blend_mode")
                                .selected_text(active.blend_mode.name())
                                .width(96.0)
                                .show_ui(ui, |ui| {
                                    for &mode in BlendMode::all() {
                                        ui.selectable_value(
                                            &mut active.blend_mode,
                                            mode,
                                            mode.name(),
                                        );
                                    }
                                });

                            let mut op_pct = active.opacity * 100.0;
                            if csp_bar_slider(
                                ui,
                                "Opacity",
                                &mut op_pct,
                                0.0..=100.0,
                                false,
                                0,
                                "%",
                            ) {
                                active.opacity = op_pct / 100.0;
                            }
                        });

                        // Layer Row 2: CSP's 6 Iconic Square Toggle Buttons
                        ui.horizontal(|ui| {
                            let b_sz = Vec2::splat(21.0);
                            if icon_button(
                                ui,
                                StudioIcon::ClipMaskBar,
                                b_sz,
                                active.clipped_to_below,
                                "Clip to Layer Below (Ctrl+Alt+G)",
                            )
                            .clicked()
                            {
                                active.clipped_to_below = !active.clipped_to_below;
                            }

                            let is_ref = active.is_reference;
                            if icon_button(
                                ui,
                                StudioIcon::LighthouseRef,
                                b_sz,
                                is_ref,
                                "Set as Reference Layer (Lighthouse)",
                            )
                            .clicked()
                            {
                                active.is_reference = !active.is_reference;
                                self.reference_layer_id =
                                    if active.is_reference { Some(active.id) } else { None };
                            }

                            if icon_button(
                                ui,
                                StudioIcon::DraftPencil,
                                b_sz,
                                active.is_draft,
                                "Set as Draft Layer",
                            )
                            .clicked()
                            {
                                active.is_draft = !active.is_draft;
                            }

                            if icon_button(
                                ui,
                                StudioIcon::PadlockLocked,
                                b_sz,
                                active.locked,
                                "Lock Layer",
                            )
                            .clicked()
                            {
                                active.locked = !active.locked;
                            }

                            if icon_button(
                                ui,
                                StudioIcon::PadlockAlpha,
                                b_sz,
                                active.alpha_locked,
                                "Lock Transparent Pixels (/)",
                            )
                            .clicked()
                            {
                                active.alpha_locked = !active.alpha_locked;
                            }

                            egui::ComboBox::from_id_salt("layer_color_tag")
                                .selected_text(active.color_tag.name())
                                .width(64.0)
                                .show_ui(ui, |ui| {
                                    for &tag in &[
                                        LayerColorTag::None,
                                        LayerColorTag::Red,
                                        LayerColorTag::Orange,
                                        LayerColorTag::Yellow,
                                        LayerColorTag::Green,
                                        LayerColorTag::Blue,
                                        LayerColorTag::Purple,
                                        LayerColorTag::Gray,
                                    ] {
                                        ui.selectable_value(&mut active.color_tag, tag, tag.name());
                                    }
                                });
                        });
                    }

                    ui.separator();

                    // Spreadsheet-style CSP Layer Stack List (Top-to-Bottom)
                    let ref_id = self.reference_layer_id;
                    for (idx, layer) in self.doc.layers.iter_mut().enumerate().rev() {
                        let is_selected = idx == self.doc.active_layer_idx;
                        let row_bg = if is_selected {
                            Color32::from_rgb(38, 98, 188)
                        } else {
                            Color32::from_rgb(39, 41, 46)
                        };

                        let (row_rect, row_resp) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 29.0),
                            Sense::click(),
                        );
                        let painter = ui.painter_at(row_rect);
                        painter.rect_filled(row_rect, 1.0, row_bg);
                        painter.line_segment(
                            [row_rect.left_bottom(), row_rect.right_bottom()],
                            Stroke::new(1.0_f32, Color32::from_rgb(28, 30, 34)),
                        );

                        // Left cell 1: Eye visibility column (22px wide)
                        let eye_col = Rect::from_min_size(
                            row_rect.left_top(),
                            Vec2::new(22.0, row_rect.height()),
                        );
                        painter.line_segment(
                            [eye_col.right_top(), eye_col.right_bottom()],
                            Stroke::new(1.0_f32, Color32::from_rgb(30, 32, 36)),
                        );
                        let eye_resp =
                            ui.interact(eye_col, ui.id().with(idx).with("eye"), Sense::click());
                        draw_icon(
                            &painter,
                            eye_col.shrink(4.0),
                            if layer.visible {
                                StudioIcon::EyeOpen
                            } else {
                                StudioIcon::EyeClosed
                            },
                            if layer.visible {
                                Color32::from_gray(225)
                            } else {
                                Color32::from_gray(85)
                            },
                        );
                        if eye_resp.clicked() {
                            layer.visible = !layer.visible;
                        }

                        // Left cell 2: Reference / Draft / Lock status column (18px wide)
                        let stat_col = Rect::from_min_size(
                            row_rect.left_top() + Vec2::new(22.0, 0.0),
                            Vec2::new(18.0, row_rect.height()),
                        );
                        painter.line_segment(
                            [stat_col.right_top(), stat_col.right_bottom()],
                            Stroke::new(1.0_f32, Color32::from_rgb(30, 32, 36)),
                        );
                        if ref_id == Some(layer.id) {
                            draw_icon(
                                &painter,
                                stat_col.shrink(3.0),
                                StudioIcon::LighthouseRef,
                                Color32::from_rgb(255, 210, 80),
                            );
                        } else if layer.is_draft {
                            draw_icon(
                                &painter,
                                stat_col.shrink(3.0),
                                StudioIcon::DraftPencil,
                                Color32::from_rgb(80, 185, 255),
                            );
                        }

                        // Red Clipping Mask vertical bar (iconic CSP red bar next to thumbnail)
                        if layer.clipped_to_below {
                            let clip_bar = Rect::from_min_size(
                                row_rect.left_top() + Vec2::new(41.0, 2.0),
                                Vec2::new(3.0, row_rect.height() - 4.0),
                            );
                            painter.rect_filled(clip_bar, 0.0, Color32::from_rgb(235, 55, 55));
                        }

                        // Layer Color Tag stripe if set
                        if let Some(rgb) = layer.color_tag.rgb() {
                            let tag_bar = Rect::from_min_size(
                                row_rect.left_top(),
                                Vec2::new(3.0, row_rect.height()),
                            );
                            painter.rect_filled(
                                tag_bar,
                                0.0,
                                Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                            );
                        }

                        // Layer Thumbnail box (checkerboard + paper preview)
                        let thumb_rect = Rect::from_min_size(
                            row_rect.left_top() + Vec2::new(46.0, 4.0),
                            Vec2::new(24.0, 21.0),
                        );
                        painter.rect_filled(thumb_rect, 1.0, Color32::from_rgb(245, 245, 248));
                        painter.rect_stroke(
                            thumb_rect,
                            1.0,
                            Stroke::new(1.0_f32, Color32::from_gray(85)),
                        );

                        // Layer Name & Blend/Opacity summary
                        let text_pos = row_rect.left_center() + Vec2::new(76.0, -4.0);
                        let meta_pos = row_rect.left_center() + Vec2::new(76.0, 6.5);
                        let text_col = if is_selected {
                            Color32::WHITE
                        } else {
                            Color32::from_gray(225)
                        };

                        if self.renaming_layer_idx == Some(idx) {
                            let mut temp_name = self.renaming_layer_name.clone();
                            let input_rect = Rect::from_min_size(
                                row_rect.left_top() + Vec2::new(74.0, 4.0),
                                Vec2::new(140.0, 20.0),
                            );
                            let mut child_ui = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(input_rect)
                                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                            );
                            let resp = child_ui.text_edit_singleline(&mut temp_name);
                            self.renaming_layer_name = temp_name;
                            if resp.lost_focus() || ui.input(|i| i.key_pressed(Key::Enter)) {
                                if !self.renaming_layer_name.trim().is_empty() {
                                    layer.name = self.renaming_layer_name.trim().to_string();
                                }
                                self.renaming_layer_idx = None;
                            }
                        } else {
                            painter.text(
                                text_pos,
                                Align2::LEFT_CENTER,
                                &layer.name,
                                FontId::proportional(11.5),
                                text_col,
                            );
                            painter.text(
                                meta_pos,
                                Align2::LEFT_CENTER,
                                format!("{} • {:.0}%", layer.blend_mode.name(), layer.opacity * 100.0),
                                FontId::proportional(9.5),
                                if is_selected {
                                    Color32::from_rgb(205, 225, 255)
                                } else {
                                    Color32::from_gray(145)
                                },
                            );
                            if row_resp.double_clicked() {
                                self.renaming_layer_idx = Some(idx);
                                self.renaming_layer_name = layer.name.clone();
                            }
                        }

                        // Right-side lock icons if active
                        if layer.locked {
                            let lr = Rect::from_min_size(
                                row_rect.right_top() + Vec2::new(-20.0, 6.0),
                                Vec2::splat(16.0),
                            );
                            draw_icon(&painter, lr, StudioIcon::PadlockLocked, Color32::WHITE);
                        } else if layer.alpha_locked {
                            let lr = Rect::from_min_size(
                                row_rect.right_top() + Vec2::new(-20.0, 6.0),
                                Vec2::splat(16.0),
                            );
                            draw_icon(&painter, lr, StudioIcon::PadlockAlpha, Color32::WHITE);
                        }

                        if row_resp.clicked() {
                            self.doc.active_layer_idx = idx;
                        }
                    }

                    ui.separator();

                    // Layer Palette Bottom Action Icon Bar (Authentic CSP Bottom Strip)
                    ui.horizontal(|ui| {
                        let b_sz = Vec2::splat(21.0);
                        if icon_button(
                            ui,
                            StudioIcon::NewRasterLayer,
                            b_sz,
                            false,
                            "New Raster Layer (Ctrl+Shift+N)",
                        )
                        .clicked()
                        {
                            self.history.push_state("New Layer", &self.doc);
                            self.doc
                                .add_layer(format!("Layer {}", self.doc.layers.len() + 1));
                        }
                        if icon_button(
                            ui,
                            StudioIcon::NewVectorLayer,
                            b_sz,
                            false,
                            "New Vector Layer",
                        )
                        .clicked()
                        {
                            self.history.push_state("New Vector Layer", &self.doc);
                            self.doc
                                .add_vector_layer(format!("Vector {}", self.doc.layers.len() + 1));
                        }
                        if icon_button(
                            ui,
                            StudioIcon::NewLayerFolder,
                            b_sz,
                            false,
                            "New Layer Folder",
                        )
                        .clicked()
                        {
                            self.history.push_state("New Folder", &self.doc);
                            self.doc.add_folder(format!("Folder {}", self.doc.layers.len() + 1));
                        }
                        if icon_button(
                            ui,
                            StudioIcon::DuplicateLayer,
                            b_sz,
                            false,
                            "Duplicate Layer",
                        )
                        .clicked()
                        {
                            self.duplicate_active_layer();
                        }
                        if icon_button(ui, StudioIcon::MoveUp, b_sz, false, "Move Layer Up").clicked()
                        {
                            self.doc.move_layer_up(self.doc.active_layer_idx);
                        }
                        if icon_button(ui, StudioIcon::MoveDown, b_sz, false, "Move Layer Down")
                            .clicked()
                        {
                            self.doc.move_layer_down(self.doc.active_layer_idx);
                        }
                        if icon_button(
                            ui,
                            StudioIcon::MergeDown,
                            b_sz,
                            false,
                            "Merge with Layer Below (Ctrl+E)",
                        )
                        .clicked()
                        {
                            self.merge_active_layer_down();
                        }
                        let has_mask = self.doc.layers.get(self.doc.active_layer_idx).map_or(false, |l| l.mask.is_some());
                        if icon_button(
                            ui,
                            StudioIcon::LayerMask,
                            b_sz,
                            has_mask,
                            if has_mask { "Delete Layer Mask" } else { "Create Layer Mask" },
                        )
                        .clicked()
                        {
                            let active_idx = self.doc.active_layer_idx;
                            if let Some(l) = self.doc.layers.get(active_idx) {
                                if l.mask.is_none() {
                                    self.history.push_state("Create Layer Mask", &self.doc);
                                    self.doc.add_layer_mask(active_idx);
                                    self.status_message = "Created Layer Mask".into();
                                } else {
                                    self.history.push_state("Delete Layer Mask", &self.doc);
                                    self.doc.delete_layer_mask(active_idx);
                                    self.status_message = "Deleted Layer Mask".into();
                                }
                            }
                        }
                        if icon_button(
                            ui,
                            StudioIcon::DeleteTrash,
                            b_sz,
                            false,
                            "Delete Layer",
                        )
                        .clicked()
                        {
                            self.history.push_state("Delete Layer", &self.doc);
                            self.doc.delete_layer(self.doc.active_layer_idx);
                        }
                    });
                });
            });

        // =========================================================================
        // 6. Central Drawing Canvas Viewport (Document Tab Bar + Rulers + Canvas + Nav Bar)
        // =========================================================================
        egui::CentralPanel::default().show(ctx, |ui| {
            // Document Tab Bar (CSP Multi-Document Dynamic Strip)
            ui.horizontal(|ui| {
                let active_layer_name = self
                    .doc
                    .active_layer()
                    .map(|l| l.name.as_str())
                    .unwrap_or("Layer 1");

                let mut to_switch: Option<usize> = None;
                let mut to_close: Option<usize> = None;

                for i in 0..self.tabs.len() {
                    let is_active = i == self.active_tab;
                    let tab_title = if is_active {
                        format!(
                            "{}* ({:.0}%) ({}, RGB)",
                            self.doc.name,
                            self.viewport.zoom * 100.0,
                            active_layer_name
                        )
                    } else {
                        self.tabs[i].title.clone()
                    };

                    let tab_width = 160.0_f32.max(tab_title.len() as f32 * 6.5 + 32.0).min(280.0);
                    let (tab_rect, tab_resp) = ui.allocate_exact_size(Vec2::new(tab_width, 22.0), Sense::click());
                    let painter = ui.painter_at(tab_rect);

                    let tab_bg = if is_active {
                        Color32::from_rgb(48, 51, 57)
                    } else if tab_resp.hovered() {
                        Color32::from_rgb(40, 42, 47)
                    } else {
                        Color32::from_rgb(32, 34, 38)
                    };

                    painter.rect_filled(
                        tab_rect,
                        Rounding {
                            nw: 2.0,
                            ne: 2.0,
                            sw: 0.0,
                            se: 0.0,
                        },
                        tab_bg,
                    );

                    if is_active {
                        painter.line_segment(
                            [tab_rect.left_top(), tab_rect.right_top()],
                            Stroke::new(2.0_f32, Color32::from_rgb(55, 135, 240)),
                        );
                    }

                    // Tab text
                    let text_color = if is_active {
                        Color32::from_rgb(235, 240, 246)
                    } else {
                        Color32::from_gray(160)
                    };
                    painter.text(
                        tab_rect.left_center() + Vec2::new(8.0, 0.0),
                        Align2::LEFT_CENTER,
                        tab_title,
                        FontId::proportional(11.0),
                        text_color,
                    );

                    // Close button "×"
                    let close_rect = Rect::from_center_size(
                        tab_rect.right_center() - Vec2::new(10.0, 0.0),
                        Vec2::splat(14.0),
                    );
                    let close_hover = tab_resp.hovered() && ui.rect_contains_pointer(close_rect);
                    if close_hover {
                        painter.rect_filled(close_rect, 2.0, Color32::from_rgba_unmultiplied(230, 60, 60, 180));
                    }
                    painter.text(
                        close_rect.center(),
                        Align2::CENTER_CENTER,
                        "×",
                        FontId::proportional(12.0),
                        if close_hover { Color32::WHITE } else { Color32::from_gray(170) },
                    );

                    if tab_resp.clicked() {
                        if close_hover {
                            to_close = Some(i);
                        } else if !is_active {
                            to_switch = Some(i);
                        }
                    }
                }

                if let Some(target) = to_close {
                    self.close_tab(target);
                } else if let Some(target) = to_switch {
                    self.switch_tab(target);
                }

                if icon_button(
                    ui,
                    StudioIcon::NewDoc,
                    Vec2::splat(20.0),
                    false,
                    "New Canvas (Ctrl+N)",
                )
                .clicked()
                {
                    self.new_dialog_open = true;
                }
            });

            // Canvas Viewport (with Top/Left Pixel Rulers, Scrollbars & Selection Launcher)
            let (_, stroke_started) =
                self.viewport
                    .show(ui, &mut self.doc, &mut self.config, &mut self.history);
            if stroke_started {
                let rgb = [
                    self.config.primary_color[0],
                    self.config.primary_color[1],
                    self.config.primary_color[2],
                ];
                if self.color_history.first() != Some(&rgb) {
                    self.color_history.retain(|c| *c != rgb);
                    self.color_history.insert(0, rgb);
                    self.color_history.truncate(8);
                }
            }

            // Bottom Canvas Control Bar (CSP Integrated Canvas Navigation Strip)
            ui.horizontal(|ui| {
                let b_sz = Vec2::splat(18.0);
                if icon_button(ui, StudioIcon::ZoomOut, b_sz, false, "Zoom Out").clicked() {
                    self.viewport.zoom = (self.viewport.zoom * 0.85).clamp(0.05, 32.0);
                }

                let zoom_pct = format!("{:.1}%", self.viewport.zoom * 100.0);
                egui::ComboBox::from_id_salt("bottom_zoom_presets")
                    .selected_text(zoom_pct)
                    .width(66.0)
                    .show_ui(ui, |ui| {
                        for &(lbl, z) in &[
                            ("12.5%", 0.125_f32),
                            ("25.0%", 0.25),
                            ("50.0%", 0.50),
                            ("100.0%", 1.0),
                            ("200.0%", 2.0),
                            ("400.0%", 4.0),
                        ] {
                            if ui.selectable_label(false, lbl).clicked() {
                                self.viewport.zoom = z;
                            }
                        }
                        if ui.selectable_label(false, "Fit to Screen").clicked() {
                            self.viewport.reset_view(
                                ui.available_size(),
                                self.doc.width,
                                self.doc.height,
                            );
                        }
                    });

                if icon_button(ui, StudioIcon::ZoomIn, b_sz, false, "Zoom In").clicked() {
                    self.viewport.zoom = (self.viewport.zoom * 1.15).clamp(0.05, 32.0);
                }
                if icon_button(ui, StudioIcon::FitScreen, b_sz, false, "Fit to Screen").clicked() {
                    self.viewport
                        .reset_view(ui.available_size(), self.doc.width, self.doc.height);
                }

                ui.separator();

                if icon_button(ui, StudioIcon::UndoArrow, b_sz, false, "Rotate Left 5°").clicked() {
                    self.viewport.rotation -= 5.0_f32.to_radians();
                }
                let rot_deg = self.viewport.rotation.to_degrees();
                if ui
                    .button(RichText::new(format!("{:.1}°", rot_deg)).size(10.5).monospace())
                    .on_hover_text("Reset Rotation")
                    .clicked()
                {
                    self.viewport.rotation = 0.0;
                }
                if icon_button(ui, StudioIcon::RedoArrow, b_sz, false, "Rotate Right 5°").clicked()
                {
                    self.viewport.rotation += 5.0_f32.to_radians();
                }

                ui.separator();

                if icon_button(
                    ui,
                    StudioIcon::FlipH,
                    b_sz,
                    self.viewport.flip_h,
                    "Flip Horizontal",
                )
                .clicked()
                {
                    self.viewport.flip_h = !self.viewport.flip_h;
                }
                if icon_button(
                    ui,
                    StudioIcon::FlipV,
                    b_sz,
                    self.viewport.flip_v,
                    "Flip Vertical",
                )
                .clicked()
                {
                    self.viewport.flip_v = !self.viewport.flip_v;
                }

                ui.separator();

                if icon_button(
                    ui,
                    StudioIcon::MangaGuides,
                    b_sz,
                    self.viewport.show_manga_guides,
                    "Toggle Manga Manuscript Guides",
                )
                .clicked()
                {
                    self.viewport.show_manga_guides = !self.viewport.show_manga_guides;
                }
                if icon_button(
                    ui,
                    StudioIcon::CanvasGrid,
                    b_sz,
                    self.viewport.show_grid,
                    "Toggle Grid",
                )
                .clicked()
                {
                    self.viewport.show_grid = !self.viewport.show_grid;
                }
                if icon_button(
                    ui,
                    StudioIcon::RulerBar,
                    b_sz,
                    self.viewport.show_rulers,
                    "Toggle Pixel Rulers",
                )
                .clicked()
                {
                    self.viewport.show_rulers = !self.viewport.show_rulers;
                }
            });
        });

        // =========================================================================
        // 7. "New Document" Modal Dialog (CSP Project Presets: Illustration, Webtoon, Comic)
        // =========================================================================
        if self.new_dialog_open {
            egui::Window::new("New — CLIP STUDIO PAINT Project Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(RichText::new("Use of work:").strong());
                    ui.horizontal(|ui| {
                        if ui.button("Illustration (1600×2400)").clicked() {
                            self.new_width = 1600;
                            self.new_height = 2400;
                            self.new_dpi = 300.0;
                            self.new_preset_name = "Illustration".into();
                        }
                        if ui.button("Manga B5 (1600×2400 • 600dpi)").clicked() {
                            self.new_width = 1600;
                            self.new_height = 2400;
                            self.new_dpi = 600.0;
                            self.new_preset_name = "Manga_B5".into();
                        }
                        if ui.button("Comic A4 (2480×3508)").clicked() {
                            self.new_width = 2480;
                            self.new_height = 3508;
                            self.new_dpi = 350.0;
                            self.new_preset_name = "Comic_A4".into();
                        }
                        if ui.button("Webtoon (800×4800)").clicked() {
                            self.new_width = 800;
                            self.new_height = 4800;
                            self.new_dpi = 72.0;
                            self.new_preset_name = "Webtoon_01".into();
                        }
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        ui.add(egui::DragValue::new(&mut self.new_width).range(100..=8000));
                        ui.label("Height:");
                        ui.add(egui::DragValue::new(&mut self.new_height).range(100..=12000));
                        ui.label("Resolution:");
                        ui.add(egui::DragValue::new(&mut self.new_dpi).range(72.0..=1200.0));
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("  OK  ").clicked() {
                            let new_doc = Document::new(
                                format!("{}.clip", self.new_preset_name),
                                self.new_width,
                                self.new_height,
                                self.new_dpi,
                            );
                            self.create_new_tab(new_doc, None);
                            self.new_dialog_open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.new_dialog_open = false;
                        }
                    });
                });
        }

        self.render_filter_dialogs(ctx);
    }
}

impl ComicCraftApp {
    pub fn switch_tab(&mut self, target_idx: usize) {
        if target_idx == self.active_tab || target_idx >= self.tabs.len() {
            return;
        }
        let active_idx = self.active_tab;
        let old_session = TabSession {
            doc: std::mem::replace(&mut self.doc, Document::new("temp", 10, 10, 300.0)),
            history: std::mem::replace(&mut self.history, HistoryManager::new(25)),
            viewport: std::mem::replace(&mut self.viewport, CanvasViewport::default()),
            file_path: self.file_path.take(),
        };
        self.tabs[active_idx].title = old_session.doc.name.clone();
        self.tabs[active_idx].session = Some(old_session);

        self.active_tab = target_idx;
        if let Some(target_session) = self.tabs[target_idx].session.take() {
            self.doc = target_session.doc;
            self.history = target_session.history;
            self.viewport = target_session.viewport;
            self.file_path = target_session.file_path;
        }
        self.doc.mark_full_dirty();
        self.viewport.texture = None;
        self.viewport.initialized = false;
        self.status_message = format!("Switched to tab: {}", self.doc.name);
    }

    pub fn create_new_tab(&mut self, new_doc: Document, path: Option<PathBuf>) {
        let active_idx = self.active_tab;
        let old_session = TabSession {
            doc: std::mem::replace(&mut self.doc, Document::new("temp", 10, 10, 300.0)),
            history: std::mem::replace(&mut self.history, HistoryManager::new(25)),
            viewport: std::mem::replace(&mut self.viewport, CanvasViewport::default()),
            file_path: self.file_path.take(),
        };
        self.tabs[active_idx].title = old_session.doc.name.clone();
        self.tabs[active_idx].session = Some(old_session);

        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;
        let tab_title = new_doc.name.clone();
        self.tabs.push(TabMeta {
            id: tab_id,
            session: None,
            title: tab_title,
        });
        self.active_tab = self.tabs.len() - 1;

        self.doc = new_doc;
        self.history = HistoryManager::new(25);
        let mut vp = CanvasViewport::default();
        vp.pan = Vec2::new(750.0, 520.0);
        self.viewport = vp;
        self.file_path = path;
        self.doc.mark_full_dirty();
        self.status_message = format!("Created tab: {}", self.doc.name);
    }

    pub fn close_tab(&mut self, target_idx: usize) {
        if target_idx >= self.tabs.len() {
            return;
        }
        if self.tabs.len() == 1 {
            self.doc = Document::new("Illustration_01.clip", 1600, 2400, 300.0);
            self.history = HistoryManager::new(25);
            self.file_path = None;
            self.tabs[0].title = "Illustration_01.clip".to_string();
            self.doc.mark_full_dirty();
            self.viewport.texture = None;
            self.viewport.initialized = false;
            return;
        }

        if target_idx == self.active_tab {
            let next_active = if target_idx + 1 < self.tabs.len() {
                target_idx + 1
            } else {
                target_idx - 1
            };
            self.switch_tab(next_active);
            self.tabs.remove(target_idx);
            if target_idx < self.active_tab {
                self.active_tab -= 1;
            }
        } else {
            self.tabs.remove(target_idx);
            if target_idx < self.active_tab {
                self.active_tab -= 1;
            }
        }
        self.status_message = format!("Closed tab; active: {}", self.doc.name);
    }

    pub fn open_binarization_dialog(&mut self) {
        if let Some(layer) = self.doc.active_layer() {
            self.binarization_dialog.original_layer = Some(layer.clone());
            self.binarization_dialog.threshold = 128;
            self.binarization_dialog.preview = true;
            self.binarization_dialog.open = true;
        }
    }

    pub fn open_levels_dialog(&mut self) {
        if let Some(layer) = self.doc.active_layer() {
            self.levels_dialog.original_layer = Some(layer.clone());
            self.levels_dialog.in_black = 0;
            self.levels_dialog.in_white = 255;
            self.levels_dialog.gamma = 1.0;
            self.levels_dialog.out_black = 0;
            self.levels_dialog.out_white = 255;
            self.levels_dialog.preview = true;
            self.levels_dialog.open = true;
        }
    }

    pub fn open_gradient_map_dialog(&mut self) {
        if let Some(layer) = self.doc.active_layer() {
            self.gradient_map_dialog.original_layer = Some(layer.clone());
            self.gradient_map_dialog.selected_preset = 0;
            self.gradient_map_dialog.preview = true;
            self.gradient_map_dialog.open = true;
        }
    }

    pub fn apply_gradient_preset(&mut self, preset_idx: usize) {
        self.history.push_state("Gradient Map", &self.doc);
        let idx = self.doc.active_layer_idx;
        let sel = &self.doc.selection;
        let map = match preset_idx {
            1 => comiccraft_core::GradientMap::preset_sepia(),
            2 => comiccraft_core::GradientMap::preset_anime_sky(),
            3 => comiccraft_core::GradientMap::preset_cyberpunk(),
            _ => comiccraft_core::GradientMap::preset_sunset(),
        };
        if let Some(layer) = self.doc.layers.get_mut(idx) {
            comiccraft_core::apply_gradient_map(layer, sel, &map);
        }
        self.doc.mark_full_dirty();
    }

    pub fn open_brightness_contrast_dialog(&mut self) {
        if let Some(layer) = self.doc.active_layer() {
            self.bc_dialog.original_layer = Some(layer.clone());
            self.bc_dialog.brightness = 0;
            self.bc_dialog.contrast = 0;
            self.bc_dialog.preview = true;
            self.bc_dialog.open = true;
        }
    }

    pub fn open_hue_sat_lum_dialog(&mut self) {
        if let Some(layer) = self.doc.active_layer() {
            self.hsl_dialog.original_layer = Some(layer.clone());
            self.hsl_dialog.hue = 0.0;
            self.hsl_dialog.sat = 0.0;
            self.hsl_dialog.lum = 0.0;
            self.hsl_dialog.preview = true;
            self.hsl_dialog.open = true;
        }
    }

    pub fn open_gaussian_blur_dialog(&mut self) {
        if let Some(layer) = self.doc.active_layer() {
            self.blur_dialog.original_layer = Some(layer.clone());
            self.blur_dialog.radius = 5.0;
            self.blur_dialog.preview = true;
            self.blur_dialog.open = true;
            let idx = self.doc.active_layer_idx;
            let sel = &self.doc.selection;
            if let Some(l) = self.doc.layers.get_mut(idx) {
                apply_gaussian_blur(l, sel, self.blur_dialog.radius);
            }
            self.doc.mark_full_dirty();
        }
    }

    pub fn apply_invert_colors(&mut self) {
        self.history.push_state("Invert Colors", &self.doc);
        let idx = self.doc.active_layer_idx;
        let sel = &self.doc.selection;
        if let Some(layer) = self.doc.layers.get_mut(idx) {
            apply_invert(layer, sel);
        }
        self.doc.mark_full_dirty();
    }

    pub fn copy_to_clipboard(&mut self, merged: bool) {
        match crate::clipboard_utils::copy_selection_or_layer_to_clipboard(&mut self.doc, merged) {
            Ok(msg) => self.status_message = msg,
            Err(err) => self.status_message = err,
        }
    }

    pub fn cut_to_clipboard(&mut self) {
        match crate::clipboard_utils::copy_selection_or_layer_to_clipboard(&mut self.doc, false) {
            Ok(_) => {
                self.history.push_state("Cut", &self.doc);
                self.doc.clear_active_selection_or_layer();
                self.status_message = "Cut to clipboard".into();
            }
            Err(err) => self.status_message = err,
        }
    }

    pub fn paste_from_clipboard(&mut self) {
        self.history.push_state("Paste Image", &self.doc);
        match crate::clipboard_utils::paste_image_into_document(&mut self.doc) {
            Ok((_, w, h)) => {
                self.status_message = format!("Pasted image ({}x{}) into new layer", w, h);
            }
            Err(err) => {
                self.history.undo(&mut self.doc);
                self.status_message = err;
            }
        }
    }

    pub fn new_canvas_from_clipboard(&mut self) {
        match crate::clipboard_utils::get_clipboard_image() {
            Ok((img_w, img_h, bytes)) => {
                let mut new_doc = Document::new(
                    format!("Clipboard ({}x{})", img_w, img_h),
                    img_w,
                    img_h,
                    300.0,
                );
                if let Some(layer) = new_doc.layers.get_mut(1) {
                    layer.name = "Pasted Layer".into();
                    let total = (img_w * img_h * 4) as usize;
                    if bytes.len() >= total {
                        layer.pixels = bytes[..total].to_vec();
                    }
                }
                new_doc.mark_full_dirty();
                new_doc.update_composite();

                self.history = HistoryManager::new(25);
                self.doc = new_doc;
                self.file_path = None;
                self.viewport.texture = None;
                self.viewport.initialized = false;
                self.viewport.layer_sig = 0;
                self.status_message = format!("Created new canvas from clipboard ({}x{})", img_w, img_h);
            }
            Err(err) => {
                self.status_message = err;
            }
        }
    }

    fn render_filter_dialogs(&mut self, ctx: &egui::Context) {
        // ---------------------------------------------------------------------
        // 1. Brightness / Contrast
        // ---------------------------------------------------------------------
        if self.bc_dialog.open {
            let mut close = false;
            let mut commit = false;
            let mut changed = false;

            egui::Window::new("Brightness/Contrast — CLIP STUDIO PAINT")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    let mut b_f = self.bc_dialog.brightness as f32;
                    if csp_bar_slider(ui, "Brightness", &mut b_f, -100.0..=100.0, false, 0, "") {
                        self.bc_dialog.brightness = b_f.round() as i32;
                        changed = true;
                    }
                    let mut c_f = self.bc_dialog.contrast as f32;
                    if csp_bar_slider(ui, "Contrast", &mut c_f, -100.0..=100.0, false, 0, "") {
                        self.bc_dialog.contrast = c_f.round() as i32;
                        changed = true;
                    }

                    if ui.checkbox(&mut self.bc_dialog.preview, "Preview").changed() {
                        changed = true;
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("  OK  ").clicked() {
                            commit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });

            if changed && self.bc_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.bc_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                let sel = &self.doc.selection;
                if let Some(layer) = self.doc.layers.get_mut(idx) {
                    apply_brightness_contrast(layer, sel, self.bc_dialog.brightness, self.bc_dialog.contrast);
                }
                self.doc.mark_full_dirty();
            } else if changed && !self.bc_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.bc_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                self.doc.mark_full_dirty();
            }

            if commit {
                if let Some(mut orig) = self.bc_dialog.original_layer.take() {
                    let idx = self.doc.active_layer_idx;
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                    self.history.push_state("Brightness/Contrast", &self.doc);
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                }
                self.bc_dialog.open = false;
            } else if close {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = self.bc_dialog.original_layer.take() {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels = orig.pixels;
                    }
                }
                self.doc.mark_full_dirty();
                self.bc_dialog.open = false;
            }
        }

        // ---------------------------------------------------------------------
        // 2. Hue / Saturation / Luminosity
        // ---------------------------------------------------------------------
        if self.hsl_dialog.open {
            let mut close = false;
            let mut commit = false;
            let mut changed = false;

            egui::Window::new("Hue/Saturation/Luminosity — CLIP STUDIO PAINT")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    if csp_bar_slider(ui, "Hue", &mut self.hsl_dialog.hue, -180.0..=180.0, false, 0, "°") {
                        changed = true;
                    }
                    if csp_bar_slider(ui, "Saturation", &mut self.hsl_dialog.sat, -100.0..=100.0, false, 0, " %") {
                        changed = true;
                    }
                    if csp_bar_slider(ui, "Luminosity", &mut self.hsl_dialog.lum, -100.0..=100.0, false, 0, " %") {
                        changed = true;
                    }

                    if ui.checkbox(&mut self.hsl_dialog.preview, "Preview").changed() {
                        changed = true;
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("  OK  ").clicked() {
                            commit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });

            if changed && self.hsl_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.hsl_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                let sel = &self.doc.selection;
                if let Some(layer) = self.doc.layers.get_mut(idx) {
                    apply_hue_saturation_luminosity(
                        layer,
                        sel,
                        self.hsl_dialog.hue,
                        self.hsl_dialog.sat,
                        self.hsl_dialog.lum,
                    );
                }
                self.doc.mark_full_dirty();
            } else if changed && !self.hsl_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.hsl_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                self.doc.mark_full_dirty();
            }

            if commit {
                if let Some(mut orig) = self.hsl_dialog.original_layer.take() {
                    let idx = self.doc.active_layer_idx;
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                    self.history.push_state("Hue/Saturation/Luminosity", &self.doc);
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                }
                self.hsl_dialog.open = false;
            } else if close {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = self.hsl_dialog.original_layer.take() {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels = orig.pixels;
                    }
                }
                self.doc.mark_full_dirty();
                self.hsl_dialog.open = false;
            }
        }

        // ---------------------------------------------------------------------
        // 3. Gaussian Blur
        // ---------------------------------------------------------------------
        if self.blur_dialog.open {
            let mut close = false;
            let mut commit = false;
            let mut changed = false;

            egui::Window::new("Gaussian Blur — CLIP STUDIO PAINT")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    if csp_bar_slider(ui, "Radius", &mut self.blur_dialog.radius, 1.0..=50.0, false, 1, " px") {
                        changed = true;
                    }

                    if ui.checkbox(&mut self.blur_dialog.preview, "Preview").changed() {
                        changed = true;
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("  OK  ").clicked() {
                            commit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });

            if changed && self.blur_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.blur_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                let sel = &self.doc.selection;
                if let Some(layer) = self.doc.layers.get_mut(idx) {
                    apply_gaussian_blur(layer, sel, self.blur_dialog.radius);
                }
                self.doc.mark_full_dirty();
            } else if changed && !self.blur_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.blur_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                self.doc.mark_full_dirty();
            }

            if commit {
                if let Some(mut orig) = self.blur_dialog.original_layer.take() {
                    let idx = self.doc.active_layer_idx;
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                    self.history.push_state("Gaussian Blur", &self.doc);
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                }
                self.blur_dialog.open = false;
            } else if close {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = self.blur_dialog.original_layer.take() {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels = orig.pixels;
                    }
                }
                self.doc.mark_full_dirty();
                self.blur_dialog.open = false;
            }
        }

        // ---------------------------------------------------------------------
        // 4. Binarization (Threshold)
        // ---------------------------------------------------------------------
        if self.binarization_dialog.open {
            let mut close = false;
            let mut commit = false;
            let mut changed = false;

            egui::Window::new("Binarization (Threshold) — CLIP STUDIO PAINT")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    let mut th_f = self.binarization_dialog.threshold as f32;
                    if csp_bar_slider(ui, "Threshold", &mut th_f, 1.0..=254.0, false, 0, "") {
                        self.binarization_dialog.threshold = th_f.round() as u8;
                        changed = true;
                    }
                    if ui.checkbox(&mut self.binarization_dialog.preview, "Preview").changed() {
                        changed = true;
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("  OK  ").clicked() {
                            commit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });

            if changed && self.binarization_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.binarization_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                let sel = &self.doc.selection;
                if let Some(layer) = self.doc.layers.get_mut(idx) {
                    comiccraft_core::apply_binarization(layer, sel, self.binarization_dialog.threshold);
                }
                self.doc.mark_full_dirty();
            } else if changed && !self.binarization_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.binarization_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                self.doc.mark_full_dirty();
            }

            if commit {
                if let Some(mut orig) = self.binarization_dialog.original_layer.take() {
                    let idx = self.doc.active_layer_idx;
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                    self.history.push_state("Binarization", &self.doc);
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                }
                self.binarization_dialog.open = false;
            } else if close {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = self.binarization_dialog.original_layer.take() {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels = orig.pixels;
                    }
                }
                self.doc.mark_full_dirty();
                self.binarization_dialog.open = false;
            }
        }

        // ---------------------------------------------------------------------
        // 5. Level Correction (Levels)
        // ---------------------------------------------------------------------
        if self.levels_dialog.open {
            let mut close = false;
            let mut commit = false;
            let mut changed = false;

            egui::Window::new("Level Correction — CLIP STUDIO PAINT")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    let mut in_b_f = self.levels_dialog.in_black as f32;
                    if csp_bar_slider(ui, "Input Shadow (Black)", &mut in_b_f, 0.0..=254.0, false, 0, "") {
                        self.levels_dialog.in_black = in_b_f.round() as u8;
                        changed = true;
                    }
                    let mut in_w_f = self.levels_dialog.in_white as f32;
                    if csp_bar_slider(ui, "Input Highlight (White)", &mut in_w_f, 1.0..=255.0, false, 0, "") {
                        self.levels_dialog.in_white = in_w_f.round() as u8;
                        changed = true;
                    }
                    if csp_bar_slider(ui, "Gamma", &mut self.levels_dialog.gamma, 0.1..=5.0, false, 2, "") {
                        changed = true;
                    }
                    let mut out_b_f = self.levels_dialog.out_black as f32;
                    if csp_bar_slider(ui, "Output Shadow", &mut out_b_f, 0.0..=255.0, false, 0, "") {
                        self.levels_dialog.out_black = out_b_f.round() as u8;
                        changed = true;
                    }
                    let mut out_w_f = self.levels_dialog.out_white as f32;
                    if csp_bar_slider(ui, "Output Highlight", &mut out_w_f, 0.0..=255.0, false, 0, "") {
                        self.levels_dialog.out_white = out_w_f.round() as u8;
                        changed = true;
                    }

                    if ui.checkbox(&mut self.levels_dialog.preview, "Preview").changed() {
                        changed = true;
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("  OK  ").clicked() {
                            commit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });

            if changed && self.levels_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.levels_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                let sel = &self.doc.selection;
                if let Some(layer) = self.doc.layers.get_mut(idx) {
                    comiccraft_core::apply_levels(
                        layer,
                        sel,
                        self.levels_dialog.in_black,
                        self.levels_dialog.in_white,
                        self.levels_dialog.gamma,
                        self.levels_dialog.out_black,
                        self.levels_dialog.out_white,
                    );
                }
                self.doc.mark_full_dirty();
            } else if changed && !self.levels_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.levels_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                self.doc.mark_full_dirty();
            }

            if commit {
                if let Some(mut orig) = self.levels_dialog.original_layer.take() {
                    let idx = self.doc.active_layer_idx;
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                    self.history.push_state("Level Correction", &self.doc);
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                }
                self.levels_dialog.open = false;
            } else if close {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = self.levels_dialog.original_layer.take() {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels = orig.pixels;
                    }
                }
                self.doc.mark_full_dirty();
                self.levels_dialog.open = false;
            }
        }

        // ---------------------------------------------------------------------
        // 6. Gradient Map
        // ---------------------------------------------------------------------
        if self.gradient_map_dialog.open {
            let mut close = false;
            let mut commit = false;
            let mut changed = false;

            egui::Window::new("Gradient Map — CLIP STUDIO PAINT")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Gradient Preset:");
                        let preset_names = ["Sunset Glow", "Sepia Tone", "Anime Sky", "Cyberpunk Neon"];
                        let current_label = preset_names.get(self.gradient_map_dialog.selected_preset).unwrap_or(&"Custom");
                        egui::ComboBox::from_id_salt("gradient_map_presets")
                            .selected_text(*current_label)
                            .show_ui(ui, |ui| {
                                for (idx, name) in preset_names.iter().enumerate() {
                                    if ui.selectable_value(&mut self.gradient_map_dialog.selected_preset, idx, *name).clicked() {
                                        changed = true;
                                    }
                                }
                            });
                    });

                    if ui.checkbox(&mut self.gradient_map_dialog.preview, "Preview").changed() {
                        changed = true;
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("  OK  ").clicked() {
                            commit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });

            let get_map = |preset: usize| match preset {
                1 => comiccraft_core::GradientMap::preset_sepia(),
                2 => comiccraft_core::GradientMap::preset_anime_sky(),
                3 => comiccraft_core::GradientMap::preset_cyberpunk(),
                _ => comiccraft_core::GradientMap::preset_sunset(),
            };

            if changed && self.gradient_map_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.gradient_map_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                let sel = &self.doc.selection;
                let map = get_map(self.gradient_map_dialog.selected_preset);
                if let Some(layer) = self.doc.layers.get_mut(idx) {
                    comiccraft_core::apply_gradient_map(layer, sel, &map);
                }
                self.doc.mark_full_dirty();
            } else if changed && !self.gradient_map_dialog.preview {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = &self.gradient_map_dialog.original_layer {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels.copy_from_slice(&orig.pixels);
                    }
                }
                self.doc.mark_full_dirty();
            }

            if commit {
                if let Some(mut orig) = self.gradient_map_dialog.original_layer.take() {
                    let idx = self.doc.active_layer_idx;
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                    self.history.push_state("Gradient Map", &self.doc);
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        std::mem::swap(&mut layer.pixels, &mut orig.pixels);
                    }
                }
                self.gradient_map_dialog.open = false;
            } else if close {
                let idx = self.doc.active_layer_idx;
                if let Some(orig) = self.gradient_map_dialog.original_layer.take() {
                    if let Some(layer) = self.doc.layers.get_mut(idx) {
                        layer.pixels = orig.pixels;
                    }
                }
                self.doc.mark_full_dirty();
                self.gradient_map_dialog.open = false;
            }
        }

        // ---------------------------------------------------------------------
        // 7. Border Selected Area Dialog
        // ---------------------------------------------------------------------
        if self.border_dialog.open {
            let mut close = false;
            let mut commit = false;

            egui::Window::new("Border Selected Area — CLIP STUDIO PAINT")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    let mut w_f = self.border_dialog.width_px as f32;
                    if csp_bar_slider(ui, "Border Thickness", &mut w_f, 1.0..=50.0, false, 0, "px") {
                        self.border_dialog.width_px = w_f.round().max(1.0) as u32;
                    }
                    ui.add_space(4.0);
                    ui.label("Position:");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.border_dialog.position, comiccraft_core::SelectionBorderPosition::Inside, "Inside");
                        ui.radio_value(&mut self.border_dialog.position, comiccraft_core::SelectionBorderPosition::Centered, "Centered");
                        ui.radio_value(&mut self.border_dialog.position, comiccraft_core::SelectionBorderPosition::Outside, "Outside");
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Border Color:");
                        let mut rgb = [self.border_dialog.color[0], self.border_dialog.color[1], self.border_dialog.color[2]];
                        if ui.color_edit_button_srgb(&mut rgb).changed() {
                            self.border_dialog.color[0] = rgb[0];
                            self.border_dialog.color[1] = rgb[1];
                            self.border_dialog.color[2] = rgb[2];
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("  OK  ").clicked() {
                            commit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });

            if commit {
                self.history.push_state("Border Selected Area", &self.doc);
                self.doc.stroke_selection_border(
                    self.border_dialog.width_px,
                    self.border_dialog.position,
                    self.border_dialog.color,
                );
                self.status_message = format!("Bordered selection with {}px stroke.", self.border_dialog.width_px);
                self.border_dialog.open = false;
            } else if close {
                self.border_dialog.open = false;
            }
        }

        // ---------------------------------------------------------------------
        // 8. Expand Selected Area Dialog
        // ---------------------------------------------------------------------
        if self.expand_dialog.open {
            let mut close = false;
            let mut commit = false;

            egui::Window::new("Expand Selected Area — CLIP STUDIO PAINT")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    let mut w_f = self.expand_dialog.width_px as f32;
                    if csp_bar_slider(ui, "Expand Width", &mut w_f, 1.0..=100.0, false, 0, "px") {
                        self.expand_dialog.width_px = w_f.round().max(1.0) as u32;
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("  OK  ").clicked() {
                            commit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });

            if commit {
                self.doc.selection.grow(self.expand_dialog.width_px as i32);
                self.status_message = format!("Expanded selection by {}px.", self.expand_dialog.width_px);
                self.expand_dialog.open = false;
            } else if close {
                self.expand_dialog.open = false;
            }
        }

        // ---------------------------------------------------------------------
        // 9. Shrink Selected Area Dialog
        // ---------------------------------------------------------------------
        if self.shrink_dialog.open {
            let mut close = false;
            let mut commit = false;

            egui::Window::new("Shrink Selected Area — CLIP STUDIO PAINT")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    let mut w_f = self.shrink_dialog.width_px as f32;
                    if csp_bar_slider(ui, "Shrink Width", &mut w_f, 1.0..=100.0, false, 0, "px") {
                        self.shrink_dialog.width_px = w_f.round().max(1.0) as u32;
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("  OK  ").clicked() {
                            commit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });

            if commit {
                self.doc.selection.grow(-(self.shrink_dialog.width_px as i32));
                self.status_message = format!("Shrunk selection by {}px.", self.shrink_dialog.width_px);
                self.shrink_dialog.open = false;
            } else if close {
                self.shrink_dialog.open = false;
            }
        }
    }

    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        if let Some(target) = self.screenshot_target.clone() {
            self.frame_count += 1;
            ctx.request_repaint();

            if self.frame_count == 3 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }

            let mut captured = None;
            ctx.input(|i| {
                for ev in &i.events {
                    if let egui::Event::Screenshot { image, .. } = ev {
                        captured = Some(image.clone());
                    }
                }
            });

            if let Some(img) = captured {
                let w = img.width() as u32;
                let h = img.height() as u32;
                let mut raw = Vec::with_capacity((w * h * 4) as usize);
                for px in &img.pixels {
                    raw.extend_from_slice(&[px.r(), px.g(), px.b(), px.a()]);
                }
                if let Some(rgba) = image::RgbaImage::from_raw(w, h, raw) {
                    let _ = rgba.save(&target);
                }
                std::process::exit(0);
            }
        }
    }

    pub fn duplicate_active_layer(&mut self) {
        let active_idx = self.doc.active_layer_idx;
        self.history.push_state("Duplicate Layer", &self.doc);
        if let Some(new_idx) = self.doc.duplicate_layer(active_idx) {
            self.status_message = format!("Duplicated layer: {}", self.doc.layers[new_idx].name);
        }
    }

    pub fn merge_active_layer_down(&mut self) {
        let active_idx = self.doc.active_layer_idx;
        if active_idx > 0 && self.doc.layers.len() > 1 {
            self.history.push_state("Merge Down", &self.doc);
            if self.doc.merge_down(active_idx) {
                self.status_message = format!("Merged layer down into: {}", self.doc.layers[self.doc.active_layer_idx].name);
            }
        }
    }

    pub fn merge_visible_layers(&mut self) {
        self.history.push_state("Merge Visible Layers", &self.doc);
        self.doc.merge_visible();
        self.status_message = "Merged all visible layers.".to_string();
    }

    pub fn flatten_image(&mut self) {
        self.history.push_state("Flatten Image", &self.doc);
        self.doc.flatten_image();
        self.status_message = "Flattened image into single Background layer.".to_string();
    }

    pub fn toggle_quick_mask(&mut self) {
        if !self.quick_mask_active {
            self.pre_quick_mask_active_layer = self.doc.active_layer_idx;
            let qm_idx = self.doc.add_layer("Quick Mask");
            if let Some(l) = self.doc.layers.get_mut(qm_idx) {
                if !self.doc.selection.is_empty() {
                    let w = self.doc.width;
                    let h = self.doc.height;
                    let red = [235, 45, 45, 128];
                    for y in 0..h {
                        for x in 0..w {
                            if self.doc.selection.contains(x, y) {
                                l.set_pixel(x, y, red);
                            }
                        }
                    }
                }
            }
            self.doc.selection.deselect();
            self.doc.active_layer_idx = qm_idx;
            self.quick_mask_active = true;
            self.doc.mark_full_dirty();
            self.status_message = "Quick Mask: ON (Paint mask with brush/eraser, press Ctrl+Q to exit)".to_string();
        } else {
            let qm_idx_opt = self.doc.layers.iter().position(|l| l.name == "Quick Mask");
            if let Some(qm_idx) = qm_idx_opt {
                let qm_layer = self.doc.layers.remove(qm_idx);
                let total = (self.doc.width * self.doc.height) as usize;
                let mut mask = vec![false; total];
                let mut any = false;
                for (i, px) in qm_layer.pixels.chunks_exact(4).enumerate() {
                    if px[3] > 10 {
                        mask[i] = true;
                        any = true;
                    }
                }
                if any {
                    self.doc.selection.mask = Some(mask);
                    self.doc.selection.recompute_bounds();
                } else {
                    self.doc.selection.deselect();
                }
            }
            self.doc.active_layer_idx = self.pre_quick_mask_active_layer.min(self.doc.layers.len() - 1);
            self.quick_mask_active = false;
            self.doc.mark_full_dirty();
            self.status_message = "Quick Mask: OFF (Converted painted mask to selection)".to_string();
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }

        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(Key::Q) {
                self.toggle_quick_mask();
            }
            if i.modifiers.command && i.key_pressed(Key::J) {
                self.duplicate_active_layer();
            }
            if i.modifiers.command && i.key_pressed(Key::E) {
                self.merge_active_layer_down();
            }
            if i.modifiers.command && i.key_pressed(Key::Z) {
                if i.modifiers.shift {
                    self.history.redo(&mut self.doc);
                } else {
                    self.history.undo(&mut self.doc);
                }
            }
            if i.modifiers.command && i.key_pressed(Key::Y) {
                self.history.redo(&mut self.doc);
            }
            if i.modifiers.command && i.key_pressed(Key::X) {
                self.cut_to_clipboard();
            }
            if i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::C) {
                self.copy_to_clipboard(true);
            }
            if i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::C) {
                self.copy_to_clipboard(false);
            }
            if i.modifiers.command && i.key_pressed(Key::V) {
                self.paste_from_clipboard();
            }
            if i.modifiers.command && i.key_pressed(Key::O) {
                self.open_file_dialog();
            }
            if i.modifiers.command && i.key_pressed(Key::S) {
                self.save_current_file();
            }
            if i.modifiers.command && i.key_pressed(Key::N) {
                self.new_dialog_open = true;
            }
            if i.modifiers.command && i.key_pressed(Key::W) {
                self.close_tab(self.active_tab);
            }
            if i.modifiers.command && i.key_pressed(Key::R) {
                self.viewport.show_rulers = !self.viewport.show_rulers;
            }
            if i.modifiers.command && i.key_pressed(Key::A) {
                self.doc.selection.select_all();
            }
            if i.modifiers.command && i.key_pressed(Key::D) {
                self.doc.selection.deselect();
            }
            if i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::I) {
                self.doc.selection.invert();
            }
            if i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::I) {
                self.apply_invert_colors();
            }
            if i.modifiers.command && i.key_pressed(Key::T) {
                if self.viewport.active_transform.is_some() {
                    self.viewport.commit_transform(&mut self.doc, &mut self.history);
                } else {
                    self.viewport.start_transform(&mut self.doc);
                }
            }
            if i.key_pressed(Key::Delete) {
                self.history.push_state("Clear Layer/Selection", &self.doc);
                self.doc.clear_active_selection_or_layer();
            }

            if !i.modifiers.any() {
                if i.key_pressed(Key::P) {
                    self.config.tool = ToolKind::Pen;
                    self.is_transparent_color = false;
                }
                if i.key_pressed(Key::B) {
                    self.config.tool = ToolKind::Brush;
                    self.is_transparent_color = false;
                }
                if i.key_pressed(Key::E) {
                    self.config.tool = ToolKind::Eraser;
                }
                if i.key_pressed(Key::G) {
                    self.config.tool = match self.config.tool {
                        ToolKind::Fill => ToolKind::CloseAndFill,
                        ToolKind::CloseAndFill => ToolKind::Fill,
                        _ => ToolKind::Fill,
                    };
                    self.is_transparent_color = false;
                }
                if i.key_pressed(Key::M) {
                    self.config.tool = match self.config.tool {
                        ToolKind::SelectionRect => ToolKind::SelectionEllipse,
                        ToolKind::SelectionEllipse => ToolKind::SelectionRect,
                        _ => ToolKind::SelectionRect,
                    };
                }
                if i.key_pressed(Key::W) {
                    self.config.tool = ToolKind::AutoSelect;
                }
                if i.key_pressed(Key::L) {
                    self.config.tool = ToolKind::SelectionLasso;
                }
                if i.key_pressed(Key::I) {
                    self.config.tool = ToolKind::Eyedropper;
                }
                if i.key_pressed(Key::Z) {
                    self.config.tool = ToolKind::Zoom;
                }
                if i.key_pressed(Key::U) {
                    self.config.tool = match self.config.tool {
                        ToolKind::Figure => ToolKind::ComicFrame,
                        ToolKind::ComicFrame => ToolKind::Ruler,
                        ToolKind::Ruler => ToolKind::Figure,
                        _ => ToolKind::Figure,
                    };
                    self.sub_tool_category = 0;
                    let default_st = match self.config.tool {
                        ToolKind::Figure => SubTool::FigureRect,
                        ToolKind::Ruler => SubTool::SymmetryRuler,
                        _ => SubTool::GPen,
                    };
                    self.config.select_sub_tool(default_st);
                }
                if i.key_pressed(Key::T) {
                    self.config.tool = ToolKind::SpeechBalloon;
                }
                if i.key_pressed(Key::X) {
                    std::mem::swap(
                        &mut self.config.primary_color,
                        &mut self.config.secondary_color,
                    );
                    self.is_transparent_color = false;
                }
                if i.key_pressed(Key::C) {
                    self.is_transparent_color = !self.is_transparent_color;
                    if self.is_transparent_color {
                        self.config.tool = ToolKind::Eraser;
                    } else {
                        self.config.tool = ToolKind::Pen;
                    }
                }
            }

            if i.key_pressed(Key::OpenBracket) {
                self.config.size = (self.config.size * 0.8).max(0.5);
            }
            if i.key_pressed(Key::CloseBracket) {
                self.config.size = (self.config.size * 1.25).min(300.0);
            }
        });

        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for df in dropped {
            if let Some(path) = df.path {
                self.load_file_path(path);
                break;
            }
        }
    }

    pub fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "CLIP STUDIO & Sub Tools (*.clip, *.sut, *.png, *.jpg, *.webp)",
                &["clip", "sut", "ccbrush", "png", "jpg", "jpeg", "webp", "bmp"],
            )
            .add_filter("CLIP STUDIO FORMAT (*.clip)", &["clip"])
            .add_filter("CLIP STUDIO SUB TOOL (*.sut)", &["sut", "ccbrush"])
            .pick_file()
        {
            self.load_file_path(path);
        }
    }

    pub fn load_file_path(&mut self, path: PathBuf) {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if matches!(ext.as_str(), "sut" | "ccbrush") {
            self.import_subtool_path(&path);
            return;
        }

        let res: Result<Document, String> = if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp") {
            match image::open(&path) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    let fname = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Imported.png")
                        .to_string();
                    let mut d = Document::new(fname.clone(), w, h, 300.0);
                    if let Some(layer) = d.layers.get_mut(1) {
                        layer.name = fname;
                        layer.pixels = rgba.into_raw();
                    }
                    d.update_composite();
                    Ok(d)
                }
                Err(e) => Err(format!("Image error: {e}")),
            }
        } else {
            open_clip_file(&path).map_err(|e| e.to_string())
        };

        match res {
            Ok(opened_doc) => {
                let opened_path = if ext == "clip" { Some(path.clone()) } else { None };
                if self.history.can_undo() || self.doc.layers.len() > 2 {
                    self.create_new_tab(opened_doc, opened_path);
                } else {
                    self.history = HistoryManager::new(25);
                    self.doc = opened_doc;
                    self.file_path = opened_path;
                    self.tabs[self.active_tab].title = self.doc.name.clone();
                    self.viewport.texture = None;
                    self.viewport.initialized = false;
                    self.viewport.layer_sig = 0;
                    self.doc.mark_full_dirty();
                }
                self.status_message = format!(
                    "Opened {} ({}×{} px, {} layers)",
                    path.display(),
                    self.doc.width,
                    self.doc.height,
                    self.doc.layers.len()
                );
            }
            Err(err) => {
                self.status_message = format!("Error opening {}: {}", path.display(), err);
            }
        }
    }

    pub fn open_import_subtool_dialog(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .set_title("Import Custom Sub Tool / Pencil (.sut, .png, .ccbrush)")
            .add_filter(
                "Clip Studio Sub Tool & Brush Tips (*.sut, *.png, *.jpg, *.webp, *.ccbrush, *.json)",
                &["sut", "ccbrush", "json", "png", "jpg", "jpeg", "webp", "bmp"],
            )
            .add_filter("CLIP STUDIO SUB TOOL (*.sut)", &["sut"])
            .add_filter("Brush Tip Image (*.png, *.jpg, *.webp)", &["png", "jpg", "jpeg", "webp", "bmp"])
            .pick_files()
        {
            for path in paths {
                self.import_subtool_path(&path);
            }
        }
    }

    pub fn import_subtool_path(&mut self, path: &std::path::Path) {
        match import_custom_subtool_file(path, self.config.tool) {
            Ok(custom) => {
                let name = custom.name.clone();
                let kind_name = custom.tool_kind.name();
                let has_tip = custom.tip_stamp.is_some();
                let has_paper = custom.paper_texture.is_some();
                self.sub_tool_category = 0;
                self.config.register_custom_sub_tool(custom);
                self.hardness_step = ((self.config.hardness * 5.0).round() as usize)
                    .saturating_sub(1)
                    .min(4);
                self.antialias_level = self.config.antialiasing as usize;
                self.save_persisted_custom_subtools();
                self.status_message = format!(
                    "Imported custom {kind_name} `{name}` (tip={}, paper_tex={})",
                    if has_tip { "custom" } else { "standard" },
                    if has_paper { "custom" } else { "procedural" }
                );
            }
            Err(e) => {
                self.status_message = format!("Failed to import sub tool {}: {}", path.display(), e);
            }
        }
    }

    pub fn duplicate_active_subtool(&mut self) {
        let base_name = self.config.active_sub_tool_name();
        let new_name = format!("{base_name} Copy");
        let preset = self.config.active_preset();
        let (tip_stamp, paper_texture) = if let SubTool::Custom(cid) = self.config.sub_tool {
            if let Some(c) = self.config.custom_sub_tools.iter().find(|c| c.id == cid) {
                (c.tip_stamp.clone(), c.paper_texture.clone())
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        let tool_kind = match self.config.tool {
            ToolKind::Pen | ToolKind::Pencil | ToolKind::Brush | ToolKind::Airbrush | ToolKind::Eraser => {
                self.config.tool
            }
            _ => ToolKind::Pencil,
        };
        let custom = CustomSubTool {
            id: 0,
            name: new_name.clone(),
            tool_kind,
            preset,
            antialiasing: self.config.antialiasing,
            tip_stamp,
            paper_texture,
            source_path: None,
        };
        self.config.register_custom_sub_tool(custom);
        self.save_persisted_custom_subtools();
        self.status_message = format!("Created custom sub tool `{new_name}`");
    }

    pub fn export_active_subtool_dialog(&mut self) {
        let name = self.config.active_sub_tool_name();
        let preset = self.config.active_preset();
        let (tip_stamp, paper_texture) = if let SubTool::Custom(cid) = self.config.sub_tool {
            if let Some(c) = self.config.custom_sub_tools.iter().find(|c| c.id == cid) {
                (c.tip_stamp.clone(), c.paper_texture.clone())
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        let custom = CustomSubTool {
            id: 1,
            name: name.clone(),
            tool_kind: self.config.tool,
            preset,
            antialiasing: self.config.antialiasing,
            tip_stamp,
            paper_texture,
            source_path: None,
        };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("ComicCraft Custom Sub Tool (*.ccbrush, *.json)", &["ccbrush", "json"])
            .set_file_name(format!("{name}.ccbrush"))
            .save_file()
        {
            match serde_json::to_vec_pretty(&custom) {
                Ok(bytes) => {
                    if let Err(e) = std::fs::write(&path, bytes) {
                        self.status_message = format!("Export failed: {e}");
                    } else {
                        self.status_message = format!("Exported sub tool `{name}` to {}", path.display());
                    }
                }
                Err(e) => {
                    self.status_message = format!("Serialize error: {e}");
                }
            }
        }
    }

    fn custom_subtools_store_path() -> Option<PathBuf> {
        let appdata = std::env::var_os("APPDATA")?;
        let dir = PathBuf::from(appdata).join("ArtCraft").join("ComicCraft");
        let _ = std::fs::create_dir_all(&dir);
        Some(dir.join("custom_subtools.json"))
    }

    fn load_persisted_custom_subtools(&mut self) {
        if let Some(store_path) = Self::custom_subtools_store_path() {
            if let Ok(bytes) = std::fs::read(&store_path) {
                if let Ok(list) = serde_json::from_slice::<Vec<CustomSubTool>>(&bytes) {
                    self.config.custom_sub_tools = list;
                }
            }
        }

        // Auto-import any .sut / .ccbrush files placed next to comiccraft.exe, in ./brushes, or in Downloads
        let mut scan_dirs = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                scan_dirs.push(exe_dir.to_path_buf());
                scan_dirs.push(exe_dir.join("brushes"));
            }
        }
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            scan_dirs.push(PathBuf::from(user_profile).join("Downloads"));
        }

        let mut added = false;
        for dir in scan_dirs {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let p = entry.path();
                let is_brush = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("sut") || e.eq_ignore_ascii_case("ccbrush"))
                    .unwrap_or(false);
                if !is_brush {
                    continue;
                }
                let stem = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let path_str = p.display().to_string();
                let already = self.config.custom_sub_tools.iter().any(|c| {
                    c.source_path.as_deref() == Some(path_str.as_str())
                        || (!stem.is_empty() && c.name.eq_ignore_ascii_case(&stem))
                });
                if !already {
                    if let Ok(custom) = import_custom_subtool_file(&p, ToolKind::Pencil) {
                        self.config.register_custom_sub_tool(custom);
                        added = true;
                    }
                }
            }
        }
        if added {
            self.save_persisted_custom_subtools();
        }
    }

    fn save_persisted_custom_subtools(&self) {
        if let Some(store_path) = Self::custom_subtools_store_path() {
            if let Ok(bytes) = serde_json::to_vec(&self.config.custom_sub_tools) {
                let _ = std::fs::write(store_path, bytes);
            }
        }
    }

    fn save_current_file(&mut self) {
        let path = match &self.file_path {
            Some(p) => p.clone(),
            None => {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("CLIP STUDIO FORMAT (*.clip)", &["clip"])
                    .set_file_name(&self.doc.name)
                    .save_file()
                {
                    self.file_path = Some(p.clone());
                    p
                } else {
                    return;
                }
            }
        };

        match save_clip_file(&self.doc, &path) {
            Ok(_) => {
                self.status_message = format!("Saved .clip document to {}", path.display());
            }
            Err(e) => {
                self.status_message = format!("Failed to save .clip: {}", e);
            }
        }
    }
}


pub fn setup_csp_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(Color32::from_rgb(222, 225, 232));
    visuals.panel_fill = Color32::from_rgb(43, 45, 50); // Authentic CSP dark slate
    visuals.window_fill = Color32::from_rgb(38, 40, 45);
    visuals.extreme_bg_color = Color32::from_rgb(25, 27, 30);
    visuals.selection.bg_fill = Color32::from_rgb(38, 108, 212);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::from_rgb(75, 150, 250));

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(52, 55, 61);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(215, 218, 224));
    visuals.widgets.inactive.rounding = Rounding::same(1.5);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(65, 69, 77);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.rounding = Rounding::same(1.5);

    visuals.widgets.active.bg_fill = Color32::from_rgb(38, 108, 212);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(1.5);

    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(43, 45, 50);
    visuals.widgets.noninteractive.fg_stroke =
        Stroke::new(1.0_f32, Color32::from_rgb(70, 74, 82));
    visuals.widgets.noninteractive.rounding = Rounding::same(1.5);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(3.0, 2.5);
    style.spacing.button_padding = Vec2::new(4.0, 2.0);
    style.spacing.window_margin = egui::Margin::same(4.0);
    ctx.set_style(style);
}

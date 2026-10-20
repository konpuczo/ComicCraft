use comiccraft_core::document::Document;
use comiccraft_core::ruler::{PerspectiveKind, PerspectiveRuler};
use comiccraft_core::selection::SelectionOp;
use comiccraft_core::stroke::StrokePoint;
use comiccraft_core::tools::{FillReferenceMode, SubTool, ToolConfig, ToolKind};

#[test]
fn test_layer_mask_compositing_and_apply() {
    let mut doc = Document::new("Mask Test", 50, 50, 300.0);
    // Layer 1 is active (index 1). Fill it with solid red.
    let red = [255, 0, 0, 255];
    for px in doc.layers[1].pixels.chunks_exact_mut(4) {
        px.copy_from_slice(&red);
    }
    doc.update_composite();
    // Verify top-left is red
    assert_eq!(doc.sample_color(10, 10), Some([255, 0, 0, 255]));

    // Add layer mask
    doc.add_layer_mask(1);
    assert!(doc.layers[1].mask.is_some());
    assert!(doc.layers[1].mask_enabled);

    // Mask out top-left region (mask value = 0)
    if let Some(mask) = doc.layers[1].mask.as_mut() {
        for y in 0..25 {
            for x in 0..25 {
                mask[y * 50 + x] = 0;
            }
        }
    }
    doc.mark_full_dirty();
    doc.update_composite();

    // In top-left (10, 10), red layer is masked out, so Paper (white) shows through!
    assert_eq!(doc.sample_color(10, 10), Some([255, 255, 255, 255]));
    // In bottom-right (35, 35), mask is 255, so red shows through!
    assert_eq!(doc.sample_color(35, 35), Some([255, 0, 0, 255]));

    // Apply layer mask (destructive bake into alpha)
    doc.apply_layer_mask(1);
    assert!(doc.layers[1].mask.is_none());
    assert_eq!(doc.layers[1].get_pixel(10, 10).unwrap()[3], 0);
    assert_eq!(doc.layers[1].get_pixel(35, 35).unwrap()[3], 255);
}

#[test]
fn test_layer_folder_hierarchy_visibility() {
    let mut doc = Document::new("Folder Test", 40, 40, 300.0);
    // Add folder
    let folder_idx = doc.add_folder("Group 1");
    let folder_id = doc.layers[folder_idx].id;

    // Add child layer inside folder
    let child_idx = doc.add_layer("Child");
    doc.layers[child_idx].parent = Some(folder_id);

    // Paint blue on child layer
    let blue = [0, 0, 255, 255];
    for px in doc.layers[child_idx].pixels.chunks_exact_mut(4) {
        px.copy_from_slice(&blue);
    }

    doc.update_composite();
    // Child is visible, so (10, 10) is blue
    assert_eq!(doc.sample_color(10, 10), Some([0, 0, 255, 255]));

    // Hide parent folder
    doc.layers[folder_idx].visible = false;
    doc.mark_full_dirty();
    doc.update_composite();

    // Now child is effectively hidden, so Paper (white) shows through!
    assert_eq!(doc.sample_color(10, 10), Some([255, 255, 255, 255]));
}

#[test]
fn test_vector_layer_stroke_and_vector_eraser() {
    let mut doc = Document::new("Vector Test", 100, 100, 300.0);
    let vec_idx = doc.add_vector_layer("Vector 1");
    assert!(doc.layers[vec_idx].is_vector());

    let mut cfg = ToolConfig::default();
    cfg.tool = ToolKind::Pen;
    cfg.sub_tool = SubTool::GPen;
    cfg.primary_color = [0, 0, 0, 255];
    cfg.size = 10.0;
    cfg.opacity = 1.0;

    // Draw vector stroke from (20, 50) to (80, 50)
    doc.begin_stroke(StrokePoint::new(20.0, 50.0, 1.0), &cfg);
    doc.continue_stroke(StrokePoint::new(50.0, 50.0, 1.0));
    doc.continue_stroke(StrokePoint::new(80.0, 50.0, 1.0));
    doc.end_stroke();

    assert_eq!(doc.layers[vec_idx].vector_strokes.len(), 1);
    doc.update_composite();
    // Midpoint (50, 50) has black ink
    let mid_color = doc.sample_color(50, 50).unwrap();
    assert_eq!(mid_color[0], 0);
    assert_eq!(mid_color[1], 0);
    assert_eq!(mid_color[2], 0);

    // Vector erase near midpoint (52, 50) with radius 5
    let hit = doc.erase_vector_at(52.0, 50.0, 5.0);
    assert!(hit);
    assert_eq!(doc.layers[vec_idx].vector_strokes.len(), 0);

    doc.update_composite();
    // Midpoint is now white paper
    assert_eq!(doc.sample_color(50, 50), Some([255, 255, 255, 255]));
}

#[test]
fn test_auto_select_wand_and_reference_layer() {
    let mut doc = Document::new("Wand Test", 100, 100, 300.0);
    // Draw a square lineart barrier on layer 1 (active)
    // from x=20..80, y=20..80
    for x in 20..=80 {
        doc.layers[1].set_pixel(x, 20, [0, 0, 0, 255]);
        doc.layers[1].set_pixel(x, 80, [0, 0, 0, 255]);
    }
    for y in 20..=80 {
        doc.layers[1].set_pixel(20, y, [0, 0, 0, 255]);
        doc.layers[1].set_pixel(80, y, [0, 0, 0, 255]);
    }
    // Set layer 1 as Reference Layer (Lighthouse)
    doc.layers[1].is_reference = true;

    // Add a paint layer above it
    let _paint_idx = doc.add_layer("Color Layer");

    let mut cfg = ToolConfig::default();
    cfg.tool = ToolKind::AutoSelect;
    cfg.fill_mode = FillReferenceMode::ReferReferenceLayer;
    cfg.wand_tolerance = 10;
    cfg.wand_contiguous = true;
    cfg.selection_op = SelectionOp::Replace;

    // Click inside the box at (50, 50)
    doc.auto_select(50, 50, &cfg);

    // Inside must be selected
    assert!(doc.selection.contains(50, 50));
    assert!(doc.selection.contains(40, 40));
    // Outside the box must NOT be selected
    assert!(!doc.selection.contains(10, 10));
    assert!(!doc.selection.contains(90, 90));

    // Now fill inside the selection with green
    let mut fill_cfg = ToolConfig::default();
    fill_cfg.tool = ToolKind::Fill;
    fill_cfg.primary_color = [0, 255, 0, 255];
    fill_cfg.fill_mode = FillReferenceMode::ReferReferenceLayer;

    doc.flood_fill(50, 50, &fill_cfg);
    doc.update_composite();

    assert_eq!(doc.sample_color(50, 50), Some([0, 255, 0, 255]));
    // Outside is still white paper
    assert_eq!(doc.sample_color(10, 10), Some([255, 255, 255, 255]));
}

#[test]
fn test_gradient_map_levels_and_binarization() {
    use comiccraft_core::filter::{apply_binarization, apply_gradient_map, apply_levels, GradientMap};
    let mut layer = comiccraft_core::layer::Layer::new(1, "Test", 10, 10);
    let sel = comiccraft_core::selection::Selection::new(10, 10);

    // Set pixel (0, 0) to 50% gray
    layer.set_pixel(0, 0, [128, 128, 128, 255]);
    // Set pixel (1, 1) to dark gray
    layer.set_pixel(1, 1, [40, 40, 40, 255]);

    // Apply sunset gradient map
    let map = GradientMap::preset_sunset();
    apply_gradient_map(&mut layer, &sel, &map);

    let mapped = layer.get_pixel(0, 0).unwrap();
    // Midtone should map into the warm sunset range (reddish/orange)
    assert!(mapped[0] > 180);
    assert_eq!(mapped[3], 255);

    // Test binarization (threshold 100)
    layer.set_pixel(2, 2, [50, 50, 50, 255]);
    layer.set_pixel(3, 3, [150, 150, 150, 255]);
    apply_binarization(&mut layer, &sel, 100);

    assert_eq!(layer.get_pixel(2, 2).unwrap(), [0, 0, 0, 255]);
    assert_eq!(layer.get_pixel(3, 3).unwrap(), [255, 255, 255, 255]);

    // Test levels
    layer.set_pixel(4, 4, [100, 100, 100, 255]);
    apply_levels(&mut layer, &sel, 50, 200, 1.0, 0, 255);
    let leveled = layer.get_pixel(4, 4).unwrap();
    assert!(leveled[0] > 0 && leveled[0] < 255);
}

#[test]
fn test_perspective_ruler_snapping() {
    let ruler = PerspectiveRuler::new_two_point(1000.0, 800.0);
    assert!(ruler.enabled);
    assert_eq!(ruler.kind, PerspectiveKind::TwoPoint);

    // Test vertical snapping
    let start = StrokePoint::new(500.0, 200.0, 1.0);
    let raw_down = StrokePoint::new(502.5, 350.0, 1.0); // Slight horizontal wobble
    let snapped_down = ruler.snap_point(start, raw_down);
    // X must snap strictly to start.x (vertical line)
    assert!((snapped_down.x - 500.0).abs() < 1e-4);
    assert!((snapped_down.y - 350.0).abs() < 1e-4);

    // Test VP1 snapping along the horizon
    let start_h = StrokePoint::new(500.0, 400.0, 1.0);
    let raw_left = StrokePoint::new(300.0, 403.0, 1.0); // Slight vertical wobble
    let snapped_left = ruler.snap_point(start_h, raw_left);
    // Y must snap back onto horizon
    assert!((snapped_left.y - 400.0).abs() < 1e-4);

    // Test in Document stroke engine
    let mut doc = Document::new("Persp Test", 200, 200, 300.0);
    doc.perspective_ruler.enabled = true;
    doc.perspective_ruler.snap_to_perspective = true;
    doc.perspective_ruler.eye_level_y = 100.0;
    doc.perspective_ruler.vp1_x = -100.0;
    doc.perspective_ruler.vp1_y = 100.0;
    doc.perspective_ruler.vp2_x = 300.0;
    doc.perspective_ruler.vp2_y = 100.0;

    let mut cfg = ToolConfig::default();
    cfg.stabilization = 0;
    doc.begin_stroke(StrokePoint::new(100.0, 50.0, 1.0), &cfg);
    // User moves mouse down with horizontal jitter
    doc.continue_stroke(StrokePoint::new(102.0, 80.0, 1.0));
    doc.end_stroke();

    // Verify pixel drawn at (100, 65) (midpoint of vertical line) is painted
    let p = doc.layers[1].get_pixel(100, 65).unwrap();
    assert_eq!(p[3], 255); // Solid ink
}

#[test]
fn test_selection_border_and_grow() {
    use comiccraft_core::selection::SelectionBorderPosition;

    let mut doc = Document::new("Border Test", 50, 50, 300.0);
    // Select rect (10, 10) to (30, 30)
    doc.selection.select_rect(10, 10, 30, 30);
    assert!(!doc.selection.is_empty());

    // Stroke border: 2px Inside
    let stroke_color = [20, 20, 24, 255];
    let ok = doc.stroke_selection_border(2, SelectionBorderPosition::Inside, stroke_color);
    assert!(ok);

    // Boundary pixel at (10, 10) should have the border stroke
    let border_px = doc.layers[1].get_pixel(10, 10).unwrap();
    assert_eq!(border_px, stroke_color);

    // Deep interior at (20, 20) should NOT be stroked
    let inside_px = doc.layers[1].get_pixel(20, 20).unwrap();
    assert_eq!(inside_px[3], 0);

    // Test selection grow
    let old_bounds = doc.selection.bounds.unwrap();
    doc.selection.grow(3);
    let new_bounds = doc.selection.bounds.unwrap();
    assert!(new_bounds.0 < old_bounds.0); // Expanded outward

    doc.selection.grow(-3);
    let shrunk_bounds = doc.selection.bounds.unwrap();
    assert_eq!(shrunk_bounds.0, old_bounds.0);
}

#[test]
fn test_convert_brightness_to_opacity() {
    let mut doc = Document::new("Lineart Extract", 40, 40, 300.0);
    // Fill with white paper background
    for px in doc.layers[1].pixels.chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 255, 255, 255]);
    }
    // Draw black ink line at (20, y)
    for y in 10..30 {
        doc.layers[1].set_pixel(20, y, [0, 0, 0, 255]);
    }

    let ok = doc.convert_brightness_to_opacity();
    assert!(ok);

    // White paper at (5, 5) should be completely transparent (alpha = 0)
    let paper_px = doc.layers[1].get_pixel(5, 5).unwrap();
    assert_eq!(paper_px[3], 0);

    // Black ink line at (20, 15) should be completely opaque black ink (alpha = 255)
    let ink_px = doc.layers[1].get_pixel(20, 15).unwrap();
    assert_eq!(ink_px[3], 255);
    assert_eq!(ink_px[0], 0);
    assert_eq!(ink_px[1], 0);
    assert_eq!(ink_px[2], 0);
}

#[test]
fn test_layer_duplicate_and_merge_down() {
    let mut doc = Document::new("Dup Merge", 30, 30, 300.0);
    // Paint Layer 1 red
    for px in doc.layers[1].pixels.chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 0, 0, 255]);
    }

    // Duplicate Layer 1
    let dup_idx = doc.duplicate_layer(1).unwrap();
    assert_eq!(dup_idx, 2);
    assert_eq!(doc.layers.len(), 3);
    assert_eq!(doc.layers[2].name, "Layer 1 copy");

    // Paint Layer 2 blue on top-half
    for y in 0..15 {
        for x in 0..30 {
            doc.layers[2].set_pixel(x, y, [0, 0, 255, 255]);
        }
    }

    // Merge Layer 2 down into Layer 1
    let merged_ok = doc.merge_down(2);
    assert!(merged_ok);
    assert_eq!(doc.layers.len(), 2);

    // Top half should be blue
    assert_eq!(doc.layers[1].get_pixel(10, 5).unwrap(), [0, 0, 255, 255]);
    // Bottom half should be red
    assert_eq!(doc.layers[1].get_pixel(10, 25).unwrap(), [255, 0, 0, 255]);
}

#[test]
fn test_vector_simplify_and_width_adjustment() {
    let mut doc = Document::new("Vec Edit", 100, 100, 300.0);
    let v_idx = doc.add_vector_layer("Vector Layer");

    let mut cfg = ToolConfig::default();
    cfg.tool = ToolKind::Pen;
    cfg.sub_tool = SubTool::GPen;
    cfg.size = 6.0;

    doc.begin_stroke(StrokePoint::new(10.0, 10.0, 1.0), &cfg);
    // Collinear points along straight line
    doc.continue_stroke(StrokePoint::new(20.0, 20.0, 1.0));
    doc.continue_stroke(StrokePoint::new(30.0, 30.0, 1.0));
    doc.continue_stroke(StrokePoint::new(40.0, 40.0, 1.0));
    doc.continue_stroke(StrokePoint::new(50.0, 50.0, 1.0));
    doc.end_stroke();

    assert_eq!(doc.layers[v_idx].vector_strokes[0].points.len(), 5);

    // Simplify collinear points with tolerance 1.0
    doc.simplify_vector_layer(v_idx, 1.0);
    assert_eq!(doc.layers[v_idx].vector_strokes[0].points.len(), 2);

    // Alter line width by +4.0 px
    doc.adjust_vector_line_width(v_idx, 4.0);
    assert_eq!(doc.layers[v_idx].vector_strokes[0].style.size, 10.0);
}


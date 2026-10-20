use comiccraft_core::filter::{
    apply_brightness_contrast, apply_gaussian_blur, apply_hue_saturation_luminosity, apply_invert,
};
use comiccraft_core::layer::Layer;
use comiccraft_core::ruler::SymmetryRuler;
use comiccraft_core::selection::Selection;
use comiccraft_core::speed_lines::{
    draw_ellipse_outline, draw_rect_outline, draw_straight_line, render_focus_lines,
    render_stream_lines, FocusLinesConfig, StreamLinesConfig,
};
use comiccraft_core::stroke::StrokePoint;
use comiccraft_core::transform::TransformState;

#[test]
fn test_symmetry_ruler_reflections() {
    // 1. Bilateral symmetry (2 lines / 1 reflection axis)
    let ruler_bilateral = SymmetryRuler {
        enabled: true,
        center_x: 100.0,
        center_y: 100.0,
        angle_deg: 0.0,
        lines: 2,
        line_symmetric: true,
    };
    let pts = ruler_bilateral.symmetrical_counterparts(StrokePoint::new(120.0, 100.0, 1.0));
    assert_eq!(pts.len(), 1);
    // Reflected counterpart across vertical axis at x=100 should be (80, 100)
    assert!((pts[0].x - 80.0).abs() < 1e-4);
    assert!((pts[0].y - 100.0).abs() < 1e-4);

    // 2. Radial Mandala symmetry (8 lines)
    let ruler_mandala = SymmetryRuler {
        enabled: true,
        center_x: 200.0,
        center_y: 200.0,
        angle_deg: 0.0,
        lines: 8,
        line_symmetric: false,
    };
    let pts8 = ruler_mandala.symmetrical_counterparts(StrokePoint::new(250.0, 200.0, 1.0));
    assert_eq!(pts8.len(), 7);
    // All 7 counterpart points must be distance 50 from center (200, 200)
    for p in pts8 {
        let dist = ((p.x - 200.0).powi(2) + (p.y - 200.0).powi(2)).sqrt();
        assert!((dist - 50.0).abs() < 1e-3);
    }
}

#[test]
fn test_image_filters() {
    let mut layer = Layer::new(1, "Filter Test Layer", 100, 100);
    let sel = Selection::new(100, 100); // Unconstrained selection

    // Fill center 20x20 with midtone gray (128, 128, 128, 255)
    for y in 40..60 {
        for x in 40..60 {
            let idx = ((y * 100 + x) * 4) as usize;
            layer.pixels[idx] = 128;
            layer.pixels[idx + 1] = 128;
            layer.pixels[idx + 2] = 128;
            layer.pixels[idx + 3] = 255;
        }
    }

    // Test Invert: 128 -> 127
    apply_invert(&mut layer, &sel);
    let sample_idx = ((50 * 100 + 50) * 4) as usize;
    assert_eq!(layer.pixels[sample_idx], 127);
    assert_eq!(layer.pixels[sample_idx + 3], 255);

    // Test Brightness/Contrast
    apply_brightness_contrast(&mut layer, &sel, 50, 0);
    let bright_val = layer.pixels[sample_idx];
    assert!(bright_val > 127);

    // Test Hue/Saturation/Luminosity
    apply_hue_saturation_luminosity(&mut layer, &sel, 180.0, 50.0, 10.0);
    assert!(layer.pixels[sample_idx + 3] == 255);

    // Test Gaussian Blur spreads alpha and color to neighboring pixels
    let before_alpha = layer.pixels[((35 * 100 + 35) * 4) + 3];
    assert_eq!(before_alpha, 0);
    apply_gaussian_blur(&mut layer, &sel, 4.0);
    let after_alpha = layer.pixels[((38 * 100 + 38) * 4) + 3];
    assert!(after_alpha > 0);
}

#[test]
fn test_manga_speed_lines_and_direct_draw() {
    let mut layer = Layer::new(2, "SpeedLines Layer", 200, 200);
    let sel = Selection::new(200, 200);

    // 1. Direct draw straight line
    draw_straight_line(
        &mut layer,
        &sel,
        10.0,
        10.0,
        90.0,
        10.0,
        3.0,
        [255, 0, 0, 255],
    );
    let line_mid = ((10 * 200 + 50) * 4) as usize;
    assert_eq!(layer.pixels[line_mid], 255);
    assert_eq!(layer.pixels[line_mid + 3], 255);

    // 2. Direct draw rectangle outline
    draw_rect_outline(
        &mut layer,
        &sel,
        10.0,
        20.0,
        60.0,
        80.0,
        2.0,
        [0, 255, 0, 255],
    );
    let rect_edge = ((20 * 200 + 35) * 4) as usize;
    assert_eq!(layer.pixels[rect_edge + 1], 255);

    // 3. Direct draw ellipse outline
    draw_ellipse_outline(
        &mut layer,
        &sel,
        60.0,
        60.0,
        140.0,
        140.0,
        2.0,
        [0, 0, 255, 255],
    );
    let ellipse_top = ((60 * 200 + 100) * 4) as usize;
    assert_eq!(layer.pixels[ellipse_top + 2], 255);

    // 4. Manga Radiation Focus Lines
    let focus_cfg = FocusLinesConfig {
        center_x: 100.0,
        center_y: 100.0,
        inner_radius: 30.0,
        outer_radius: 90.0,
        count: 40,
        thickness: 2.0,
        jitter: 0.0,
        color: [20, 20, 25, 255],
    };
    render_focus_lines(&mut layer, &sel, &focus_cfg);
    // Center point (100, 100) should remain clear (inner radius 30)
    let center_px = ((100 * 200 + 100) * 4) as usize;
    assert_eq!(layer.pixels[center_px + 3], 0);

    // 5. Directional Speed Lines
    let stream_cfg = StreamLinesConfig {
        angle_deg: 45.0,
        count: 30,
        length: 80.0,
        thickness: 2.0,
        jitter: 0.0,
        color: [20, 20, 25, 255],
    };
    render_stream_lines(&mut layer, &sel, &stream_cfg);
}

#[test]
fn test_free_transform_commit() {
    let mut layer = Layer::new(3, "Transform Layer", 100, 100);
    let mut sel = Selection::new(100, 100);

    // Select 20x20 square from (30, 30) to (50, 50) and fill it red
    for y in 30..50 {
        for x in 30..50 {
            let idx = ((y * 100 + x) * 4) as usize;
            layer.pixels[idx] = 255;
            layer.pixels[idx + 1] = 0;
            layer.pixels[idx + 2] = 0;
            layer.pixels[idx + 3] = 255;
        }
    }
    sel.select_rect(30, 30, 49, 49);

    // Begin transform: cuts pixels out of layer
    let mut tf = TransformState::begin(&mut layer, &sel).expect("TransformState created");
    assert_eq!(tf.patch_w, 20);
    assert_eq!(tf.patch_h, 20);

    // Check that original pixels were cut from layer
    let cut_idx = ((40 * 100 + 40) * 4) as usize;
    assert_eq!(layer.pixels[cut_idx + 3], 0);

    // Translate by +20, +20 and commit
    tf.translate_x = 20.0;
    tf.translate_y = 20.0;
    tf.commit(&mut layer);

    // Committed position around (60, 60) should now have red pixels
    let new_idx = ((60 * 100 + 60) * 4) as usize;
    assert_eq!(layer.pixels[new_idx], 255);
    assert_eq!(layer.pixels[new_idx + 3], 255);
}

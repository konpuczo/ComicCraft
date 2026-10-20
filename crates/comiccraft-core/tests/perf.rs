use comiccraft_core::blend::BlendMode;
use comiccraft_core::document::Document;
use comiccraft_core::stroke::StrokePoint;
use comiccraft_core::tools::ToolConfig;
use std::time::Instant;

fn build_doc() -> Document {
    let mut doc = Document::new("perf", 1600, 2400, 300.0);
    // Clipped multiply layer on top of the ink layer exercises the clipping-mask path.
    let idx = doc.add_layer("shade");
    doc.layers[idx].clipped_to_below = true;
    doc.layers[idx].blend_mode = BlendMode::Multiply;
    doc.layers[idx].opacity = 0.7;
    for _ in 0..3 {
        doc.add_layer("extra");
    }
    doc
}

#[test]
fn partial_composite_matches_full() {
    let mut doc = build_doc();
    doc.update_composite();

    // Paint on the shade layer (clipped) and on the ink layer.
    let cfg = ToolConfig::default();
    for layer in [1usize, 2usize] {
        doc.active_layer_idx = layer;
        for i in 0..60 {
            let a = StrokePoint::new(200.0 + i as f32 * 5.0, 700.0 + i as f32, 0.9);
            let b = StrokePoint::new(205.0 + i as f32 * 5.0, 701.0 + i as f32, 0.9);
            doc.paint_stroke_segment(a, b, &cfg);
        }
    }
    assert!(doc.dirty_rect.is_some(), "stroke should produce a partial dirty rect");
    let region = doc.refresh_composite();
    assert!(region.is_some(), "expected partial recomposite");
    let partial = doc.composite_cache.clone().unwrap();

    doc.update_composite();
    let full = doc.composite_cache.clone().unwrap();
    assert!(partial == full, "partial recomposite differs from full recomposite");
}

#[test]
fn composite_and_stroke_timing() {
    let mut doc = build_doc();
    let t = Instant::now();
    for _ in 0..10 {
        doc.update_composite();
    }
    println!("full composite (6 layers, 1600x2400): {:?} per frame", t.elapsed() / 10);

    let cfg = ToolConfig::default();
    doc.active_layer_idx = 1;
    let t = Instant::now();
    let mut frames = 0;
    for i in 0..200 {
        let a = StrokePoint::new(100.0 + i as f32 * 3.0, 500.0, 0.8);
        let b = StrokePoint::new(103.0 + i as f32 * 3.0, 502.0, 0.8);
        doc.paint_stroke_segment(a, b, &cfg);
        doc.refresh_composite(); // one frame per segment, like the live viewport
        frames += 1;
    }
    println!("{} drawing frames (stroke + partial composite): {:?} total, {:?} per frame", frames, t.elapsed(), t.elapsed() / frames);
}

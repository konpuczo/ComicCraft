use comiccraft_clip::{open_clip_file, save_clip_file};
use comiccraft_core::blend::BlendMode;
use comiccraft_core::document::Document;
use comiccraft_core::layer::Layer;
use std::path::Path;

#[test]
fn test_clip_format_roundtrip() {
    let width = 512;
    let height = 512;
    let mut doc = Document::new("TestDocument", width, height, 350.0);
    doc.layers.clear();

    // Layer 1: Solid background (White)
    let mut l1 = Layer::new(1, "Background", width, height);
    l1.pixels.fill(255);
    l1.blend_mode = BlendMode::Normal;
    l1.opacity = 1.0;

    // Layer 2: Colored square (Red) with Multiply blend mode
    let mut l2 = Layer::new(2, "Ink Stroke", width, height);
    for y in 100..400 {
        for x in 100..400 {
            let idx = ((y * width + x) * 4) as usize;
            l2.pixels[idx] = 255;     // R
            l2.pixels[idx + 1] = 50;  // G
            l2.pixels[idx + 2] = 50;  // B
            l2.pixels[idx + 3] = 255; // A
        }
    }
    l2.blend_mode = BlendMode::Multiply;
    l2.opacity = 0.85;

    // Layer 3: Screen highlight (Blue) with 50% opacity
    let mut l3 = Layer::new(3, "Highlight", width, height);
    for y in 200..300 {
        for x in 200..300 {
            let idx = ((y * width + x) * 4) as usize;
            l3.pixels[idx] = 30;      // R
            l3.pixels[idx + 1] = 100; // G
            l3.pixels[idx + 2] = 240; // B
            l3.pixels[idx + 3] = 180; // A
        }
    }
    l3.blend_mode = BlendMode::Screen;
    l3.opacity = 0.50;

    doc.layers.push(l1);
    doc.layers.push(l2);
    doc.layers.push(l3);
    doc.refresh_composite();

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("comiccraft_test_auth.clip");

    // 1. Save .clip file
    save_clip_file(&doc, &test_file).expect("Failed to save .clip file");
    assert!(test_file.exists());
    let file_size = std::fs::metadata(&test_file).unwrap().len();
    assert!(file_size > 1000);

    // 2. Open .clip file
    let loaded = open_clip_file(&test_file).expect("Failed to open saved .clip file");
    assert_eq!(loaded.width, width);
    assert_eq!(loaded.height, height);
    assert_eq!(loaded.dpi, 350.0);
    assert_eq!(loaded.layers.len(), 3);

    assert_eq!(loaded.layers[0].name, "Background");
    assert_eq!(loaded.layers[0].blend_mode, BlendMode::Normal);
    assert!((loaded.layers[0].opacity - 1.0).abs() < 0.01);

    assert_eq!(loaded.layers[1].name, "Ink Stroke");
    assert_eq!(loaded.layers[1].blend_mode, BlendMode::Multiply);
    assert!((loaded.layers[1].opacity - 0.85).abs() < 0.02);

    assert_eq!(loaded.layers[2].name, "Highlight");
    assert_eq!(loaded.layers[2].blend_mode, BlendMode::Screen);
    assert!((loaded.layers[2].opacity - 0.50).abs() < 0.02);

    // Verify pixel data of ink stroke
    let stroke_pixel_idx = ((250 * width + 250) * 4) as usize;
    assert_eq!(loaded.layers[1].pixels[stroke_pixel_idx], 255);
    assert_eq!(loaded.layers[1].pixels[stroke_pixel_idx + 1], 50);
    assert_eq!(loaded.layers[1].pixels[stroke_pixel_idx + 2], 50);
    assert_eq!(loaded.layers[1].pixels[stroke_pixel_idx + 3], 255);

    // Clean up
    let _ = std::fs::remove_file(test_file);
}

#[test]
fn test_external_clip_to_psd_compatibility() {
    let width = 256;
    let height = 256;
    let mut doc = Document::new("CSP_Compat_Doc", width, height, 300.0);
    doc.layers.clear();

    let mut l1 = Layer::new(1, "BaseLayer", width, height);
    l1.pixels.fill(200);
    l1.blend_mode = BlendMode::Normal;
    l1.opacity = 1.0;

    let mut l2 = Layer::new(2, "DarkLayer", width, height);
    for i in 0..width * height {
        let idx = (i * 4) as usize;
        l2.pixels[idx] = 10;
        l2.pixels[idx + 1] = 20;
        l2.pixels[idx + 2] = 30;
        l2.pixels[idx + 3] = 255;
    }
    l2.blend_mode = BlendMode::Multiply;
    l2.opacity = 0.75;

    doc.layers.push(l1);
    doc.layers.push(l2);
    doc.refresh_composite();

    let temp_dir = std::env::temp_dir();
    let test_clip = temp_dir.join("comiccraft_for_reference.clip");
    let test_psd = temp_dir.join("comiccraft_for_reference.psd");

    save_clip_file(&doc, &test_clip).expect("Failed to save .clip");

    let script = Path::new(r"C:\ArtCraft\reference\clip_to_psd.py");
    if script.exists() {
        let status = std::process::Command::new("python")
            .arg(&script)
            .arg(&test_clip)
            .arg("-o")
            .arg(&test_psd)
            .arg("--blank-psd-preview")
            .arg("--log-level")
            .arg("DEBUG")
            .status()
            .expect("Failed to run clip_to_psd.py");

        assert!(status.success(), "clip_to_psd.py failed to convert ComicCraft .clip file");
        assert!(test_psd.exists(), "Output PSD was not produced");
        let psd_size = std::fs::metadata(&test_psd).unwrap().len();
        assert!(psd_size > 100, "Produced PSD is empty");
        let _ = std::fs::remove_file(&test_psd);
    }

    let _ = std::fs::remove_file(&test_clip);
}

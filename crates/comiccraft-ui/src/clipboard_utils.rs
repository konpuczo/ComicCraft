use comiccraft_core::Document;
use std::borrow::Cow;

/// Copies the current selection (or active layer / merged canvas) to the system clipboard.
pub fn copy_selection_or_layer_to_clipboard(
    doc: &mut Document,
    merged: bool,
) -> Result<String, String> {
    let (buf, w, h) = if merged {
        if doc.composite_cache.is_none() || doc.is_dirty {
            doc.update_composite();
        }
        let comp = doc
            .composite_cache
            .as_ref()
            .cloned()
            .unwrap_or_default();
        if doc.selection.is_empty() {
            (comp, doc.width, doc.height)
        } else if let Some((min_x, min_y, max_x, max_y)) = doc.selection.bounds {
            let sw = (max_x.saturating_sub(min_x) + 1) as usize;
            let sh = (max_y.saturating_sub(min_y) + 1) as usize;
            let mut out = vec![0u8; sw * sh * 4];
            let dw = doc.width as usize;
            for y in min_y..=max_y {
                let dy = (y - min_y) as usize;
                for x in min_x..=max_x {
                    if doc.selection.contains(x, y) {
                        let dx = (x - min_x) as usize;
                        let src_idx = (y as usize * dw + x as usize) * 4;
                        let dst_idx = (dy * sw + dx) * 4;
                        if src_idx + 4 <= comp.len() && dst_idx + 4 <= out.len() {
                            out[dst_idx..dst_idx + 4].copy_from_slice(&comp[src_idx..src_idx + 4]);
                        }
                    }
                }
            }
            (out, sw as u32, sh as u32)
        } else {
            (comp, doc.width, doc.height)
        }
    } else {
        let layer = doc
            .layers
            .get(doc.active_layer_idx)
            .ok_or_else(|| "No active layer to copy".to_string())?;
        if doc.selection.is_empty() {
            (layer.pixels.clone(), layer.width, layer.height)
        } else if let Some((min_x, min_y, max_x, max_y)) = doc.selection.bounds {
            let sw = (max_x.saturating_sub(min_x) + 1) as usize;
            let sh = (max_y.saturating_sub(min_y) + 1) as usize;
            let mut out = vec![0u8; sw * sh * 4];
            let lw = layer.width as usize;
            for y in min_y..=max_y {
                let dy = (y - min_y) as usize;
                for x in min_x..=max_x {
                    if doc.selection.contains(x, y) {
                        let dx = (x - min_x) as usize;
                        let src_idx = (y as usize * lw + x as usize) * 4;
                        let dst_idx = (dy * sw + dx) * 4;
                        if src_idx + 4 <= layer.pixels.len() && dst_idx + 4 <= out.len() {
                            out[dst_idx..dst_idx + 4].copy_from_slice(&layer.pixels[src_idx..src_idx + 4]);
                        }
                    }
                }
            }
            (out, sw as u32, sh as u32)
        } else {
            (layer.pixels.clone(), layer.width, layer.height)
        }
    };

    if w == 0 || h == 0 || buf.is_empty() {
        return Err("Nothing to copy".to_string());
    }

    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| format!("Clipboard error: {e}"))?;

    let img_data = arboard::ImageData {
        width: w as usize,
        height: h as usize,
        bytes: Cow::Borrowed(&buf),
    };

    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Failed to set clipboard image: {e}"))?;

    if merged {
        Ok(format!("Copied merged image ({}x{}) to clipboard", w, h))
    } else {
        Ok(format!("Copied layer image ({}x{}) to clipboard", w, h))
    }
}

/// Retrieves an image from the system clipboard, either directly as bitmap data
/// or by reading a file path if the user copied an image file.
pub fn get_clipboard_image() -> Result<(u32, u32, Vec<u8>), String> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| format!("Clipboard error: {e}"))?;

    if let Ok(img) = clipboard.get_image() {
        let w = img.width as u32;
        let h = img.height as u32;
        if w > 0 && h > 0 {
            return Ok((w, h, img.bytes.to_vec()));
        }
    }

    // Fallback: check if clipboard contains a valid file path to an image file
    if let Ok(text) = clipboard.get_text() {
        let trimmed = text.trim().trim_matches('"');
        let path = std::path::Path::new(trimmed);
        if path.is_file() {
            if let Ok(img) = image::open(path) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                if w > 0 && h > 0 {
                    return Ok((w, h, rgba.into_raw()));
                }
            }
        }
    }

    Err("No image found in clipboard".to_string())
}

/// Pastes raw RGBA pixels onto a new raster layer in the document, centered on the canvas.
pub fn paste_rgba_into_document(
    doc: &mut Document,
    img_w: u32,
    img_h: u32,
    bytes: &[u8],
) -> usize {
    let layer_name = format!("Pasted Layer {}", doc.layers.len() + 1);
    let l_idx = doc.add_layer(layer_name);

    let doc_w = doc.width;
    let doc_h = doc.height;
    let start_x = (doc_w as i64 - img_w as i64) / 2;
    let start_y = (doc_h as i64 - img_h as i64) / 2;

    if let Some(layer) = doc.layers.get_mut(l_idx) {
        let (dst_x0, src_x0, copy_w) = if start_x < 0 {
            (0u32, (-start_x) as u32, img_w.saturating_sub((-start_x) as u32).min(doc_w))
        } else {
            (start_x as u32, 0u32, (doc_w - start_x as u32).min(img_w))
        };
        let (dst_y0, src_y0, copy_h) = if start_y < 0 {
            (0u32, (-start_y) as u32, img_h.saturating_sub((-start_y) as u32).min(doc_h))
        } else {
            (start_y as u32, 0u32, (doc_h - start_y as u32).min(img_h))
        };

        for r in 0..copy_h {
            let sy = src_y0 + r;
            let dy = dst_y0 + r;
            let src_start = ((sy * img_w + src_x0) * 4) as usize;
            let src_end = src_start + (copy_w * 4) as usize;
            let dst_start = ((dy * doc_w + dst_x0) * 4) as usize;
            let dst_end = dst_start + (copy_w * 4) as usize;
            if src_end <= bytes.len() && dst_end <= layer.pixels.len() {
                layer.pixels[dst_start..dst_end].copy_from_slice(&bytes[src_start..src_end]);
            }
        }
    }

    doc.active_layer_idx = l_idx;
    doc.mark_full_dirty();
    doc.update_composite();

    l_idx
}

/// Pastes an image from the clipboard onto a new raster layer in the document,
/// centered on the canvas.
pub fn paste_image_into_document(doc: &mut Document) -> Result<(usize, u32, u32), String> {
    let (img_w, img_h, bytes) = get_clipboard_image()?;
    let l_idx = paste_rgba_into_document(doc, img_w, img_h, &bytes);
    Ok((l_idx, img_w, img_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paste_rgba_centered() {
        let mut doc = Document::new("Test", 100, 100, 300.0);
        let initial_layers = doc.layers.len();

        let img_w = 20;
        let img_h = 20;
        let bytes = vec![255, 0, 0, 255].repeat((img_w * img_h) as usize);

        let l_idx = paste_rgba_into_document(&mut doc, img_w, img_h, &bytes);
        assert_eq!(doc.layers.len(), initial_layers + 1);
        assert_eq!(doc.active_layer_idx, l_idx);

        let layer = &doc.layers[l_idx];
        assert_eq!(layer.get_pixel(50, 50), Some([255, 0, 0, 255]));
        assert_eq!(layer.get_pixel(40, 40), Some([255, 0, 0, 255]));
        assert_eq!(layer.get_pixel(10, 10), Some([0, 0, 0, 0]));
        assert_eq!(layer.get_pixel(80, 80), Some([0, 0, 0, 0]));
    }

    #[test]
    fn test_paste_rgba_oversized() {
        let mut doc = Document::new("Test", 50, 50, 300.0);
        let img_w = 100;
        let img_h = 100;
        let bytes = vec![0, 128, 255, 255].repeat((img_w * img_h) as usize);

        let l_idx = paste_rgba_into_document(&mut doc, img_w, img_h, &bytes);
        let layer = &doc.layers[l_idx];

        assert_eq!(layer.get_pixel(0, 0), Some([0, 128, 255, 255]));
        assert_eq!(layer.get_pixel(25, 25), Some([0, 128, 255, 255]));
        assert_eq!(layer.get_pixel(49, 49), Some([0, 128, 255, 255]));
    }
}

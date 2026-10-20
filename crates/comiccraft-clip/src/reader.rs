use crate::chunks::{parse_container, parse_exta};
use crate::error::ClipError;
use crate::sqlite_db::read_sqlite_database;
use crate::tiles::decode_tiles;
use comiccraft_core::document::Document;
use comiccraft_core::layer::Layer;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ClipMetadata {
    pub file_size: u64,
    pub width: u32,
    pub height: u32,
    pub dpi: f32,
    pub layer_count: usize,
    pub layers: Vec<LayerMeta>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LayerMeta {
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend_mode: String,
    pub clipped: bool,
    pub alpha_locked: bool,
    pub color_tag: String,
}

/// Opens and parses an authentic Clip Studio Paint (.clip) file into a ComicCraft Document.
pub fn open_clip_file(path: &Path) -> Result<Document, ClipError> {
    let file_bytes = fs::read(path)?;
    let chunks = parse_container(&file_bytes)?;

    let mut exta_map: HashMap<Vec<u8>, &[u8]> = HashMap::new();
    let mut sqli_payload: Option<&[u8]> = None;

    for chunk in &chunks {
        if &chunk.name == b"Exta" {
            if let Ok((id, payload)) = parse_exta(chunk.data) {
                exta_map.insert(id, payload);
            }
        } else if &chunk.name == b"SQLi" {
            sqli_payload = Some(chunk.data);
        }
    }

    let sqli_data = sqli_payload.ok_or_else(|| {
        ClipError::Malformed("missing SQLi database chunk in .clip container".into())
    })?;

    let meta = read_sqlite_database(sqli_data)?;

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled.clip")
        .to_string();

    let mut doc = Document::new(file_name, meta.width, meta.height, meta.dpi);
    doc.layers.clear();

    let cw = meta.width;
    let ch = meta.height;
    let canvas_len = (cw as usize) * (ch as usize) * 4;
    let mut selected_idx: Option<usize> = None;

    for (idx, layer_meta) in meta.layers.into_iter().enumerate() {
        if layer_meta.selected {
            selected_idx = Some(idx);
        }

        let pixels = if let Some(payload) = exta_map.get(&layer_meta.external_id) {
            let decoded = decode_tiles(layer_meta.width, layer_meta.height, payload)?;
            if layer_meta.width == cw
                && layer_meta.height == ch
                && layer_meta.offset_x == 0
                && layer_meta.offset_y == 0
            {
                decoded
            } else {
                let mut canvas_buf = vec![0u8; canvas_len];
                if let Some(fill) = layer_meta.solid_fill_color {
                    for px in canvas_buf.chunks_exact_mut(4) {
                        px.copy_from_slice(&fill);
                    }
                }
                let lw = layer_meta.width as i32;
                let lh = layer_meta.height as i32;
                for ly in 0..lh {
                    let dy = ly + layer_meta.offset_y;
                    if dy < 0 || dy >= ch as i32 {
                        continue;
                    }
                    for lx in 0..lw {
                        let dx = lx + layer_meta.offset_x;
                        if dx < 0 || dx >= cw as i32 {
                            continue;
                        }
                        let s_i = ((ly * lw + lx) as usize) * 4;
                        let d_i = ((dy * (cw as i32) + dx) as usize) * 4;
                        if s_i + 4 <= decoded.len() && d_i + 4 <= canvas_buf.len() {
                            canvas_buf[d_i..d_i + 4].copy_from_slice(&decoded[s_i..s_i + 4]);
                        }
                    }
                }
                canvas_buf
            }
        } else if let Some(fill) = layer_meta.solid_fill_color {
            if idx == 0 {
                doc.background_color = fill;
            }
            let mut canvas_buf = vec![0u8; canvas_len];
            for px in canvas_buf.chunks_exact_mut(4) {
                px.copy_from_slice(&fill);
            }
            canvas_buf
        } else {
            vec![0u8; canvas_len]
        };

        let mut layer = Layer::new(layer_meta.main_id as u64, layer_meta.name, 0, 0);
        layer.visible = layer_meta.visible;
        layer.opacity = layer_meta.opacity;
        layer.blend_mode = layer_meta.blend_mode;
        layer.clipped_to_below = layer_meta.clipped;
        layer.alpha_locked = layer_meta.alpha_locked;
        layer.color_tag = layer_meta.color_tag;
        layer.width = cw;
        layer.height = ch;
        layer.pixels = pixels;
        doc.layers.push(layer);
    }

    if doc.layers.is_empty() {
        doc.layers.push(Layer::new(1, "Layer 1", doc.width, doc.height));
    }

    doc.active_layer_idx = selected_idx
        .unwrap_or_else(|| if doc.layers.len() > 1 { 1 } else { 0 })
        .min(doc.layers.len() - 1);
    doc.update_composite();

    Ok(doc)
}


/// Inspects metadata of a .clip file without fully decompressing all layer tiles.
pub fn inspect_clip_file(path: &Path) -> Result<ClipMetadata, ClipError> {
    let file_size = fs::metadata(path)?.len();
    let file_bytes = fs::read(path)?;
    let chunks = parse_container(&file_bytes)?;

    let sqli_payload = chunks
        .iter()
        .find(|c| &c.name == b"SQLi")
        .map(|c| c.data)
        .ok_or_else(|| ClipError::Malformed("missing SQLi chunk".into()))?;

    let meta = read_sqlite_database(sqli_payload)?;

    let layers = meta
        .layers
        .iter()
        .map(|l| LayerMeta {
            id: l.main_id as u64,
            name: l.name.clone(),
            visible: l.visible,
            opacity: l.opacity,
            blend_mode: l.blend_mode.name().to_string(),
            clipped: l.clipped,
            alpha_locked: l.alpha_locked,
            color_tag: l.color_tag.name().to_string(),
        })
        .collect();

    Ok(ClipMetadata {
        file_size,
        width: meta.width,
        height: meta.height,
        dpi: meta.dpi,
        layer_count: meta.layers.len(),
        layers,
    })
}

use crate::chunks::{build_exta, ContainerWriter};
use crate::error::ClipError;
use crate::sqlite_db::create_sqlite_database;
use crate::tiles::encode_tiles;
use comiccraft_core::document::Document;
use std::fs::File;
use std::io::{BufWriter, Cursor, Write};
use std::path::Path;

fn generate_preview_png(doc: &Document) -> Vec<u8> {
    let pw = 128u32;
    let ph = 128u32;
    let mut img = image::RgbaImage::new(pw, ph);
    if let Some(ref comp) = doc.composite_cache {
        if !comp.is_empty() && doc.width > 0 && doc.height > 0 {
            for y in 0..ph {
                let src_y = ((y as u64 * doc.height as u64) / ph as u64) as u32;
                for x in 0..pw {
                    let src_x = ((x as u64 * doc.width as u64) / pw as u64) as u32;
                    let src_idx = ((src_y * doc.width + src_x) * 4) as usize;
                    if src_idx + 4 <= comp.len() {
                        let pixel = image::Rgba([
                            comp[src_idx],
                            comp[src_idx + 1],
                            comp[src_idx + 2],
                            comp[src_idx + 3],
                        ]);
                        img.put_pixel(x, y, pixel);
                    }
                }
            }
        }
    }
    let mut buf = Cursor::new(Vec::new());
    let _ = img.write_to(&mut buf, image::ImageFormat::Png);
    buf.into_inner()
}

/// Saves a ComicCraft document as an authentic Clip Studio Paint (.clip) file.
pub fn save_clip_file(doc: &Document, path: &Path) -> Result<(), ClipError> {
    let mut writer = ContainerWriter::new();

    // 1. CHNKHead chunk (40 bytes payload)
    // Structure:
    // u64 version (256)
    // u64 sqlite_chunk_pos (patched later)
    // 16 bytes GUID
    // 8 bytes padding
    let mut head_payload = Vec::with_capacity(40);
    head_payload.extend_from_slice(&256u64.to_be_bytes());
    head_payload.extend_from_slice(&0u64.to_be_bytes()); // placeholder
    head_payload.extend_from_slice(&[
        0x2f, 0x7b, 0xf5, 0x1a, 0x5b, 0xc4, 0x7f, 0xb0,
        0x10, 0x84, 0x7e, 0x0b, 0x5e, 0x5a, 0x22, 0xb8,
    ]);
    head_payload.extend_from_slice(&0u64.to_be_bytes()); // padding

    let head_pos = writer.add_chunk(b"Head", &head_payload);

    // 2. CHNKExta chunks for each layer
    let mut layer_ext_ids = Vec::new();
    for (idx, layer) in doc.layers.iter().enumerate() {
        // Authentic 40-byte external ID: "extrnlid" + 32-char hex
        let id_str = format!("extrnlid{:032x}", (layer.id as u128) + ((idx as u128) + 1));
        let id_bytes = id_str.into_bytes();
        layer_ext_ids.push(id_bytes.clone());

        let tile_data = encode_tiles(layer.width, layer.height, &layer.pixels)?;
        let exta_payload = build_exta(&id_bytes, &tile_data);
        writer.add_chunk(b"Exta", &exta_payload);
    }

    // 3. Generate preview PNG
    let preview_png = generate_preview_png(doc);

    // 4. Generate SQLite DB
    let sqlite_bytes = create_sqlite_database(
        doc.width,
        doc.height,
        doc.dpi,
        &doc.layers,
        &layer_ext_ids,
        &preview_png,
    )?;

    // 5. CHNKSQLi chunk
    let sqlite_pos = writer.add_chunk(b"SQLi", &sqlite_bytes);

    // 6. CHNKFoot chunk
    writer.add_chunk(b"Foot", &[]);

    // 7. Back-patch sqlite_chunk_pos in CHNKHead payload
    // Head chunk layout: 16-byte chunk header (CHNK + name + size) + 8 (version) + 8 (sqlite_pos)
    let sqlite_patch_pos = head_pos + 16 + 8;
    writer.patch_u64(sqlite_patch_pos, sqlite_pos as u64);

    // 8. Finalize container and write to disk
    let file_data = writer.finish();
    let file = File::create(path)?;
    let mut buf_writer = BufWriter::new(file);
    buf_writer.write_all(&file_data)?;
    buf_writer.flush()?;

    Ok(())
}

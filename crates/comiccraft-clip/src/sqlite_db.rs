use crate::error::ClipError;
use comiccraft_core::blend::BlendMode;
use comiccraft_core::layer::{Layer, LayerColorTag};
use rusqlite::{params, Connection};
use std::collections::HashMap;

/// Builds the 160-byte Offscreen Attribute binary BLOB required by Clip Studio Paint.
pub fn build_offscreen_attribute(width: u32, height: u32) -> Vec<u8> {
    let grid_w = (width + 255) / 256;
    let grid_h = (height + 255) / 256;

    let mut out = Vec::with_capacity(160);
    // Header (16 bytes)
    out.extend_from_slice(&16u32.to_be_bytes());
    out.extend_from_slice(&102u32.to_be_bytes()); // info_section_size
    out.extend_from_slice(&42u32.to_be_bytes());  // extra_info_section_size
    out.extend_from_slice(&0u32.to_be_bytes());

    // "Parameter" in UTF-16BE: length 9 (4 bytes) + 18 bytes
    out.extend_from_slice(&9u32.to_be_bytes());
    for &ch in b"Parameter" {
        out.push(0);
        out.push(ch);
    }

    // Dimensions (16 bytes)
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&grid_w.to_be_bytes());
    out.extend_from_slice(&grid_h.to_be_bytes());

    // Attributes array (16 ints = 64 bytes)
    let mut attrs = [0u32; 16];
    attrs[1] = 1; // 1 alpha channel
    attrs[2] = 4; // 4 color channels (BGRX)
    attrs[8] = 8; // 8-bit depth
    for val in attrs {
        out.extend_from_slice(&val.to_be_bytes());
    }

    // "InitColor" in UTF-16BE: length 9 (4 bytes) + 18 bytes
    out.extend_from_slice(&9u32.to_be_bytes());
    for &ch in b"InitColor" {
        out.push(0);
        out.push(ch);
    }

    // 5 ints (20 bytes)
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes()); // default_fill_black_white
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());

    out
}

/// Parses width, height, grid_w, grid_h, default_fill_white from an Offscreen Attribute BLOB.
pub fn parse_offscreen_attribute(data: &[u8]) -> Option<(u32, u32, u32, u32, bool)> {
    if data.len() < 54 {
        return None;
    }
    // After 16 byte header + 4 byte str len + 18 byte "Parameter" = 38 bytes
    let width = u32::from_be_bytes([data[38], data[39], data[40], data[41]]);
    let height = u32::from_be_bytes([data[42], data[43], data[44], data[45]]);
    let grid_w = u32::from_be_bytes([data[46], data[47], data[48], data[49]]);
    let grid_h = u32::from_be_bytes([data[50], data[51], data[52], data[53]]);
    let default_fill_white = if data.len() >= 148 {
        u32::from_be_bytes([data[144], data[145], data[146], data[147]]) != 0
    } else {
        false
    };
    Some((width, height, grid_w, grid_h, default_fill_white))
}

pub fn blend_mode_to_csp(blend: BlendMode) -> i32 {
    match blend {
        BlendMode::Normal => 0,
        BlendMode::Darken => 1,
        BlendMode::Multiply => 2,
        BlendMode::ColorBurn => 3,
        BlendMode::Lighten => 7,
        BlendMode::Screen => 8,
        BlendMode::ColorDodge => 9,
        BlendMode::Overlay => 14,
        BlendMode::SoftLight => 15,
        BlendMode::HardLight => 16,
        BlendMode::Difference => 21,
        BlendMode::Exclusion => 22,
    }
}

pub fn csp_to_blend_mode(code: i32) -> BlendMode {
    match code {
        0 => BlendMode::Normal,
        1 => BlendMode::Darken,
        2 => BlendMode::Multiply,
        3 => BlendMode::ColorBurn,
        7 => BlendMode::Lighten,
        8 => BlendMode::Screen,
        9 | 10 | 11 | 12 => BlendMode::ColorDodge,
        14 => BlendMode::Overlay,
        15 => BlendMode::SoftLight,
        16 | 17 | 18 | 19 => BlendMode::HardLight,
        21 => BlendMode::Difference,
        22 => BlendMode::Exclusion,
        _ => BlendMode::Normal,
    }
}

pub struct LayerDbRecord {
    pub main_id: i64,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub clipped: bool,
    pub alpha_locked: bool,
    pub color_tag: LayerColorTag,
    pub external_id: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub solid_fill_color: Option<[u8; 4]>,
    pub selected: bool,
}

pub struct DocumentDbMeta {
    pub width: u32,
    pub height: u32,
    pub dpi: f32,
    pub layers: Vec<LayerDbRecord>,
}


/// Creates an authentic CSP SQLite database in memory and returns its serialized file bytes.
pub fn create_sqlite_database(
    width: u32,
    height: u32,
    dpi: f32,
    layers: &[Layer],
    layer_external_ids: &[Vec<u8>],
    preview_png: &[u8],
) -> Result<Vec<u8>, ClipError> {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "comiccraft_csp_{}_{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let conn = Connection::open(&temp_path)?;

    // Create authentic Clip Studio Paint tables
    conn.execute(
        "CREATE TABLE Canvas (
            CanvasId INTEGER PRIMARY KEY,
            CanvasWidth INTEGER,
            CanvasHeight INTEGER,
            CanvasResolution REAL,
            CanvasColorMode INTEGER,
            CanvasRootFolder INTEGER
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE Layer (
            MainId INTEGER PRIMARY KEY,
            CanvasId INTEGER,
            LayerName TEXT,
            LayerType INTEGER,
            LayerComposite INTEGER,
            LayerOpacity INTEGER,
            LayerClip INTEGER,
            LayerVisibility INTEGER,
            LayerLock INTEGER,
            LayerMasking INTEGER,
            LayerOffsetX INTEGER,
            LayerOffsetY INTEGER,
            LayerRenderOffscrOffsetX INTEGER,
            LayerRenderOffscrOffsetY INTEGER,
            LayerMaskOffsetX INTEGER,
            LayerMaskOffsetY INTEGER,
            LayerMaskOffscrOffsetX INTEGER,
            LayerMaskOffscrOffsetY INTEGER,
            LayerFolder INTEGER,
            LayerFirstChildIndex INTEGER,
            LayerNextIndex INTEGER,
            LayerSelect INTEGER,
            LayerRenderMipmap INTEGER,
            LayerLayerMaskMipmap INTEGER
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE Mipmap (
            MainId INTEGER PRIMARY KEY,
            BaseMipmapInfo INTEGER
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE MipmapInfo (
            MainId INTEGER PRIMARY KEY,
            Offscreen INTEGER,
            NextIndex INTEGER
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE Offscreen (
            MainId INTEGER PRIMARY KEY,
            LayerId INTEGER,
            BlockData BLOB,
            Attribute BLOB
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE CanvasPreview (
            MainId INTEGER PRIMARY KEY,
            ImageData BLOB
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE ExternalTableAndColumnName (
            TableName TEXT,
            ColumnName TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE Catalog (
            SoftwareName TEXT,
            Version TEXT,
            Creator TEXT
        )",
        [],
    )?;

    // Insert ExternalTableAndColumnName
    conn.execute(
        "INSERT INTO ExternalTableAndColumnName (TableName, ColumnName) VALUES ('Offscreen', 'BlockData')",
        [],
    )?;

    // Insert Catalog
    conn.execute(
        "INSERT INTO Catalog (SoftwareName, Version, Creator) VALUES ('Clip Studio Paint', '1.13.2', 'ArtCraft ComicCraft')",
        [],
    )?;

    // Insert Canvas
    let root_folder_id = 1i64;
    conn.execute(
        "INSERT INTO Canvas (CanvasId, CanvasWidth, CanvasHeight, CanvasResolution, CanvasColorMode, CanvasRootFolder)
         VALUES (1, ?1, ?2, ?3, 1, ?4)",
        params![width, height, dpi as f64, root_folder_id],
    )?;

    let first_child_id = if layers.is_empty() { 0 } else { 2i64 };

    // Insert Root Folder layer (MainId = 1)
    conn.execute(
        "INSERT INTO Layer (
            MainId, CanvasId, LayerName, LayerType, LayerComposite, LayerOpacity, LayerClip,
            LayerVisibility, LayerLock, LayerMasking, LayerOffsetX, LayerOffsetY,
            LayerRenderOffscrOffsetX, LayerRenderOffscrOffsetY, LayerMaskOffsetX, LayerMaskOffsetY,
            LayerMaskOffscrOffsetX, LayerMaskOffscrOffsetY, LayerFolder, LayerFirstChildIndex,
            LayerNextIndex, LayerSelect, LayerRenderMipmap, LayerLayerMaskMipmap
        ) VALUES (
            1, 1, 'Root', 0, 0, 256, 0,
            1, 0, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 1, ?1,
            0, 0, NULL, NULL
        )",
        params![first_child_id],
    )?;

    let attr_blob = build_offscreen_attribute(width, height);

    // Insert Each Layer in tree order
    for (idx, layer) in layers.iter().enumerate() {
        let layer_id = (idx + 2) as i64;
        let next_index = if idx + 1 < layers.len() { (idx + 3) as i64 } else { 0i64 };
        let mipmap_id = (idx + 10) as i64;
        let mipmap_info_id = (idx + 100) as i64;
        let offscreen_id = (idx + 1000) as i64;

        let composite = blend_mode_to_csp(layer.blend_mode);
        let opacity_int = (layer.opacity.clamp(0.0, 1.0) * 256.0).round() as i32;
        let visible_int = if layer.visible { 1 } else { 0 };
        let clipped_int = if layer.clipped_to_below { 1 } else { 0 };
        let lock_int = if layer.alpha_locked { 1 } else { 0 };
        let select_int = if idx == 0 { 1 } else { 0 };

        conn.execute(
            "INSERT INTO Layer (
                MainId, CanvasId, LayerName, LayerType, LayerComposite, LayerOpacity, LayerClip,
                LayerVisibility, LayerLock, LayerMasking, LayerOffsetX, LayerOffsetY,
                LayerRenderOffscrOffsetX, LayerRenderOffscrOffsetY, LayerMaskOffsetX, LayerMaskOffsetY,
                LayerMaskOffscrOffsetX, LayerMaskOffscrOffsetY, LayerFolder, LayerFirstChildIndex,
                LayerNextIndex, LayerSelect, LayerRenderMipmap, LayerLayerMaskMipmap
            ) VALUES (
                ?1, 1, ?2, 1, ?3, ?4, ?5,
                ?6, ?7, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                ?8, ?9, ?10, NULL
            )",
            params![
                layer_id,
                layer.name,
                composite,
                opacity_int,
                clipped_int,
                visible_int,
                lock_int,
                next_index,
                select_int,
                mipmap_id
            ],
        )?;

        // Mipmap
        conn.execute(
            "INSERT INTO Mipmap (MainId, BaseMipmapInfo) VALUES (?1, ?2)",
            params![mipmap_id, mipmap_info_id],
        )?;

        // MipmapInfo
        conn.execute(
            "INSERT INTO MipmapInfo (MainId, Offscreen, NextIndex) VALUES (?1, ?2, 0)",
            params![mipmap_info_id, offscreen_id],
        )?;

        // Offscreen with BlockData as raw bytes and Attribute
        let ext_id = &layer_external_ids[idx];
        conn.execute(
            "INSERT INTO Offscreen (MainId, LayerId, BlockData, Attribute) VALUES (?1, ?2, ?3, ?4)",
            params![offscreen_id, layer_id, ext_id, attr_blob],
        )?;
    }

    // Insert CanvasPreview
    conn.execute(
        "INSERT INTO CanvasPreview (MainId, ImageData) VALUES (1, ?1)",
        params![preview_png],
    )?;

    drop(conn);

    let bytes = std::fs::read(&temp_path)?;
    let _ = std::fs::remove_file(&temp_path);
    Ok(bytes)
}

/// Reads authentic Clip Studio Paint SQLite database bytes and returns canvas dimensions and layer metadata.
pub fn read_sqlite_database(db_bytes: &[u8]) -> Result<DocumentDbMeta, ClipError> {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "comiccraft_read_{}_{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    std::fs::write(&temp_path, db_bytes)?;
    let conn = Connection::open(&temp_path)?;

    // Query Canvas (real CSP stores CanvasWidth & CanvasHeight as REAL floats e.g. 3840.0)
    let (width, height, dpi, root_folder): (u32, u32, f32, Option<i64>) = conn.query_row(
        "SELECT CAST(CanvasWidth AS INTEGER), CAST(CanvasHeight AS INTEGER), CAST(COALESCE(CanvasResolution, 350.0) AS REAL), CanvasRootFolder FROM Canvas LIMIT 1",
        [],
        |row| {
            let w: i64 = row.get(0)?;
            let h: i64 = row.get(1)?;
            let d: f64 = row.get(2)?;
            let rf: Option<i64> = row.get(3).ok();
            Ok((w.max(1) as u32, h.max(1) as u32, d as f32, rf))
        },
    )?;

    // Check which optional columns exist in the Layer table
    let mut layer_cols = std::collections::HashSet::new();
    if let Ok(mut col_stmt) = conn.prepare("PRAGMA table_info(Layer)") {
        if let Ok(rows) = col_stmt.query_map([], |r| r.get::<_, String>(1)) {
            for col in rows.flatten() {
                layer_cols.insert(col);
            }
        }
    }
    let has_draw_color = layer_cols.contains("DrawColorEnable")
        && layer_cols.contains("DrawColorMainRed")
        && layer_cols.contains("DrawColorMainGreen")
        && layer_cols.contains("DrawColorMainBlue");

    // Map MipmapInfo -> Offscreen -> BlockData & Attribute
    struct OffscreenRow {
        block_data: Vec<u8>,
        attribute: Vec<u8>,
    }
    let mut offscreen_map: HashMap<i64, OffscreenRow> = HashMap::new();
    let mut off_stmt = conn.prepare("SELECT MainId, BlockData, Attribute FROM Offscreen")?;
    let off_rows = off_stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let block_data: Vec<u8> = row.get(1)?;
        let attribute: Vec<u8> = row.get(2)?;
        Ok((id, OffscreenRow { block_data, attribute }))
    })?;
    for item in off_rows {
        let (id, row) = item?;
        offscreen_map.insert(id, row);
    }
    drop(off_stmt);

    let mut mipmap_info_map: HashMap<i64, i64> = HashMap::new();
    let mut mi_stmt = conn.prepare("SELECT MainId, Offscreen FROM MipmapInfo")?;
    let mi_rows = mi_stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let off: i64 = row.get(1)?;
        Ok((id, off))
    })?;
    for item in mi_rows {
        let (id, off) = item?;
        mipmap_info_map.insert(id, off);
    }
    drop(mi_stmt);

    let mut mipmap_map: HashMap<i64, i64> = HashMap::new();
    let mut mm_stmt = conn.prepare("SELECT MainId, BaseMipmapInfo FROM Mipmap")?;
    let mm_rows = mm_stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let bmi: i64 = row.get(1)?;
        Ok((id, bmi))
    })?;
    for item in mm_rows {
        let (id, bmi) = item?;
        mipmap_map.insert(id, bmi);
    }
    drop(mm_stmt);

    // Query Layers
    struct RawLayerRow {
        main_id: i64,
        name: String,
        layer_type: i32,
        composite: i32,
        opacity: i32,
        clip: i32,
        visibility: i32,
        lock: i32,
        folder: i32,
        first_child: i64,
        next_index: i64,
        render_mipmap: Option<i64>,
        offset_x: i32,
        offset_y: i32,
        draw_color: Option<[u8; 4]>,
        selected: bool,
    }

    let draw_color_sql = if has_draw_color {
        "COALESCE(DrawColorEnable, 0), COALESCE(DrawColorMainRed, 0), COALESCE(DrawColorMainGreen, 0), COALESCE(DrawColorMainBlue, 0)"
    } else {
        "0, 0, 0, 0"
    };
    let layer_query = format!(
        "SELECT MainId, LayerName, COALESCE(LayerType, 1), COALESCE(LayerComposite, 0),
                COALESCE(LayerOpacity, 256), COALESCE(LayerClip, 0), COALESCE(LayerVisibility, 1),
                COALESCE(LayerLock, 0), COALESCE(LayerFolder, 0), COALESCE(LayerFirstChildIndex, 0),
                COALESCE(LayerNextIndex, 0), LayerRenderMipmap,
                COALESCE(LayerOffsetX, 0) + COALESCE(LayerRenderOffscrOffsetX, 0),
                COALESCE(LayerOffsetY, 0) + COALESCE(LayerRenderOffscrOffsetY, 0),
                COALESCE(LayerSelect, 0),
                {draw_color_sql}
         FROM Layer"
    );

    let mut layer_map: HashMap<i64, RawLayerRow> = HashMap::new();
    let mut l_stmt = conn.prepare(&layer_query)?;
    let l_rows = l_stmt.query_map([], |row| {
        let main_id: i64 = row.get(0)?;
        let name: String = row.get(1).unwrap_or_else(|_| "Layer".into());
        let layer_type: i32 = row.get(2)?;
        let composite: i32 = row.get(3)?;
        let opacity: i32 = row.get(4)?;
        let clip: i32 = row.get(5)?;
        let visibility: i32 = row.get(6)?;
        let lock: i32 = row.get(7)?;
        let folder: i32 = row.get(8)?;
        let first_child: i64 = row.get(9)?;
        let next_index: i64 = row.get(10)?;
        let render_mipmap: Option<i64> = row.get(11).ok();
        let offset_x: i32 = row.get(12).unwrap_or(0);
        let offset_y: i32 = row.get(13).unwrap_or(0);
        let select_flags: i32 = row.get(14).unwrap_or(0);
        let dc_en: i64 = row.get(15).unwrap_or(0);
        let dc_r: i64 = row.get(16).unwrap_or(0);
        let dc_g: i64 = row.get(17).unwrap_or(0);
        let dc_b: i64 = row.get(18).unwrap_or(0);

        let draw_color = if dc_en != 0 {
            let r = ((dc_r.max(0) as u64) >> 24).min(255) as u8;
            let g = ((dc_g.max(0) as u64) >> 24).min(255) as u8;
            let b = ((dc_b.max(0) as u64) >> 24).min(255) as u8;
            Some([r, g, b, 255])
        } else {
            None
        };

        Ok(RawLayerRow {
            main_id,
            name,
            layer_type,
            composite,
            opacity,
            clip,
            visibility,
            lock,
            folder,
            first_child,
            next_index,
            render_mipmap,
            offset_x,
            offset_y,
            draw_color,
            selected: (select_flags & 1) != 0,
        })
    })?;

    for item in l_rows {
        let row = item?;
        layer_map.insert(row.main_id, row);
    }
    drop(l_stmt);
    drop(conn);
    let _ = std::fs::remove_file(&temp_path);

    // Collect layers in tree order
    let mut ordered_layer_ids = Vec::new();
    fn traverse_tree(
        current_id: i64,
        layer_map: &HashMap<i64, RawLayerRow>,
        out: &mut Vec<i64>,
    ) {
        let mut curr = current_id;
        while curr > 0 {
            if let Some(l) = layer_map.get(&curr) {
                if l.folder != 0 {
                    if l.first_child > 0 {
                        traverse_tree(l.first_child, layer_map, out);
                    }
                } else {
                    out.push(curr);
                }
                curr = l.next_index;
            } else {
                break;
            }
        }
    }

    if let Some(rf) = root_folder {
        if let Some(root_row) = layer_map.get(&rf) {
            traverse_tree(root_row.first_child, &layer_map, &mut ordered_layer_ids);
        }
    }

    // If tree traversal was empty, fall back to all non-folder layers sorted by MainId
    if ordered_layer_ids.is_empty() {
        let mut all_ids: Vec<_> = layer_map
            .iter()
            .filter(|(_, r)| r.folder == 0 && (r.layer_type == 1 || r.render_mipmap.is_some()))
            .map(|(&id, _)| id)
            .collect();
        all_ids.sort();
        ordered_layer_ids = all_ids;
    }

    let mut layers = Vec::new();
    for id in ordered_layer_ids {
        if let Some(raw) = layer_map.get(&id) {
            let mut ext_id = Vec::new();
            let mut layer_w = width;
            let mut layer_h = height;
            let mut solid_fill_color = raw.draw_color;

            if let Some(mm_id) = raw.render_mipmap {
                if let Some(&mi_id) = mipmap_map.get(&mm_id) {
                    if let Some(&off_id) = mipmap_info_map.get(&mi_id) {
                        if let Some(off) = offscreen_map.get(&off_id) {
                            ext_id = off.block_data.clone();
                            if let Some((w, h, _, _, def_white)) =
                                parse_offscreen_attribute(&off.attribute)
                            {
                                layer_w = w;
                                layer_h = h;
                                if solid_fill_color.is_none() && def_white {
                                    solid_fill_color = Some([255, 255, 255, 255]);
                                }
                            }
                        }
                    }
                }
            }

            layers.push(LayerDbRecord {
                main_id: raw.main_id,
                name: raw.name.clone(),
                visible: (raw.visibility & 1) != 0,
                opacity: (raw.opacity as f32) / 256.0,
                blend_mode: csp_to_blend_mode(raw.composite),
                clipped: raw.clip != 0,
                alpha_locked: raw.lock != 0,
                color_tag: LayerColorTag::None,
                external_id: ext_id,
                width: layer_w,
                height: layer_h,
                offset_x: raw.offset_x,
                offset_y: raw.offset_y,
                solid_fill_color,
                selected: raw.selected,
            });
        }
    }

    Ok(DocumentDbMeta {
        width,
        height,
        dpi,
        layers,
    })
}


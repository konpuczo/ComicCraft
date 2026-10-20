use crate::error::ClipError;
use byteorder::{BigEndian, ReadBytesExt};
use comiccraft_core::{
    BrushPreset, CustomSubTool, GrayMap, InkMode, TipTexture, ToolKind,
};
use rusqlite::{Connection, OpenFlags};
use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;

/// Imports a custom sub-tool from a Clip Studio Paint `.sut` file, a custom brush tip `.png`/`.jpg`
/// image, or a `.json` preset file.
pub fn import_custom_subtool_file(
    path: &Path,
    fallback_tool: ToolKind,
) -> Result<CustomSubTool, ClipError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "sut" => import_sut_file(path),
        "json" | "ccbrush" => {
            let bytes = std::fs::read(path)?;
            let mut custom: CustomSubTool = serde_json::from_slice(&bytes)
                .map_err(|e| ClipError::Malformed(format!("Invalid preset JSON: {e}")))?;
            custom.source_path = Some(path.display().to_string());
            Ok(custom)
        }
        "png" | "jpg" | "jpeg" | "webp" | "bmp" => import_image_tip_subtool(path, fallback_tool),
        _ => {
            // Try .sut SQLite detection first, then image fallback
            let bytes = std::fs::read(path)?;
            if bytes.starts_with(b"SQLite format 3\0") {
                import_sut_file(path)
            } else {
                import_image_tip_subtool(path, fallback_tool)
            }
        }
    }
}

/// Imports a Clip Studio Paint `.sut` (Sub Tool) SQLite database file, extracting its `Node`
/// metadata, `Variant` brush physics, `Effector` pressure curves, and embedded `MaterialFile`
/// TAR archives (`BrushPattern` tip stamp and `PaperTexture` grain bitmap).
pub fn import_sut_file(path: &Path) -> Result<CustomSubTool, ClipError> {
    let raw = std::fs::read(path)?;
    let sqlite_offset = find_subslice(&raw, b"SQLite format 3\0")
        .ok_or_else(|| ClipError::Malformed("No SQLite3 database found in .sut file".into()))?;

    let mut tmp_guard = None;
    let db_path = if sqlite_offset == 0 {
        path.to_path_buf()
    } else {
        let p = std::env::temp_dir().join(format!("comiccraft_sut_{}.db", std::process::id()));
        std::fs::write(&p, &raw[sqlite_offset..])?;
        tmp_guard = Some(p.clone());
        p
    };

    let res = parse_sut_sqlite(&db_path, path);
    if let Some(p) = tmp_guard {
        let _ = std::fs::remove_file(p);
    }
    res
}

fn parse_sut_sqlite(db_path: &Path, orig_path: &Path) -> Result<CustomSubTool, ClipError> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let fallback_name = orig_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Custom Pencil")
        .to_string();

    // 1. Read Node table
    let node_cols = table_columns(&conn, "Node");
    let mut name = fallback_name.clone();
    let mut node_icon: i64 = 102;
    let mut variant_id: i64 = 0;

    if !node_cols.is_empty() {
        let name_expr = if node_cols.contains("NodeName") {
            "COALESCE(NodeName, '')"
        } else {
            "''"
        };
        let icon_expr = if node_cols.contains("NodeIcon") {
            "COALESCE(NodeIcon, 102)"
        } else {
            "102"
        };
        let var_expr = if node_cols.contains("NodeVariantID") {
            "COALESCE(NodeVariantID, 0)"
        } else {
            "0"
        };
        let sql = format!("SELECT {name_expr}, {icon_expr}, {var_expr} FROM Node LIMIT 1");
        if let Ok((n, ic, vid)) = conn.query_row(&sql, [], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, i64>(1).unwrap_or(102),
                row.get::<_, i64>(2).unwrap_or(0),
            ))
        }) {
            if !n.trim().is_empty() {
                name = n.trim().to_string();
            }
            node_icon = ic;
            variant_id = vid;
        }
    }

    let tool_kind = classify_tool_kind(&name, node_icon);

    // 2. Read Variant table
    let var_cols = table_columns(&conn, "Variant");
    let mut preset = match tool_kind {
        ToolKind::Pencil => BrushPreset::for_tool(ToolKind::Pencil, comiccraft_core::SubTool::RealPencil),
        ToolKind::Brush => BrushPreset::for_tool(ToolKind::Brush, comiccraft_core::SubTool::DenseWatercolor),
        ToolKind::Airbrush => BrushPreset::for_tool(ToolKind::Airbrush, comiccraft_core::SubTool::AirbrushSoft),
        ToolKind::Eraser => BrushPreset::for_tool(ToolKind::Eraser, comiccraft_core::SubTool::HardEraser),
        _ => BrushPreset::for_tool(ToolKind::Pen, comiccraft_core::SubTool::GPen),
    };
    let mut antialiasing: u8 = 2;
    let mut invert_texture = false;

    if !var_cols.is_empty() {
        let where_clause = if variant_id > 0 && var_cols.contains("VariantID") {
            format!("WHERE VariantID = {variant_id}")
        } else {
            String::new()
        };

        let get_f64 = |col: &str, default: f64| -> f64 {
            if !var_cols.contains(col) {
                return default;
            }
            let q = format!("SELECT CAST(COALESCE({col}, {default}) AS REAL) FROM Variant {where_clause} LIMIT 1");
            conn.query_row(&q, [], |r| r.get::<_, f64>(0)).unwrap_or(default)
        };
        let get_i64 = |col: &str, default: i64| -> i64 {
            if !var_cols.contains(col) {
                return default;
            }
            let q = format!("SELECT CAST(COALESCE({col}, {default}) AS INTEGER) FROM Variant {where_clause} LIMIT 1");
            conn.query_row(&q, [], |r| r.get::<_, i64>(0)).unwrap_or(default)
        };
        let get_blob = |col: &str| -> Option<Vec<u8>> {
            if !var_cols.contains(col) {
                return None;
            }
            let q = format!("SELECT {col} FROM Variant {where_clause} LIMIT 1");
            conn.query_row(&q, [], |r| r.get::<_, Option<Vec<u8>>>(0))
                .ok()
                .flatten()
        };

        let opacity_pct = get_f64("Opacity", 100.0);
        preset.default_opacity = (opacity_pct as f32 / 100.0).clamp(0.05, 1.0);

        let aa = get_i64("AntiAlias", 2);
        antialiasing = (aa.clamp(0, 3)) as u8;

        let raw_size = get_f64("BrushSize", preset.default_size as f64) as f32;
        preset.default_size = if raw_size > 120.0 && matches!(tool_kind, ToolKind::Pencil | ToolKind::Pen) {
            // Many CSP .sut brushes store last-tested giant canvas sizes (e.g. 1000.0); normalize to comfortable drawing size
            (raw_size / 55.0).clamp(8.0, 28.0)
        } else {
            raw_size.clamp(1.0, 200.0)
        };

        let hardness_pct = get_f64("BrushHardness", (preset.default_hardness * 100.0) as f64);
        preset.default_hardness = (hardness_pct as f32 / 100.0).clamp(0.1, 1.0);

        let interval = get_f64("BrushInterval", (preset.spacing * 100.0) as f64);
        preset.spacing = (interval as f32 / 100.0).clamp(0.02, 0.50);

        let thickness = get_f64("BrushThickness", 100.0);
        preset.aspect = (thickness as f32 / 100.0).clamp(0.08, 1.0);

        let rotation = get_f64("BrushRotation", 0.0);
        preset.angle_deg = (rotation as f32).rem_euclid(360.0);

        let rot_rand = get_f64("BrushRotationRandomScale", 0.0);
        let rot_eff = get_i64("BrushRotationEffector", 0);
        if rot_eff > 0 && rot_rand > 0.0 {
            preset.random_angle = (rot_rand as f32 / 100.0).clamp(0.0, 1.0);
        }

        let revision = get_i64("BrushRevision", preset.default_stabilization as i64);
        preset.default_stabilization = (revision.clamp(0, 50)) as u32;

        let tex_density = get_f64("TextureDensity", 0.0) as f32;
        let tex_scale2 = get_f64("TextureScale2", 15.0) as f32;
        invert_texture = get_i64("TextureReverseDensity", 0) != 0;

        if tex_density > 0.0 {
            let scale = (tex_scale2 / 7.0).clamp(0.6, 4.0);
            let strength = (tex_density / 100.0 * 0.72).clamp(0.15, 0.85);
            preset.texture = TipTexture::Grain { scale, strength };
        }

        let use_water_edge = get_i64("BrushUseWaterEdge", 0);
        if use_water_edge != 0 {
            let power = get_f64("BrushWaterEdgeAlphaPower", 5.0) as f32;
            preset.water_edge = (power / 12.0).clamp(0.20, 0.85);
        }

        let use_spray = get_i64("BrushUseSpray", 0);
        if use_spray != 0 {
            let dens = get_i64("BrushSprayDensity", 12);
            preset.droplets = (dens.clamp(4, 36)) as u32;
        }

        let use_in = get_i64("BrushUseIn", 0);
        if use_in != 0 {
            let len = get_f64("BrushInLength", 10.0) as f32;
            preset.taper_in = (len / 10.0).clamp(0.5, 4.0);
        }
        let use_out = get_i64("BrushUseOut", 0);
        if use_out != 0 {
            let len = get_f64("BrushOutLength", 10.0) as f32;
            preset.taper_out = (len / 10.0).clamp(0.5, 4.5);
        }

        let flow_pct = get_f64("BrushFlow", 100.0) as f32;
        let use_watercolor = get_i64("BrushUseWaterColor", 0) != 0
            || get_i64("BrushUseWaterColor2", 0) != 0;

        if tool_kind == ToolKind::Eraser {
            preset.mode = InkMode::Erase;
        } else if use_watercolor {
            let mix = (get_f64("BrushMixColor", 50.0) as f32 / 100.0).clamp(0.1, 0.9);
            let pickup = (1.0 - mix).clamp(0.15, 0.75);
            preset.mode = InkMode::Mix {
                flow: (flow_pct / 100.0).clamp(0.15, 0.95),
                pickup,
            };
        } else if tool_kind == ToolKind::Pencil || tex_density > 0.0 || flow_pct < 98.0 {
            // Pencils and textured brushes build up organically per stroke/pass
            let base_flow = (flow_pct / 100.0 * 0.58).clamp(0.22, 0.75);
            preset.mode = InkMode::Buildup { flow: base_flow };
        } else {
            preset.mode = InkMode::Stroke;
        }

        // Parse Effector headers (48-byte big-endian header) for pressure sensitivity
        if let Some(sz_eff) = get_blob("BrushSizeEffector") {
            if let Some(min_ratio) = parse_effector_min_ratio(&sz_eff) {
                preset.size_min = min_ratio.clamp(0.10, 0.95);
            }
        }
        if let Some(op_eff) = get_blob("BrushOpacityEffector") {
            if let Some(min_ratio) = parse_effector_min_ratio(&op_eff) {
                preset.opacity_min = min_ratio.clamp(0.10, 0.95);
            }
        }
    }

    // 3. Extract MaterialFile TAR archives (PaperTexture & BrushPattern PNGs)
    let mut tip_stamp: Option<GrayMap> = None;
    let mut paper_texture: Option<GrayMap> = None;

    let mat_cols = table_columns(&conn, "MaterialFile");
    if mat_cols.contains("FileData") {
        let mut stmt = conn.prepare("SELECT FileData FROM MaterialFile")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            if let Ok(Some(blob)) = row.get::<_, Option<Vec<u8>>>(0) {
                let (xml_opt, png_opt) = extract_tar_xml_and_png(&blob);
                let Some(png_bytes) = png_opt else {
                    continue;
                };
                let Ok(img) = image::load_from_memory(&png_bytes) else {
                    continue;
                };
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width(), rgba.height());
                let xml = xml_opt.unwrap_or_default();

                if xml.contains("PaperTexture") {
                    if paper_texture.is_none() {
                        paper_texture = GrayMap::from_rgba_paper(w, h, rgba.as_raw(), invert_texture);
                    }
                } else if xml.contains("BrushPattern") {
                    if tip_stamp.is_none() {
                        tip_stamp = GrayMap::from_rgba_tip(w, h, rgba.as_raw());
                    }
                } else if tip_stamp.is_none() {
                    tip_stamp = GrayMap::from_rgba_tip(w, h, rgba.as_raw());
                } else if paper_texture.is_none() {
                    paper_texture = GrayMap::from_rgba_paper(w, h, rgba.as_raw(), invert_texture);
                }
            }
        }
    }

    Ok(CustomSubTool {
        id: 0,
        name,
        tool_kind,
        preset,
        antialiasing,
        tip_stamp,
        paper_texture,
        source_path: Some(orig_path.display().to_string()),
    })
}

/// Imports a standard image (`.png`, `.jpg`, `.webp`, `.bmp`) as a custom brush tip sub-tool.
fn import_image_tip_subtool(
    path: &Path,
    fallback_tool: ToolKind,
) -> Result<CustomSubTool, ClipError> {
    let img = image::open(path)
        .map_err(|e| ClipError::Malformed(format!("Failed to decode brush tip image: {e}")))?;
    let rgba = img.to_rgba8();
    let tip_stamp = GrayMap::from_rgba_tip(rgba.width(), rgba.height(), rgba.as_raw())
        .ok_or_else(|| ClipError::Malformed("Image did not contain a valid brush tip mask".into()))?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Custom Tip Pencil")
        .to_string();

    let tool_kind = match fallback_tool {
        ToolKind::Pen | ToolKind::Pencil | ToolKind::Brush | ToolKind::Airbrush | ToolKind::Eraser => {
            fallback_tool
        }
        _ => ToolKind::Pencil,
    };

    let mut preset = BrushPreset::for_tool(ToolKind::Pencil, comiccraft_core::SubTool::RealPencil);
    preset.random_angle = 0.55;
    preset.default_size = 16.0;

    Ok(CustomSubTool {
        id: 0,
        name,
        tool_kind,
        preset,
        antialiasing: 2,
        tip_stamp: Some(tip_stamp),
        paper_texture: None,
        source_path: Some(path.display().to_string()),
    })
}

fn classify_tool_kind(name: &str, node_icon: i64) -> ToolKind {
    let lower = name.to_ascii_lowercase();
    if lower.contains("pencil")
        || lower.contains("pastel")
        || lower.contains("charcoal")
        || lower.contains("chalk")
        || lower.contains("crayon")
        || lower.contains("sketch")
        || node_icon == 102
        || node_icon == 103
    {
        ToolKind::Pencil
    } else if lower.contains("eras") || node_icon == 110 {
        ToolKind::Eraser
    } else if lower.contains("air") || lower.contains("spray") || node_icon == 107 {
        ToolKind::Airbrush
    } else if lower.contains("water")
        || lower.contains("paint")
        || lower.contains("oil")
        || lower.contains("gouache")
        || lower.contains("brush")
        || (104..=106).contains(&node_icon)
    {
        ToolKind::Brush
    } else {
        ToolKind::Pen
    }
}

fn table_columns(conn: &Connection, table: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    let sql = format!("PRAGMA table_info({table})");
    if let Ok(mut stmt) = conn.prepare(&sql) {
        if let Ok(mut rows) = stmt.query([]) {
            while let Ok(Some(row)) = rows.next() {
                if let Ok(col_name) = row.get::<_, String>(1) {
                    set.insert(col_name);
                }
            }
        }
    }
    set
}

fn parse_effector_min_ratio(blob: &[u8]) -> Option<f32> {
    if blob.len() < 32 {
        return None;
    }
    let mut rdr = Cursor::new(blob);
    let hdr_len = rdr.read_u32::<BigEndian>().ok()?;
    if hdr_len != 48 {
        return None;
    }
    let flags = rdr.read_u32::<BigEndian>().ok()?;
    let p0 = rdr.read_u32::<BigEndian>().ok()?;
    let p1 = rdr.read_u32::<BigEndian>().ok()?;
    let p2 = rdr.read_u32::<BigEndian>().ok()?;
    let p3 = rdr.read_u32::<BigEndian>().ok()?;
    if flags == 0 {
        return None;
    }
    for v in [p1, p3, p2, p0] {
        if (1..=95).contains(&v) {
            return Some(v as f32 / 100.0);
        }
    }
    None
}

/// Extracts `icedata/layerData.xml` and `thumbnail/thumbnail.png` from a USTAR TAR archive blob.
fn extract_tar_xml_and_png(blob: &[u8]) -> (Option<String>, Option<Vec<u8>>) {
    let mut xml_out: Option<String> = None;
    let mut png_out: Option<Vec<u8>> = None;

    let mut pos = 0usize;
    while pos + 512 <= blob.len() {
        let hdr = &blob[pos..pos + 512];
        let name_end = hdr[..100].iter().position(|&b| b == 0).unwrap_or(100);
        if name_end == 0 {
            break;
        }
        let name = std::str::from_utf8(&hdr[..name_end]).unwrap_or("");
        let size_str = std::str::from_utf8(&hdr[124..136])
            .unwrap_or("")
            .trim_matches(|c: char| c == '\0' || c.is_ascii_whitespace());
        let Ok(size) = usize::from_str_radix(size_str, 8) else {
            break;
        };

        let data_start = pos + 512;
        let data_end = data_start.saturating_add(size);
        if data_end > blob.len() {
            break;
        }
        let payload = &blob[data_start..data_end];
        if name.ends_with(".xml") {
            xml_out = Some(String::from_utf8_lossy(payload).into_owned());
        } else if name.ends_with(".png") {
            png_out = Some(payload.to_vec());
        }

        let padded = (size + 511) & !511;
        pos = data_start.saturating_add(padded);
    }

    // Fallback: scan raw blob for PNG magic if TAR walk didn't find one
    if png_out.is_none() {
        let png_sig = b"\x89PNG\r\n\x1a\n";
        let iend_sig = b"IEND\xaeB`\x82";
        if let Some(start) = find_subslice(blob, png_sig) {
            if let Some(iend_rel) = find_subslice(&blob[start..], iend_sig) {
                let end = start + iend_rel + iend_sig.len();
                png_out = Some(blob[start..end].to_vec());
            }
        }
    }

    (xml_out, png_out)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

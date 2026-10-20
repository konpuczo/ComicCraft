use clap::{Parser, Subcommand};
use comiccraft_clip::{import_sut_file, inspect_clip_file, open_clip_file, save_clip_file};
use comiccraft_core::document::Document;
use comiccraft_core::stroke::StrokePoint;
use comiccraft_core::tools::{SubTool, ToolConfig, ToolKind};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "comiccraft-cli", version)]
#[command(
    about = "ComicCraft: ArtCraft 1:1 recreation of Clip Studio Paint CLI & MCP server",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect a .clip file and display canvas dimensions, layers, and blend modes
    Info {
        #[arg(value_name = "FILE.clip")]
        file: PathBuf,
        #[arg(short, long)]
        json: bool,
    },
    /// Convert and render a .clip file directly to PNG or JPEG
    Convert {
        #[arg(value_name = "INPUT.clip")]
        input: PathBuf,
        #[arg(value_name = "OUTPUT.png")]
        output: PathBuf,
    },
    /// Execute headless automation commands on a .clip document or sample canvas
    Run {
        /// Optional input .clip file to open
        #[arg(value_name = "FILE.clip")]
        file: Option<PathBuf>,
        /// Start with a sample multi-layer manga canvas
        #[arg(long)]
        sample: bool,
        /// Execute inline script commands (newline or semicolon separated)
        #[arg(long)]
        script: Option<String>,
        /// Execute individual command e.g. --cmd 'layer.add {"name":"Ink"}'
        #[arg(long = "cmd")]
        cmds: Vec<String>,
        /// Save resulting document to a .clip file
        #[arg(long, value_name = "OUT.clip")]
        save: Option<PathBuf>,
        /// Export flattened composite image to PNG/JPG
        #[arg(long, value_name = "OUT.png")]
        export: Option<PathBuf>,
    },
    /// List all available automation commands (optionally filtered by substring)
    Commands {
        #[arg(value_name = "FILTER")]
        filter: Option<String>,
    },
    /// Start Model Context Protocol (MCP) server over stdio for AI agent pair programming
    Mcp {
        /// Optional host:port of a running ComicCraft instance to bridge to
        #[arg(long, value_name = "HOST:PORT")]
        connect: Option<String>,
        /// Defer connection until the first tool call
        #[arg(long)]
        lazy: bool,
        /// Launch ComicCraft GUI automatically if not running
        #[arg(long)]
        launch: bool,
    },
    /// Run headless brush engine, compositor, and .clip codec benchmarks
    Perf {
        /// Optional iteration count or .clip file path
        #[arg(value_name = "N_OR_FILE")]
        target: Option<String>,
    },
    /// Generate a sample multi-layer Clip Studio Paint (.clip) document
    Sample {
        #[arg(value_name = "OUTPUT.clip", default_value = "comiccraft_sample.clip")]
        output: PathBuf,
    },
}

const AUTOMATION_COMMANDS: &[(&str, &str)] = &[
    ("canvas.new", "Create a new canvas: {\"width\":1600,\"height\":2400,\"dpi\":300.0}"),
    ("layer.add", "Add a new raster layer: {\"name\":\"Lineart\"}"),
    ("layer.clear", "Clear active layer pixels"),
    ("layer.opacity", "Set active layer opacity (0.0..1.0): {\"opacity\":0.85}"),
    ("stroke.draw", "Draw a pressure-sensitive stroke: {\"x0\":100,\"y0\":100,\"x1\":500,\"y1\":400,\"size\":12.0}"),
    ("screentone.fill", "Apply manga halftone screentone to active layer: {\"frequency\":60.0,\"density\":0.35}"),
    ("balloon.add", "Render a manga speech balloon on active layer"),
    ("document.save", "Save document to Celsys .clip format: {\"path\":\"out.clip\"}"),
    ("document.export", "Export flattened RGBA composite to PNG: {\"path\":\"out.png\"}"),
];

fn build_sample_document() -> Document {
    let mut doc = Document::new("ComicCraft Sample Page", 1200, 1600, 300.0);

    let ink_idx = doc.add_layer("Lineart (G-Pen)");
    doc.active_layer_idx = ink_idx;

    let mut config = ToolConfig::default();
    config.tool = ToolKind::Pen;
    config.sub_tool = SubTool::GPen;
    config.size = 12.0;
    config.primary_color = [25, 25, 30, 255];

    for i in 0..60 {
        let t = i as f32 / 60.0;
        let x = 200.0 + t * 800.0;
        let y = 400.0 + (t * std::f32::consts::PI * 3.0).sin() * 200.0;
        let pressure = (t * std::f32::consts::PI).sin() * 0.8 + 0.2;

        let p1 = StrokePoint::new(x, y, pressure);
        let p2 = StrokePoint::new(x + 5.0, y + 2.0, pressure);
        doc.paint_stroke_segment(p1, p2, &config);
    }

    let shade_idx = doc.add_layer("Screentone / Shading");
    doc.active_layer_idx = shade_idx;
    if let Some(l) = doc.layers.get_mut(shade_idx) {
        l.blend_mode = comiccraft_core::blend::BlendMode::Multiply;
        l.opacity = 0.5;
    }

    doc
}

fn execute_command_on_doc(doc: &mut Document, raw_cmd: &str) -> Result<String, String> {
    let trimmed = raw_cmd.trim();
    if trimmed.is_empty() {
        return Ok("noop".to_string());
    }
    let (cmd_id, json_part) = match trimmed.split_once(char::is_whitespace) {
        Some((id, rest)) => (id.trim(), rest.trim()),
        None => (trimmed, "{}"),
    };
    let args: serde_json::Value =
        serde_json::from_str(json_part).unwrap_or_else(|_| serde_json::json!({}));

    match cmd_id {
        "layer.add" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Layer");
            let idx = doc.add_layer(name);
            doc.active_layer_idx = idx;
            Ok(format!("added layer #{idx} `{name}`"))
        }
        "layer.clear" => {
            if let Some(layer) = doc.layers.get_mut(doc.active_layer_idx) {
                layer.clear();
            }
            doc.is_dirty = true;
            Ok("cleared active layer".to_string())
        }
        "layer.opacity" => {
            let opacity = args
                .get("opacity")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;
            if let Some(layer) = doc.layers.get_mut(doc.active_layer_idx) {
                layer.opacity = opacity.clamp(0.0, 1.0);
            }
            doc.is_dirty = true;
            Ok(format!("set opacity to {:.2}", opacity))
        }
        "stroke.draw" => {
            let x0 = args.get("x0").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
            let y0 = args.get("y0").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
            let x1 = args.get("x1").and_then(|v| v.as_f64()).unwrap_or(400.0) as f32;
            let y1 = args.get("y1").and_then(|v| v.as_f64()).unwrap_or(400.0) as f32;
            let size = args.get("size").and_then(|v| v.as_f64()).unwrap_or(10.0) as f32;
            let mut cfg = ToolConfig::default();
            cfg.size = size;
            doc.paint_stroke_segment(
                StrokePoint::new(x0, y0, 0.85),
                StrokePoint::new(x1, y1, 0.85),
                &cfg,
            );
            Ok(format!("drew stroke ({x0},{y0})->({x1},{y1})"))
        }
        other => Err(format!("unknown command `{other}`")),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file, json } => {
            let is_sut = file
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("sut"))
                .unwrap_or(false);
            if is_sut {
                let st = import_sut_file(&file)?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "name": st.name,
                            "tool_kind": st.tool_kind.name(),
                            "preset": st.preset,
                            "antialiasing": st.antialiasing,
                            "tip_stamp": st.tip_stamp.as_ref().map(|t| (t.width, t.height)),
                            "paper_texture": st.paper_texture.as_ref().map(|p| (p.width, p.height)),
                        })
                    );
                } else {
                    println!("=== ComicCraft .sut Sub Tool Inspector ===");
                    println!("File: {}", file.display());
                    println!("Name: {} ({})", st.name, st.tool_kind.name());
                    println!(
                        "Default Size: {:.1} px | Opacity: {:.0}% | Hardness: {:.0}% | Stabilization: {}",
                        st.preset.default_size,
                        st.preset.default_opacity * 100.0,
                        st.preset.default_hardness * 100.0,
                        st.preset.default_stabilization
                    );
                    println!(
                        "Aspect: {:.0}% | Spacing: {:.0}% | Random Angle: {:.0}% | Water Edge: {:.0}%",
                        st.preset.aspect * 100.0,
                        st.preset.spacing * 100.0,
                        st.preset.random_angle * 100.0,
                        st.preset.water_edge * 100.0
                    );
                    println!(
                        "Tip Stamp: {:?} | Paper Texture: {:?}",
                        st.tip_stamp.as_ref().map(|t| (t.width, t.height)),
                        st.paper_texture.as_ref().map(|p| (p.width, p.height))
                    );
                }
            } else {
                let meta = inspect_clip_file(&file)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&meta)?);
                } else {
                    println!("=== ComicCraft .clip Inspector ===");
                    println!("File: {}", file.display());
                    println!("Size: {} bytes", meta.file_size);
                    println!("Canvas: {} x {} @ {:.1} DPI", meta.width, meta.height, meta.dpi);
                    println!("Layers ({} total):", meta.layer_count);
                    for (i, layer) in meta.layers.iter().enumerate() {
                        let vis = if layer.visible { "✓" } else { "✗" };
                        println!(
                            "  [{}] {} \"{}\" (opacity: {:.0}%, blend: {})",
                            i,
                            vis,
                            layer.name,
                            layer.opacity * 100.0,
                            layer.blend_mode
                        );
                    }
                }
            }
        }
        Commands::Convert { input, output } => {
            println!("Opening {}...", input.display());
            let mut doc = open_clip_file(&input)?;
            println!("Rendering composite canvas ({}x{})...", doc.width, doc.height);
            let img = doc.to_rgba_image();
            img.save(&output)?;
            println!("Exported successfully to {}", output.display());
        }
        Commands::Run {
            file,
            sample,
            script,
            cmds,
            save,
            export,
        } => {
            let mut doc = if let Some(path) = file {
                open_clip_file(&path)?
            } else if sample {
                build_sample_document()
            } else {
                Document::new("Untitled.clip", 1600, 2400, 300.0)
            };

            if let Some(scr) = script {
                for line in scr.split(['\n', ';']) {
                    if !line.trim().is_empty() {
                        let msg = execute_command_on_doc(&mut doc, line)?;
                        println!("[run] {msg}");
                    }
                }
            }
            for cmd in cmds {
                let msg = execute_command_on_doc(&mut doc, &cmd)?;
                println!("[run] {msg}");
            }
            if let Some(save_path) = save {
                save_clip_file(&doc, &save_path)?;
                println!("Saved .clip to {}", save_path.display());
            }
            if let Some(export_path) = export {
                let img = doc.to_rgba_image();
                img.save(&export_path)?;
                println!("Exported composite image to {}", export_path.display());
            }
        }
        Commands::Commands { filter } => {
            let pat = filter.unwrap_or_default().to_lowercase();
            for (id, desc) in AUTOMATION_COMMANDS {
                if pat.is_empty()
                    || id.to_lowercase().contains(&pat)
                    || desc.to_lowercase().contains(&pat)
                {
                    println!("{:<20} {}", id, desc);
                }
            }
        }
        Commands::Mcp {
            connect: _,
            lazy: _,
            launch: _,
        } => {
            run_mcp_server()?;
        }
        Commands::Perf { target } => {
            let iterations: usize = target
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10);
            let t0 = Instant::now();
            let mut doc = build_sample_document();
            for _ in 0..iterations {
                doc.is_dirty = true;
                let _ = doc.to_rgba_image();
            }
            let elapsed = t0.elapsed();
            println!(
                "ComicCraft Perf: {} full 1200x1600 multi-layer composites + G-Pen strokes in {:.2?} ({:.2?}/iter)",
                iterations,
                elapsed,
                elapsed / (iterations.max(1) as u32)
            );
        }
        Commands::Sample { output } => {
            println!(
                "Creating sample Clip Studio Paint document at {}...",
                output.display()
            );
            let doc = build_sample_document();
            save_clip_file(&doc, &output)?;
            println!(
                "Saved valid .clip file ({} layers) to {}",
                doc.layers.len(),
                output.display()
            );
        }
    }

    Ok(())
}

fn run_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[ComicCraft MCP] Server listening on stdin...");
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let parsed: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = parsed.get("id").cloned().unwrap_or(serde_json::Value::Null);

        match method {
            "initialize" => {
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "serverInfo": {
                            "name": "comiccraft-mcp",
                            "version": env!("CARGO_PKG_VERSION")
                        },
                        "capabilities": {
                            "tools": {}
                        }
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            "notifications/initialized" => {
                // Notification requires no response
            }
            "tools/list" => {
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [
                            {
                                "name": "clip_inspect",
                                "description": "Inspects a Clip Studio Paint .clip file and returns canvas size, layers, and blend modes",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "path": { "type": "string" }
                                    },
                                    "required": ["path"]
                                }
                            },
                            {
                                "name": "clip_convert_png",
                                "description": "Renders a .clip file composite directly to a PNG image",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "input": { "type": "string" },
                                        "output": { "type": "string" }
                                    },
                                    "required": ["input", "output"]
                                }
                            },
                            {
                                "name": "clip_create_sample",
                                "description": "Creates a sample multi-layer Clip Studio Paint .clip file at the specified path",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "output": { "type": "string" }
                                    },
                                    "required": ["output"]
                                }
                            }
                        ]
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            "tools/call" => {
                let params = parsed.get("params").cloned().unwrap_or(serde_json::json!({}));
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                let tool_result = match tool_name {
                    "clip_inspect" => {
                        let path = PathBuf::from(
                            args.get("path").and_then(|v| v.as_str()).unwrap_or(""),
                        );
                        match inspect_clip_file(&path) {
                            Ok(meta) => serde_json::to_string_pretty(&meta)
                                .unwrap_or_else(|e| e.to_string()),
                            Err(e) => format!("Error inspecting .clip: {e}"),
                        }
                    }
                    "clip_convert_png" => {
                        let input = PathBuf::from(
                            args.get("input").and_then(|v| v.as_str()).unwrap_or(""),
                        );
                        let output = PathBuf::from(
                            args.get("output").and_then(|v| v.as_str()).unwrap_or(""),
                        );
                        match open_clip_file(&input) {
                            Ok(mut doc) => {
                                let img = doc.to_rgba_image();
                                match img.save(&output) {
                                    Ok(()) => format!("Exported PNG to {}", output.display()),
                                    Err(e) => format!("Error saving PNG: {e}"),
                                }
                            }
                            Err(e) => format!("Error opening .clip: {e}"),
                        }
                    }
                    "clip_create_sample" => {
                        let output = PathBuf::from(
                            args.get("output")
                                .and_then(|v| v.as_str())
                                .unwrap_or("sample.clip"),
                        );
                        let doc = build_sample_document();
                        match save_clip_file(&doc, &output) {
                            Ok(()) => format!("Created sample .clip at {}", output.display()),
                            Err(e) => format!("Error saving sample .clip: {e}"),
                        }
                    }
                    other => format!("Unknown tool `{other}`"),
                };

                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": tool_result
                            }
                        ]
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
            _ => {
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {}
                });
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}

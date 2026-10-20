#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod canvas_view;
mod clipboard_utils;
mod color_wheel;
mod icons;
pub mod updater;

use app::ComicCraftApp;
use comiccraft_clip::open_clip_file;
use eframe::egui::Vec2;
use std::io::Write as _;
use std::path::PathBuf;

/// Matches the `.desktop` file, macOS bundle id, and ArtCraft Launcher `CraftApp::bundle_id()`.
const APP_ID: &str = "ai.storyteller.comiccraft";

const VERSION_LINE: &str = concat!("comiccraft ", env!("CARGO_PKG_VERSION"), "\n");

const USAGE: &str = "\
Usage: comiccraft [options] [--] [files…]

Opens ComicCraft with the given .clip files.

Options:
  -h, --help                       Print this help and exit
  -V, --version                    Print the version and exit
      --screenshot <file.png>      Render the studio UI to a PNG file and exit
      --safe-gpu                   Start with the fallback renderer for this launch
      --in-window-menus            Keep the menus inside the window
      --new-instance               Start a separate ComicCraft instance
      --present-mode <mode>        How frames reach the display: fifo, autovsync or novsync
      --frame-latency <n>          Frames queued for the display: 1 to 3
      --stroke-trace <stderr|file.csv>
                                   Time each brush stroke's frames
      --control <port>             Start the localhost JSON-lines control server on this port
      --control-token <64-hex>     Control server token
      --control-token-file <path>  Read the control server token from this file
      --automation-read-root <dir> Directory the control server may read files from
      --automation-write-root <dir>
                                   Directory the control server may write files to
  --                               Treat every later argument as a file, even one starting with `-`
";

fn print_stdout(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(text.as_bytes()).and_then(|()| out.flush());
}

fn print_stderr(text: &str) {
    let mut err = std::io::stderr().lock();
    let _ = err.write_all(text.as_bytes()).and_then(|()| err.flush());
}

fn main() -> eframe::Result<()> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut screenshot_path: Option<PathBuf> = None;
    let mut after_double_dash = false;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        if after_double_dash {
            files.push(PathBuf::from(arg));
            continue;
        }
        match arg.as_str() {
            "--" => {
                after_double_dash = true;
            }
            "-h" | "--help" => {
                print_stdout(USAGE);
                std::process::exit(0);
            }
            "-V" | "--version" => {
                print_stdout(VERSION_LINE);
                std::process::exit(0);
            }
            "--screenshot" => {
                if let Some(p) = args.next() {
                    screenshot_path = Some(PathBuf::from(p));
                } else {
                    print_stderr("comiccraft: option `--screenshot` requires a path\n");
                    std::process::exit(2);
                }
            }
            "--safe-gpu" | "--in-window-menus" | "--new-instance" => {}
            "--present-mode"
            | "--frame-latency"
            | "--stroke-trace"
            | "--control"
            | "--control-token"
            | "--control-token-file"
            | "--automation-read-root"
            | "--automation-write-root" => {
                if args.next().is_none() {
                    print_stderr(&format!("comiccraft: option `{arg}` requires a value\n"));
                    std::process::exit(2);
                }
            }
            other if other.starts_with('-') => {
                print_stderr(&format!(
                    "comiccraft: unknown option `{other}` (try `comiccraft --help`)\n"
                ));
                std::process::exit(2);
            }
            path => {
                files.push(PathBuf::from(path));
            }
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_app_id(APP_ID)
            .with_title("CLIP STUDIO PAINT / ComicCraft — ArtCraft Studio Edition")
            .with_inner_size(Vec2::new(1600.0, 1000.0))
            .with_min_inner_size(Vec2::new(1024.0, 700.0))
            .with_drag_and_drop(true),
        renderer: eframe::Renderer::Wgpu,
        vsync: true,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "ComicCraft",
        native_options,
        Box::new(move |cc| {
            app::setup_csp_dark_theme(&cc.egui_ctx);
            let mut app = ComicCraftApp::default();
            app.updater.check_for_updates(cc.egui_ctx.clone());
            app.screenshot_target = screenshot_path;
            if let Some(first_file) = files.into_iter().next() {
                match open_clip_file(&first_file) {
                    Ok(doc) => {
                        app.doc = doc;
                        app.file_path = Some(first_file.clone());
                        app.status_message = format!("Opened {}", first_file.display());
                    }
                    Err(e) => {
                        app.status_message =
                            format!("Failed to open {}: {}", first_file.display(), e);
                    }
                }
            }
            Ok(Box::new(app))
        }),
    )
}

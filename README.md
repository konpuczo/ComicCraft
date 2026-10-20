# ComicCraft — ArtCraft Clip Studio Paint 1:1 Edition

[![Language: Rust](https://img.shields.io/badge/language-Rust-DEA584.svg?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Ecosystem: ArtCraft](https://img.shields.io/badge/ecosystem-ArtCraft-8A2BE2.svg?style=flat-square)](https://getartcraft.com)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat-square)](LICENSE)

**ComicCraft** is an open-source, clean-room 1:1 recreation of **Clip Studio Paint** written in pure Rust. Built for manga creators, comic artists, webtoon authors, and digital illustrators, ComicCraft provides native performance, a responsive dark studio UI, stroke stabilization, and full compatibility with Clip Studio Paint `.clip` project files.

---

## 🎨 Key Features (1:1 with Clip Studio Paint)

### 1. Studio Workspace Layout
- **Dark Studio Theme**: Native palette matching Clip Studio Paint (`#2b2d30`, `#1e1f22`, accent `#2d73d7`).
- **Interactive Viewport**: Infinite smooth zoom (10% to 3200%), canvas rotation (-180° to +180°), and horizontal/vertical canvas flipping.
- **Top Command Bar**: Quick access to File, Open, Save, Undo, Redo, Clear, and Layer operations.

### 2. Inking & Brush Engine
- **Stroke Stabilizer (Streamline)**: Dynamic weighted moving-average smoothing (0 to 50) for crisp, stutter-free lineart.
- **Sub-Tools**:
  - G-Pen, Mapping Pen (Maru), Real G-Pen, Calligraphy.
  - Darker Pencil, Design Pencil.
  - Dense Watercolor.
  - Soft & Hard Airbrush.
  - Hard & Soft Eraser.
- **Tool Properties**: Brush size slider with instant presets, opacity, stabilization, and edge hardness.

### 3. Layer System & Compositing
- Multi-layer stack with individual visibility toggles, layer opacity sliders, and layer reordering.
- Full blend modes matching Clip Studio Paint:
  - `Normal`, `Multiply`, `Screen`, `Overlay`, `Darken`, `Lighten`, `Color Dodge`, `Color Burn`, `Hard Light`, `Soft Light`, `Difference`, `Exclusion`.
- Rapid layer clearing, merging, and active layer switching.

### 4. Color Wheel & Palette
- Circular HSV hue ring with interior saturation/value gradient box.
- Foreground and background color swatches with quick swap (`⇄`) and transparency modes.

### 5. Genuine `.clip` File Format Compatibility
- Read and write Clip Studio Paint `.clip` containers:
  - Big-endian chunk container (`CSFCHUNK`, `CHNKHead`, `CHNKSQLi`, `CHNKFoot`).
  - Embedded SQLite database storing `Canvas` and `Layer` hierarchies.
  - Zlib-compressed raster tile data with CSP Adler-32 checksums.
- Also supports direct composite export to PNG and JPEG.

### 6. ArtCraft Ecosystem & MCP Agent Integration
- **CLI (`comiccraft-cli`)**: Headless tool implementing the full ArtCraft CLI suite (`info`, `convert`, `run`, `commands`, `mcp`, `perf`, `sample`, `--version`).
- **Model Context Protocol (`comiccraft-cli mcp`)**: Built-in JSON-RPC 2.0 server on stdio (`initialize`, `tools/list`, `tools/call`) allowing AI coding assistants to programmatically inspect, render, and manipulate `.clip` drawing projects.
- **ArtCraft Launcher Integration**:
  - Embeds Windows PE `VERSIONINFO` (`ProductName = "ComicCraft"`, `ProductVersion = "0.1.0"`, `CompanyName = "Learning Machines LLC"`).
  - Registered in `%LOCALAPPDATA%\ArtCraft Launcher\installed.json` (`C:\Users\<User>\AppData\Local\ArtCraft Launcher\installed.json`) and `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\ComicCraft`.

---

## 📁 Repository Structure

```
C:\ArtCraft/
├── Cargo.toml                    # Workspace definition (MIT OR Apache-2.0)
├── README.md                     # Documentation & specification
└── crates/
    ├── comiccraft-core/          # Document model, layers, stabilizer, brush physics, blend math
    ├── comiccraft-clip/          # .clip binary parser and serializer (SQLite + CSFCHUNK)
    ├── comiccraft-cli/           # Headless CLI & MCP agent server (comiccraft-cli.exe)
    └── comiccraft-ui/            # 1:1 Studio GUI (eframe / egui, comiccraft.exe)
```

---

## 🚀 Building and Running

### Prerequisites
- Rust 1.85+ (MSRV)
- Standard Rust toolchain (`stable-x86_64-pc-windows-gnu` or MSVC)

### Run the Studio GUI:
```bash
cargo run --package comiccraft-ui --release
```

### Run the CLI:
```bash
# Inspect a .clip file:
cargo run --package comiccraft-cli -- info artwork.clip

# Convert .clip to flattened PNG:
cargo run --package comiccraft-cli -- convert artwork.clip output.png

# Run headless automation commands on a document:
cargo run --package comiccraft-cli -- run --sample --cmd "layer.add {\"name\":\"FX\"}" --save out.clip --export out.png

# List available automation commands:
cargo run --package comiccraft-cli -- commands

# Run headless compositor & brush engine benchmark:
cargo run --package comiccraft-cli -- perf 20

# Generate a sample multi-layer .clip project:
cargo run --package comiccraft-cli -- sample my_comic.clip

# Run the MCP server for AI pair programming:
cargo run --package comiccraft-cli -- mcp
```

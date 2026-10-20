# ComicCraft — AI Agent Operational Guidelines

You are an expert systems engineer and graphics programmer assisting with **ComicCraft**, a clean-room 1:1 pure Rust recreation of **Clip Studio Paint (PRO/EX)** for desktop manga, comic, and illustration workflows.

---

## 1. Project Mission & Identity

- **Name**: ComicCraft (also referred to as ArtCraft Clip Studio Paint Edition)
- **Repository**: [https://github.com/konpuczo/ComicCraft.git](https://github.com/konpuczo/ComicCraft.git)
- **Language**: Pure Rust (MSRV 1.85+, 2021 edition)
- **Target Parity**: 1:1 clean-room behavioral and format compatibility with Clip Studio Paint (PRO/EX).
- **Core Principle**: Zero regressions in rendering math, $O(1)$ zero-allocation UI hot paths, pixel-perfect layer compositing, and strict compatibility with `.clip` and `.sut` file formats.

---

## 2. Workspace & Crate Architecture

The repository is organized as a Cargo workspace with four core crates:

```
ComicCraft/
├── Cargo.toml                    # Workspace definition & release profile (LTO=thin, strip=true, opt-level=3)
├── GEMINI.md                     # Antigravity project memory & roadmap
├── AGENTS.md                     # Universal instructions for AI coding assistants
├── CLAUDE.md                     # Claude Code / Anthropic instructions
├── docs/                         # Architecture documentation & roadmap specifications
│   ├── clip_studio_feature_audit_and_roadmap.md
│   └── LOCAL_AI_AGENT_GUIDE.md
└── crates/
    ├── comiccraft-core/          # Engine: Document model, layers, Catmull-Rom stabilizer, brush engine, blend math
    ├── comiccraft-clip/          # Format: .clip SQLite & CSFCHUNK roundtrip parser/writer, .sut extractor
    ├── comiccraft-cli/           # Headless: CLI commands & MCP (Model Context Protocol) JSON-RPC server
    └── comiccraft-ui/            # GUI: Native desktop application (egui / eframe) with CSP dark theme
```

### Crate Responsibilities
1. **`comiccraft-core`**:
   - `Document`: Manages canvas dimensions, resolution (DPI), layer stack, selection state, rulers, and comic frame gutters.
   - `Layer`: Pixel buffer (`Vec<u8>` RGBA8, $W \times H \times 4$), 8-bit alpha mask (`Vec<u8>`), blend mode, opacity ($0.0 \dots 1.0$), lock flags (alpha lock, lock layer), layer kind (Raster, Vector, Folder).
   - `BlendMode`: 12 exact Clip Studio blend modes (Normal, Multiply, Screen, Overlay, Darken, Lighten, Color Dodge, Color Burn, Hard Light, Soft Light, Difference, Exclusion).
   - `StrokeStabilizer`: Catmull-Rom spline interpolation and Streamline dynamic weighted moving average.
   - `Selection`: 1-bit / 8-bit selection mask, marching ants polygon outline generation, morphological operations (Expand, Shrink, Border).
   - `Rulers`: Symmetrical ruler ($N$-way reflection/radial mirror), Perspective ruler (1P, 2P, 3P vanishing point angle snapping).
   - `Filters`: Binarization (1-bit manga threshold), Levels, Gradient Map, HSV adjustments, Gaussian blur.

2. **`comiccraft-clip`**:
   - High-fidelity binary parser/serializer for `.clip` files.
   - Reads/writes outer `CSFCHUNK` container (`CHNKHead`, `CHNKExta`, `CHNKSQLi`, `CHNKFoot`).
   - Interacts with embedded SQLite database (`Canvas`, `Layer`, `LayerParam`, `ImageBlock`).
   - Encodes/decodes zlib raster blocks with Adler-32 verification.
   - Parses `.sut` (Clip Studio Sub-Tool) SQLite containers to extract brush dynamics and tip textures.

3. **`comiccraft-cli`**:
   - Binary: `comiccraft-cli.exe`.
   - Headless batch processing: `info`, `convert`, `run`, `commands`, `perf`, `sample`.
   - **MCP Server**: Implements `comiccraft-cli mcp` (JSON-RPC 2.0 stdio server) allowing AI agents to query, draw, and inspect documents via MCP tools.

4. **`comiccraft-ui`**:
   - Binary: `comiccraft.exe`.
   - Built on `eframe` / `egui 0.29`.
   - Dark studio color palette (`#2b2d30`, `#1e1f22`, accent `#2d73d7`).
   - Zero-allocation tab bar, customizable panels, navigator thumbnail, toolbar, tool properties, color wheel, and floating selection launcher.

---

## 3. Essential Commands

### Build & Check
```powershell
# Fast syntax & type check across all crates
cargo check --workspace

# Run all unit and integration tests (25 suites)
cargo test --workspace

# Run tests for a specific crate
cargo test -p comiccraft-core
cargo test -p comiccraft-clip
cargo test -p comiccraft-ui

# Build production optimized GUI binary (target/release/comiccraft.exe)
cargo build --release --bin comiccraft

# Build headless CLI & MCP server (target/release/comiccraft-cli.exe)
cargo build --release --bin comiccraft-cli
```

### Running Applications
```powershell
# Run the Desktop Studio GUI:
cargo run --package comiccraft-ui --release

# Inspect a .clip project file:
cargo run --package comiccraft-cli -- info path/to/project.clip

# Convert .clip to flattened PNG:
cargo run --package comiccraft-cli -- convert artwork.clip output.png

# Headless batch manipulation:
cargo run --package comiccraft-cli -- run --sample --cmd 'layer.add {"name":"Inks"}' --export out.png

# Launch MCP pair-programming server:
cargo run --package comiccraft-cli -- mcp
```

---

## 4. Coding Standards & Rust Invariants

When reading, proposing, or editing code in this workspace, follow these mandatory rules:

1. **No External Unapproved Crates**: Keep dependencies minimal. Workspace already contains `egui`, `eframe`, `image`, `rusqlite`, `clap`, `serde`, `bytemuck`, `rayon`, `parking_lot`, and `arboard`.
2. **Explicit Float Types in egui**: Always write `1.0_f32` or `0.0_f32` for `Stroke::new()` or dimensions in `comiccraft-ui` to avoid Rust 2024 float literal fallback warnings.
3. **Pixel Layout**: All raster buffers in `comiccraft-core` are straight RGBA8 ($4 \times W \times H$ bytes). Row-major, top-to-bottom. Alpha lock must preserve existing layer alpha while writing new RGB.
4. **Preserve Documentation**: Never strip or truncate existing comments, ASCII diagrams, docstrings, or tests unless specifically instructed.
5. **Compositing Safety**:
   - Clamp all color channel arithmetic to $0 \dots 255$.
   - Never allow integer overflows in pixel loops; use `u32` or `f32` intermediate accumulators.
   - Utilize SIMD / Rayon where available for full-canvas operations.
6. **Zero Panics in UI Loops**:
   - `eframe::update` code paths must never call `.unwrap()` or `.expect()` on dynamic canvas or document state. Use `if let` or graceful error logging.
7. **Clip Roundtrip Parity**:
   - Changes to `comiccraft-clip` must keep `tests/clip_roundtrip.rs` passing. The generated `.clip` files must open seamlessly in Clip Studio Paint.

---

## 5. Active Phase 3 Roadmap Tasks

Prioritize the following roadmap milestones when requested:

1. **In-Canvas Typography / Text Tool (`T`)**:
   - On-canvas interactive text boxes, font size, line spacing, horizontal vs vertical Japanese text (Tategaki), text outline border.
2. **Liquify Tool (`J`)**:
   - Pixel displacement mesh/brushes: Push, Expand (inflate), Pinch (deflate), and Twirl.
3. **Mesh Transformation Grid**:
   - $N \times M$ Bezier spline control grid for anatomical and clothing distortion.
4. **Interactive Comic Frame Gutter Slicer**:
   - Panel border slicing with horizontal/vertical/diagonal cutters and customizable gutters.
5. **2D Animation Engine (CSP EX)**:
   - Timeline dock panel with frame scrubber, cel folder mapping, colored onion skinning (previous blue / next green), and GIF/APNG export.

---

## 6. How to Run MCP with AI Assistants

`comiccraft-cli` has a built-in MCP server. To connect Claude Desktop, Cursor, Cline, or Roo Code:

```json
{
  "mcpServers": {
    "comiccraft": {
      "command": "D:\\projects\\ComicCraft\\target\\release\\comiccraft-cli.exe",
      "args": ["mcp"]
    }
  }
}
```

The MCP server exposes tools to:
- `open_document`: Load `.clip` files into the session.
- `inspect_layers`: Inspect layer tree, blend modes, opacities, and dimensions.
- `render_canvas`: Export full canvas preview to PNG data URI or disk.
- `execute_command`: Run headless script commands (draw line, flood fill, apply filter, add layer).

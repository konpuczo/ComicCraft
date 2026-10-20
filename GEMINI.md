# ComicCraft Studio — Antigravity Project Guide

Welcome to the dedicated Antigravity workspace for **ComicCraft Studio**.

- **GitHub Repository**: [https://github.com/konpuczo/ComicCraft.git](https://github.com/konpuczo/ComicCraft.git)
- **Architecture**: 1:1 Clean-room pure Rust recreation of Clip Studio Paint (PRO/EX)
- **Detailed Audit & Roadmap**: See [`docs/clip_studio_feature_audit_and_roadmap.md`](./docs/clip_studio_feature_audit_and_roadmap.md)

---

## 1. Project Overview & Crate Architecture

ComicCraft Studio is structured as a modular Cargo workspace:

```
C:\ArtCraft/
├── Cargo.toml                    # Workspace definition
├── README.md                     # Documentation & specifications
├── GEMINI.md                     # Antigravity project memory & guidelines
├── docs/                         # Specifications & feature gap analysis
│   └── clip_studio_feature_audit_and_roadmap.md
└── crates/
    ├── comiccraft-core/          # Document model, layers, Catmull-Rom stabilizer, brush engine, blend math
    ├── comiccraft-clip/          # .clip SQLite & CSFCHUNK roundtrip parser/writer, .sut extractor
    ├── comiccraft-cli/           # Headless CLI & MCP pair-programming server
    └── comiccraft-ui/            # Native Studio GUI (egui/eframe) with authentic CSP dark theme
```

---

## 2. Current Implementation Status

### Phase 1 & 2 Core (Fully Implemented & Verified — 25 Test Suites Passing)
- **Multi-Document Tabs**: $O(1)$ zero-allocation tab switching (`switch_tab`), independent history/viewport per tab, `Ctrl+W` tab closing.
- **Inking & Brush Dynamics**: Real-time Catmull-Rom smoothing, stylus pressure gamma, paper texture grain sampling, EMA stroke stabilization, transparent erase mode (`C`).
- **Layers & Blending**: 12 blend modes, clipping masks, alpha lock, draft layers (blue pencil), reference layers (lighthouse), layer folders, and 8-bit alpha layer masks.
- **Layer Operations**: `Duplicate Layer` (`Ctrl+J`), `Merge Down` (`Ctrl+E`), `Merge Visible`, and `Flatten Image`.
- **Selection System**: Marquee Rect/Ellipse, Lasso, Auto-Select (Magic Wand), Close-and-Fill (Lasso Fill cavity detection), Selection Floating Launcher, and Selection Border (`Inside`, `Centered`, `Outside`), Expand, and Shrink dialogs.
- **Quick Mask Mode (`Ctrl+Q`)**: Instant toggle between marching ants and translucent red paintable mask layer.
- **Manga Lineart Scans**: `Edit > Convert Brightness to Opacity` (instant white-to-transparent conversion for traditional lineart).
- **Rulers & Perspective**: Symmetrical Ruler (2-way mirror to 16-way radial kaleidoscope), Perspective Ruler (1P, 2P, 3P vanishing points with real-time brush angle snapping), and Cyan Canvas Guidelines.
- **Tonal Corrections**: Level Correction (Levels LUT), Binarization (1-bit manga inking threshold), Gradient Maps (Sunset, Sepia, Anime Sky, Cyberpunk), Brightness/Contrast, Hue/Saturation/Luminosity, Invert Colors (`Ctrl+I`), and Gaussian Blur.
- **Manga Guidelines & Speed Lines**: Bleed margins, trim marks (Tombō), inner frame margins, radiation focus lines, directional stream lines, and speech balloons.
- **Navigator**: Live canvas bird's-eye view thumbnail with interactive draggable red viewport rectangle.
- **System Clipboard**: Copy (`Ctrl+C`), Copy Merged (`Ctrl+Shift+C`), Cut (`Ctrl+X`), Paste (`Ctrl+V`), and New from Clipboard.

---

## 3. Active Next Wave Roadmap (Phase 3)

1. **In-Canvas Typography / Text Engine (`T`)**:
   - Interactive on-canvas text box editing, system font selection, font size, color, bold, italic, horizontal vs vertical Japanese text (Tategaki), and text outline border.
2. **Liquify Tool (`J`)**:
   - Pixel displacement brushes: Push, Expand (inflate), Pinch (deflate), and Twirl.
3. **Mesh Transformation**:
   - $N \times M$ spline control grid warping for fabric and anatomical distortion.
4. **Interactive Comic Frame Gutter Slicing Tool**:
   - On-canvas panel border slicing with horizontal/vertical/diagonal cutters and configurable gutters.
5. **2D Animation Engine (CSP EX)**:
   - Timeline dock panel with frame scrubber, cel folder mapping, colored onion skinning (previous blue / next green), and GIF/APNG export.

---

## 4. Development & Build Commands

- **Run all unit & integration tests**:
  ```powershell
  cargo test --workspace
  ```
- **Check code across all crates**:
  ```powershell
  cargo check --workspace
  ```
- **Run the GUI application in release mode**:
  ```powershell
  cargo run --package comiccraft-ui --release
  ```
- **Build production release executable**:
  ```powershell
  cargo build --release --bin comiccraft
  # Output: target/release/comiccraft.exe
  ```

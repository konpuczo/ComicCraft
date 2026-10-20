# Local AI Agent Handbook for ComicCraft Studio

Welcome! This guide is written for software engineers who want to pair-program on **ComicCraft** using local AI models (such as models hosted via **Ollama**, **LM Studio**, **LocalAI**, **vLLM**, or local agent frameworks like **Cline**, **Roo Code**, **Aider**, **Cursor**, **Windsurf**, or **Claude Code**).

---

## 1. Quick Model Recommendations

When working with ComicCraft's Rust codebase locally, the following models provide the strongest Rust systems programming and graphics math capabilities:

| Model | Recommended Size | Strengths |
|---|---|---|
| **Qwen 2.5 Coder** | `32B-Instruct` or `14B-Instruct` | Exceptional Rust syntax, `egui` API knowledge, 32k/128k context support. |
| **DeepSeek Coder V2.5 / V3 / R1** | Full or GGUF Q4_K_M / Q8_0 | Superb algorithmic reasoning for blend math, Catmull-Rom splines, and binary parser logic. |
| **Llama 3.3 / 3.1** | `70B-Instruct` | Strong general architecture, refactoring, and code comprehension. |
| **Command R+ / Starcoder2** | `35B+` | Great tool-calling and documentation reference. |

---

## 2. Universal System Prompt Template

Copy and paste the prompt below into the **System Prompt** / **Custom Instructions** field of your local LLM runner (LM Studio, Ollama `Modelfile`, Open WebUI, Jan, or Chatbox):

```text
You are an expert Rust systems and computer graphics engineer assisting with ComicCraft Studio (https://github.com/konpuczo/ComicCraft).

Project Overview:
- Clean-room 1:1 pure Rust recreation of Clip Studio Paint (PRO/EX).
- Workspace crates:
  1. crates/comiccraft-core: Document model, layers, Catmull-Rom stabilizer, brush engine, 12 blend modes, selection math, perspective & symmetrical rulers, manga lines & speech balloons.
  2. crates/comiccraft-clip: SQLite & CSFCHUNK .clip container parser/serializer, .sut brush importer.
  3. crates/comiccraft-cli: Headless CLI & Model Context Protocol (MCP) server.
  4. crates/comiccraft-ui: Native desktop studio UI (egui/eframe 0.29) with authentic CSP dark palette.

Coding Invariants:
1. Pure Rust (MSRV 1.85+, 2021 edition). Zero unapproved external dependencies.
2. Pixel buffers are always row-major straight RGBA8 (Vec<u8> length 4 * width * height).
3. UI code in comiccraft-ui must always use explicit f32 literals for egui::Stroke and dimensions (e.g. 1.0_f32, 1.5_f32).
4. Never call .unwrap() or .expect() on dynamic document or user data inside eframe::App::update loops.
5. All 25 test suites must pass: `cargo test --workspace`.
6. Changes to comiccraft-clip must maintain exact roundtrip binary and SQLite compatibility with real Clip Studio Paint files.

When responding:
- Keep explanations concise and code changes complete and drop-in ready.
- Always provide relevant cargo commands to verify changes.
```

---

## 3. Integrating with Local Agent Frameworks

### A. Cline / Roo Code (VS Code Extension)
1. Point your API provider to **Ollama** or **LM Studio** (`http://localhost:11434/v1` or `http://localhost:1234/v1`).
2. Select your installed coder model (e.g., `qwen2.5-coder:32b`).
3. Cline/Roo Code will automatically read the root [`AGENTS.md`](file:///d:/projects/ComicCraft/AGENTS.md) file.
4. Enable the ComicCraft MCP server in Cline's `cline_mcp_settings.json`:
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

### B. Aider (Command Line)
Launch Aider with your local model while in the `D:\projects\ComicCraft` directory:
```bash
# Using Ollama
aider --model ollama/qwen2.5-coder:32b --read AGENTS.md

# Using LM Studio / OpenAI-compatible endpoint
aider --openai-api-base http://localhost:1234/v1 --model local-model --read AGENTS.md
```

### C. Cursor / Windsurf
- Cursor automatically loads [`.cursorrules`](file:///d:/projects/ComicCraft/.cursorrules) and [`AGENTS.md`](file:///d:/projects/ComicCraft/AGENTS.md).
- Ensure the Rust Analyzer extension is pointing to the workspace root `D:\projects\ComicCraft`.

---

## 4. Key Architectural Patterns

### 1. Document & Layer Manipulation
Every document in ComicCraft is represented by `comiccraft_core::Document`:
```rust
use comiccraft_core::{Document, Layer, LayerKind, BlendMode};

// Create a new canvas (e.g., B5 Manga @ 300 DPI)
let mut doc = Document::new(2048, 2048, 300.0);

// Add a raster ink layer
let ink_layer = Layer::new_raster("Lineart", 2048, 2048);
let layer_id = doc.add_layer(ink_layer);

// Set blend mode & opacity
if let Some(layer) = doc.get_layer_mut(layer_id) {
    layer.blend_mode = BlendMode::Multiply;
    layer.opacity = 0.95;
}
```

### 2. Stroke & Brush Pipeline
Drawing strokes follow a 3-stage pipeline:
1. **Raw Input Points**: Stylus position $(x, y)$, pressure gamma ($0.0 \dots 1.0$), tilt, and timestamp.
2. **Stabilization**: `StrokeStabilizer` computes Catmull-Rom spline control points and applies Streamline weighted moving average smoothing.
3. **Raster Blatting / Interpolation**: Stamped brush circles with hardness falloff, grain sampling, and alpha compositing into the active layer's pixel buffer.

### 3. Blend Math
12 blend modes are computed per-pixel in `comiccraft_core::blend`:
- Standard formula: $C_{res} = (1 - \alpha_b) \cdot C_a + \alpha_b \cdot f(C_a, C_b)$
- Always clamp results to $0 \dots 255$.

### 4. `.clip` Container Format
A genuine Clip Studio Paint file is a composite binary container:
1. **Outer container**: Big-endian chunks (`CSFCHUNK`)
   - `CHNKHead`: Header block.
   - `CHNKExta`: Preview thumbnails & metadata.
   - `CHNKSQLi`: Embedded SQLite 3 database.
   - `CHNKFoot`: Footer block.
2. **Inner SQLite Database**:
   - `Canvas`: Width, height, DPI, bit depth.
   - `Layer`: Hierarchy, visibility, blend mode, clipping flags.
   - `ImageBlock`: 256x256 zlib-compressed raster tiles with custom Adler-32 check bytes.

---

## 5. Development & Verification Workflow

Whenever your local AI proposes or modifies code:

1. **Verify compilation**:
   ```powershell
   cargo check --workspace
   ```
2. **Run the full test suite**:
   ```powershell
   cargo test --workspace
   ```
3. **Verify `.clip` roundtrip tests**:
   ```powershell
   cargo test -p comiccraft-clip --test clip_roundtrip
   ```
4. **Launch the GUI to visually verify**:
   ```powershell
   cargo run --package comiccraft-ui --release
   ```

---

## 6. Phase 3 Feature Roadmap (Next To Implement)

If you ask your local AI to implement new features, choose from the prioritized Phase 3 roadmap:

1. **In-Canvas Typography / Text Engine (`T`)**:
   - Canvas text bounding box, font family picker, font size, color, Japanese vertical text (Tategaki), text stroke/outline.
2. **Liquify Tool (`J`)**:
   - Interactive raster warp brush: Push, Expand, Pinch, Twirl.
3. **Mesh Warp Transformation**:
   - $N \times M$ Bezier spline grid warping for clothing folds and figure distortion.
4. **Comic Frame Gutter Slicer Tool**:
   - On-canvas panel knife tool cutting rectangular frames into panels with configurable gutters.
5. **2D Animation Engine (CSP EX)**:
   - Frame timeline, cel folder mapping, colored onion skinning, GIF/APNG export.

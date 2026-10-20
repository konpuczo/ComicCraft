# CLAUDE.md — ComicCraft Project Guide

## 🎨 Identity & Mission
**ComicCraft** is a pure Rust, 1:1 clean-room recreation of **Clip Studio Paint (PRO/EX)** designed for desktop manga, comic, and illustration workflows.

## 🛠️ Build & Development Commands
- **Check code**: `cargo check --workspace`
- **Run all tests**: `cargo test --workspace`
- **Run specific test suite**: `cargo test --test csp_features`
- **Run GUI Studio**: `cargo run --package comiccraft-ui --release`
- **Build Release Binary**: `cargo build --release --bin comiccraft` (Output: `target/release/comiccraft.exe`)
- **Build CLI / MCP Tool**: `cargo build --release --bin comiccraft-cli` (Output: `target/release/comiccraft-cli.exe`)

## 🧱 Crate Hierarchy
- `crates/comiccraft-core`: Document model, layers, Catmull-Rom stabilizer, brush engine, 12 blend modes, selection math, perspective & symmetrical rulers, manga lines & speech balloons.
- `crates/comiccraft-clip`: SQLite & CSFCHUNK `.clip` container roundtrip parser and serializer, `.sut` brush importer.
- `crates/comiccraft-cli`: Headless CLI operations (`info`, `convert`, `run`, `commands`, `perf`, `sample`) and MCP stdio server (`comiccraft-cli mcp`).
- `crates/comiccraft-ui`: Native desktop UI (`eframe` / `egui 0.29`), CSP dark palette, toolbar, layer dock, color wheel, navigator, floating selection launcher.

## 📏 Coding Conventions
- **Float literals**: Always explicitly type float literals in UI code (e.g., `Stroke::new(1.0_f32, ...)` or `1.5_f32`) to satisfy future Rust editions without warnings.
- **Pixel arrays**: All pixel buffers in `comiccraft-core` are RGBA8 (`Vec<u8>`), size $W \times H \times 4$.
- **Error handling**: Avoid `.unwrap()` on canvas/document operations inside UI render loops.
- **Testing**: Maintain 100% pass rate across the 25 existing test suites. Run `cargo test --workspace` after every core or clip format modification.
- **Documentation**: Keep existing docstrings and comments intact.

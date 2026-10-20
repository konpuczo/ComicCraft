# ComicCraft — GitHub Copilot Instructions

ComicCraft is a pure Rust clean-room recreation of Clip Studio Paint (PRO/EX).

## Architecture
- `crates/comiccraft-core`: Document model, layers, Catmull-Rom stabilizer, brush engine, blend modes, selection math, manga lines, rulers.
- `crates/comiccraft-clip`: Clip Studio `.clip` container format (SQLite + CSFCHUNK parser/serializer) and `.sut` brush importer.
- `crates/comiccraft-cli`: Headless CLI and MCP server (`comiccraft-cli mcp`).
- `crates/comiccraft-ui`: Native desktop UI (`eframe` / `egui 0.29`) with authentic Clip Studio Paint dark palette.

## Code Standards
- Use explicit float typing (`1.0_f32`) for `egui::Stroke` and dimensions.
- Preserve 100% test pass rate (`cargo test --workspace`).
- Avoid panics (`.unwrap()`) in UI render loops.
- All layer buffers are straight RGBA8 `Vec<u8>`.

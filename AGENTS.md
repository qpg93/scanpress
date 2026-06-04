# ScanPress

## Build & Test
- Build release: `./build.sh release`
- Build debug: `./build.sh debug`
- Build all: `cargo build`
- Run all tests: `cargo test`
- Run CLI tests: `cargo test --test cli`
- Run compression tests: `cargo test --test compression`
- Run quick tests (exclude slow compression): `cargo test --lib`
- Format check: `cargo fmt --check`
- Lint (if clippy installed): `cargo clippy -- -D warnings`

## Project Structure
- `src/lib.rs` — library root, re-exports
- `src/bin/scanpress.rs` — CLI entry point (clap 4 derive)
- `src/config.rs` — output path building, validation, `CompressionResult`
- `src/compress/mod.rs` — compression pipeline, progress tracking (`ProgressMsg` / `ProgressCallback`), re-exports
- `src/compress/renderer.rs` — mupdf page rendering + in-place BT.601 grayscale
- `src/compress/pdf_builder.rs` — printpdf PDF reconstruction (sequential page rendering)
- `src/compress/searcher.rs` — binary search for optimal DPI / JPEG quality


## Style
- No comments in code unless documenting public API.
- Public API items must have `///` doc comments.
- CLI help text and error messages are in English.

## Key Design Decisions
- Grayscale: always render mupdf page to RGB first, then convert in-place using BT.601 luma coefficients (matching JPEG YCbCr).
- Output filename appends `-gray` suffix when grayscale enabled.
- Page rendering is sequential (MuPDF's `Document` is not `Sync`).

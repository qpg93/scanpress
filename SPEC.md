# ScanPress — Specification

## 1. Overview

A command-line tool for compressing scanned PDFs (image-based PDFs). Supports two compression modes: **target size** (auto-adjusts DPI and JPEG quality so the output fits within a size budget) and **fixed DPI** (re-samples pages at a specified DPI). Internally uses `mupdf` for page rendering, `image` for JPEG encoding, and `printpdf` for PDF reconstruction.

**Tech stack:**
- Rust 2021 edition
- CLI: `clap` 4
- Backend: `mupdf` 0.6, `image` 0.25, `printpdf` 0.7
- Progress bar: `indicatif` 0.17
- Testing: `tempfile`, `assert_cmd`, `predicates`

## 2. Project Structure

```
scanpress/
├── Cargo.toml
├── src/
│   ├── lib.rs                        # Library root, re-exports
│   ├── bin/scanpress.rs              # CLI entry point
│   ├── config.rs                     # Compression config, path building, validation, result formatting
│   ├── compress/
│   │   ├── mod.rs                    # Compression engine entry, ProgressTracker, re-exports
│   │   ├── renderer.rs              # mupdf page rendering → JPEG encoding
│   │   ├── pdf_builder.rs           # Sequential page rendering + printpdf PDF reconstruction
│   │   └── searcher.rs              # DPI / JPEG quality binary search
├── tests/
│   ├── cli.rs                        # CLI argument tests (assert_cmd)
│   ├── compression.rs                # Integration compression tests
```

## 3. Core Data Structures

### `CompressionTaskConfig` (config.rs)
```rust
input_path: PathBuf          // Input PDF path
output_path: PathBuf         // Output PDF path
dpi: Option<u32>             // None = target-size mode, Some = fixed DPI
size_limit_bytes: Option<u64>  // None = fixed-DPI mode, Some = target size (bytes)
jpeg_quality: u8             // JPEG quality 1–100
grayscale: bool              // Whether to output grayscale
min_dpi: u32                 // DPI search range lower bound
max_dpi: u32                 // DPI search range upper bound
```

### `ProgressMsg` & `ProgressCallback` (compress/mod.rs)
```rust
pub enum ProgressMsg<'a> {
    Label(&'a str),   // Updates the progress bar's current message
    Line(&'a str),    // Prints a persistent line (e.g. above the bar)
}

pub type ProgressCallback = Box<dyn FnMut(u32, u32, ProgressMsg) + Send>;
```

Binary search results are sent as both `Label` and `Line` so the user sees each trial verdict persistently.

### `CompressionResult` (config.rs)
```rust
input_path: PathBuf
output_path: PathBuf
selected_dpi: u32
selected_quality: u8
size_limit_bytes: Option<u64>
original_size: u64
output_size: u64
// Methods: saved_bytes() -> i64, saved_ratio() -> f64
```

## 4. Business Logic

### `config.rs` — Paths, Validation, Formatting
- `build_output_path()` — Generate output path by mode (default naming rules)
- `parse_size_to_bytes()` — Parse `"5MB"`, `"800KB"`, `"1.5MB"` etc.
- `prepare_compression_task()` — Validate inputs and build task config
- `format_result_summary()` — Format English result summary
- `validate_dpi_range()` — Validate DPI range bounds
- `clamp_jpeg_quality()` — Validate quality range

### `compress/` — Compression Engine
- `run_compression(config, progress_callback)` — Core compression entry point
- In-place BT.601 grayscale conversion (no extra allocation, rounded luma)
- DPI auto-evaluation / JPEG quality auto-evaluation (binary search)
- Progress messages emitted through `ProgressCallback` with two variants:
  - `ProgressMsg::Label(&str)` — updates the progress bar's current message
  - `ProgressMsg::Line(&str)` — prints a persistent line above the progress bar
- Messages sent during compression:
  - `"Starting PDF compression..."` at start (Label)
  - `"Trying DPI {n}..."` before each binary-search trial (Label)
  - `"DPI {n} → {:.2} MB ({:+.1}% over {:.2} MB) — over target"` when trial exceeds target (Line + Label)
  - `"DPI {n} → {:.2} MB (✓ within {:.2} MB)"` when trial fits target (Line + Label)
  - `"Trying JPEG quality {n}..."` before each quality trial (Label)
  - `"Quality {n} → {:.2} MB ({:+.1}% over {:.2} MB) — over target"` when quality trial exceeds target (Line + Label)
  - `"Quality {n} → {:.2} MB (✓ within {:.2} MB)"` when quality trial fits target (Line + Label)
  - `"Rendered page X / Y"` per page rendered (Label)
  - `"Trying lower JPEG quality..."` entering quality fallback (Label)
  - `"Compression complete"` at end (Label)

### `bin/scanpress.rs` — CLI Progress Output
- Uses `indicatif::ProgressBar` for a tqdm-style progress bar on stdout
- Bar template: `[{bar:40}] {percent:>3}% | {elapsed_precise} | {msg}`
- `ProgressMsg::Line` messages are printed via `bar.suspend(|| eprintln!(...))` — persistent above the bar
- `ProgressMsg::Label` messages update the bar's `{msg}` field
- Page-level rendering advances the bar; binary-search phase trial results appear as persistent lines above the bar
- After compression, bar finishes and final result summary is printed on stdout

## 5. Build & Run

```bash
./build.sh release         # optimized (production)
./build.sh debug           # unoptimized (development)
cargo build                                    # Compile (debug)
cargo test                                     # Run tests
cargo run --bin scanpress -- --help       # CLI help
```

## 6. Test Coverage

| Location      | Count | Coverage                          |
|---------------|-------|-----------------------------------|
| compress/mod.rs | 4   | Binary search estimates, progress tracker, progress totals |
| compress/searcher.rs | 3 | Target size rejection, quality fallback range |
| config.rs     | ~15   | Size parsing, quality validation, DPI validation, path building, prepare checks, result formatting |
| tests/cli.rs  | ~13   | CLI argument parsing (assert_cmd) |
| tests/compression.rs | ~20 | Compression integration (multi-page PDF), output naming, size parsing, quality validation |

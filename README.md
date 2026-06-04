# ScanPress

A command-line tool for compressing scanned PDFs (image-based PDFs). It re-renders each page as a bitmap, encodes it as JPEG, and rebuilds a new PDF — reducing file size while preserving acceptable readability.

Use cases: passports, ID cards, paper documents composed primarily of scanned images.

Not suitable for: PDFs that must retain a searchable text layer, selectable text, vector graphics, annotations, forms, or other structured content.

## Features

- Fixed-DPI compression — re-render pages at a specified DPI.
- Target-size compression — automatically search for the optimal DPI to fit within a size budget.
- Auto JPEG quality fallback — lowers quality if even the minimum DPI exceeds the target.
- Grayscale mode — reduces file size further for text or document scans.
- Custom JPEG quality, output path, and DPI search range.

## Prerequisites

Install the Rust toolchain via `rustup` (stable):

```bash
rustup default stable
cargo --version
```

This project uses the `mupdf` library — MuPDF source is bundled and compiled automatically during the build. No system dynamic libraries required.

## Build

```bash
./build.sh release         # optimized (production)
./build.sh debug           # unoptimized (development)
```

Or use `cargo` directly:

```bash
cargo build --release      # binary at target/release/scanpress
cargo build                # binary at target/debug/scanpress
```

The `build.sh` script copies the binary to the project root and removes `target/`.

## Usage

Fixed-DPI compression:

```bash
cargo run --bin scanpress -- input.pdf --dpi 200
```

Target-size compression (auto-DPI search):

```bash
cargo run --bin scanpress -- input.pdf --size 5MB
```

Specify output path:

```bash
cargo run --bin scanpress -- input.pdf --dpi 180 -o output.pdf
```

Grayscale mode:

```bash
cargo run --bin scanpress -- input.pdf --size 5MB --gray
```

Limit the DPI search range:

```bash
cargo run --bin scanpress -- input.pdf --size 5MB --min-dpi 100 --max-dpi 250
```

Or run the binary directly after building:

```bash
./target/release/scanpress input.pdf --dpi 200
```

## Arguments

- `pdf` — Input PDF file path.
- `--dpi DPI` — Fixed-DPI mode: re-render pages at the given DPI.
- `--size SIZE` — Target-size mode: e.g. `5MB`, `800KB`, `1048576B`.
- `--quality QUALITY` — JPEG quality, range `1–100`, default `75`.
- `--gray` — Convert to grayscale for smaller output.
- `-o OUTPUT` / `--output OUTPUT` — Output PDF file path.
- `--min-dpi MIN_DPI` — Minimum DPI for auto-search (default `72`).
- `--max-dpi MAX_DPI` — Maximum DPI for auto-search (default `300`).

`--dpi` and `--size` are mutually exclusive; one must be provided.

## Output Filename

When `-o` is omitted, the tool auto-generates a filename beside the input:

- Fixed-DPI mode: `{stem}.compressed.{dpi}dpi.pdf` (with `--gray`: `...gray.pdf`)
- Target-size mode: `{stem}.compressed.target-{size:.2}MB.pdf` (with `--gray`: `...gray.pdf`)

The tool refuses to overwrite the input file.

## Output Summary

After compression, a summary like this is printed:

```text
Input file: /path/to/input.pdf
Output file: /path/to/output.pdf
Actual DPI: 155
Actual JPEG quality: 75
Target limit: 5.00 MB
Input size: 12.51 MB
Output size: 4.96 MB
Size change: 7.56 MB (60.38%)
```

`Target limit` appears only in `--size` mode.

## How It Works

Each page of the original PDF is rendered through MuPDF at the specified (or auto-selected) DPI. The resulting image is JPEG-encoded and embedded into a new PDF at the original page dimensions.

In `--size` mode, a binary search finds the highest DPI that keeps the output within the target size. If even the minimum DPI exceeds the target, the tool fixes the DPI at its minimum and gradually lowers JPEG quality. If the target is too aggressive, an error is reported.

## Quality Guidelines

- `300 DPI` — good detail, larger files.
- `200 DPI` — balanced clarity and size.
- `150 DPI` — suitable for general viewing and submissions.
- `72–120 DPI` — maximum compression, noticeable detail loss.

## Caveats

- Re-rendering destroys the original text layer, vector objects, annotations, and form structure.
- Grayscale removes colour information — use only for black-and-white documents.
- Target size is not a hard guarantee; the tool reports an error if the target cannot be met.
- Always keep the original PDF in case the result is unsatisfactory.

## CI / Automation

This project uses GitHub Actions for continuous integration. On every push
and pull request to `main`, the CI workflow runs:

- `cargo fmt --check` — code formatting compliance
- `cargo build` — compilation check
- `cargo test --lib` — quick unit tests (excludes slow integration tests)

## Contributing

1. Ensure `cargo fmt --check` passes (formatting follows standard Rust style).
2. Run `cargo test --lib` for quick feedback before opening a PR.
3. All public API items must have `///` doc comments.
4. No inline comments unless documenting public API behavior.
5. CLI help text and error messages must be in English.

## License

Licensed under the GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later). See [LICENSE](LICENSE) for the full license text.

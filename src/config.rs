use std::path::{Path, PathBuf};

/// Minimum DPI boundary for the auto-search range.
pub const DEFAULT_MIN_DPI: u32 = 72;
/// Maximum DPI boundary for the auto-search range.
pub const DEFAULT_MAX_DPI: u32 = 300;
/// JPEG quality floor used when the binary search falls back to lowering
/// quality after hitting the minimum DPI.
pub const DEFAULT_MIN_QUALITY: u8 = 40;

/// Captures all parameters needed to drive a single compression run.
///
/// Built by [`prepare_compression_task`] after input validation. Exactly one
/// of `dpi` or `size_limit_bytes` must be `Some`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionTaskConfig {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub dpi: Option<u32>,
    pub size_limit_bytes: Option<u64>,
    pub jpeg_quality: u8,
    pub grayscale: bool,
    pub min_dpi: u32,
    pub max_dpi: u32,
}

/// Result summary for a completed compression run.
///
/// Returned by [`run_compression`](crate::run_compression) and displayed via
/// [`format_result_summary`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionResult {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub selected_dpi: u32,
    pub selected_quality: u8,
    pub size_limit_bytes: Option<u64>,
    pub original_size: u64,
    pub output_size: u64,
}

impl CompressionResult {
    /// Number of bytes saved (positive) or lost (negative).
    pub fn saved_bytes(&self) -> i64 {
        self.original_size as i64 - self.output_size as i64
    }

    /// Percentage of the original file size that was saved.
    pub fn saved_ratio(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            self.saved_bytes() as f64 / self.original_size as f64 * 100.0
        }
    }
}

/// Validate that a JPEG quality value is in the range `1..=100`.
/// Returns the same value on success, or an error if out of range.
pub fn validate_jpeg_quality(value: u8) -> anyhow::Result<u8> {
    if (1..=100).contains(&value) {
        Ok(value)
    } else {
        anyhow::bail!("JPEG quality must be between 1 and 100.")
    }
}

/// Check that a DPI search range is valid — both bounds are positive and
/// `min_dpi <= max_dpi`.
pub fn validate_dpi_range(min_dpi: u32, max_dpi: u32) -> anyhow::Result<()> {
    if min_dpi == 0 || max_dpi == 0 {
        anyhow::bail!("Minimum and maximum DPI must both be positive integers.")
    }

    if min_dpi > max_dpi {
        anyhow::bail!("--min-dpi cannot be greater than --max-dpi.")
    }

    Ok(())
}

/// Parse a human-readable size string (e.g. `"5MB"`, `"800KB"`, `"1.5MB"`,
/// `"1048576B"`) into a byte count. Returns an error for invalid formats.
pub fn parse_size_to_bytes(value: &str) -> anyhow::Result<u64> {
    let cleaned = value.trim().to_uppercase();
    let units = [
        ("GB", 1024_u64.pow(3)),
        ("MB", 1024_u64.pow(2)),
        ("KB", 1024),
        ("B", 1),
    ];

    for (unit, factor) in units {
        if let Some(number_text) = cleaned.strip_suffix(unit) {
            let number_text = number_text.trim();
            if number_text.is_empty() {
                break;
            }

            if let Ok(number) = number_text.parse::<f64>() {
                let bytes_value = (number * factor as f64) as u64;
                if number.is_finite() && bytes_value > 0 {
                    return Ok(bytes_value);
                }
            }
            break;
        }
    }

    anyhow::bail!("Invalid size format. Use e.g. 5MB, 800KB, 1048576B.")
}

/// Build the output PDF path.
///
/// If an explicit output path is given (and non-empty), it is used directly.
/// Otherwise an auto-generated path is produced:
/// - Fixed DPI mode: `{input_stem}.compressed.{dpi}dpi[-gray].pdf`
/// - Target-size mode: `{input_stem}.compressed.target-{size:.2}MB[-gray].pdf`
///
/// The `-gray` suffix is appended only when `grayscale` is true.
pub fn build_output_path(
    input_path: &Path,
    output_path: Option<&str>,
    dpi: Option<u32>,
    size_limit_bytes: Option<u64>,
    grayscale: bool,
) -> anyhow::Result<PathBuf> {
    if let Some(output_path) = output_path.filter(|value| !value.trim().is_empty()) {
        return absolutize_path(&expand_home(output_path));
    }

    let gray_suffix = if grayscale { "-gray" } else { "" };

    if let Some(size_limit_bytes) = size_limit_bytes {
        let size_mb = size_limit_bytes as f64 / 1024.0 / 1024.0;
        return Ok(input_path.with_file_name(format!(
            "{}.compressed.target-{size_mb:.2}MB{gray_suffix}.pdf",
            input_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("output")
        )));
    }

    if let Some(dpi) = dpi {
        return Ok(input_path.with_file_name(format!(
            "{}.compressed.{dpi}dpi{gray_suffix}.pdf",
            input_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("output")
        )));
    }

    anyhow::bail!("No DPI provided — cannot auto-generate output filename.")
}

/// Validate all user-supplied inputs and produce a ready-to-use
/// [`CompressionTaskConfig`].
///
/// Checks:
/// - At least one of `dpi` / `size_text` is provided (mutually exclusive).
/// - DPI is positive (when supplied).
/// - DPI range is valid.
/// - JPEG quality is in `1..=100`.
/// - Output path does not match the input path.
pub fn prepare_compression_task(
    pdf_path: impl AsRef<Path>,
    dpi: Option<u32>,
    size_text: Option<&str>,
    quality: u8,
    grayscale: bool,
    output: Option<&str>,
    min_dpi: u32,
    max_dpi: u32,
) -> anyhow::Result<CompressionTaskConfig> {
    let input_path = resolve_input_pdf_path(pdf_path.as_ref())?;
    let has_size = size_text.is_some_and(|value| !value.trim().is_empty());

    if dpi.is_none() && !has_size {
        anyhow::bail!("Must provide either --dpi or --size.")
    }

    if dpi.is_some() && has_size {
        anyhow::bail!("--dpi and --size are mutually exclusive.")
    }

    if dpi == Some(0) {
        anyhow::bail!("DPI must be a positive integer.")
    }

    validate_dpi_range(min_dpi, max_dpi)?;
    let jpeg_quality = validate_jpeg_quality(quality)?;
    let size_limit_bytes = if has_size {
        Some(parse_size_to_bytes(size_text.unwrap())?)
    } else {
        None
    };
    let output_path = build_output_path(&input_path, output, dpi, size_limit_bytes, grayscale)?;

    if paths_refer_to_same_file(&input_path, &output_path)? {
        anyhow::bail!("Output path cannot be the same as input path. Use a different filename.")
    }

    Ok(CompressionTaskConfig {
        input_path,
        output_path,
        dpi,
        size_limit_bytes,
        jpeg_quality,
        grayscale,
        min_dpi,
        max_dpi,
    })
}

/// Format a human-readable compression result summary with input/output
/// paths, selected DPI and JPEG quality, and size comparisons.
pub fn format_result_summary(result: &CompressionResult) -> String {
    let mut lines = vec![
        format!("Input file: {}", result.input_path.display()),
        format!("Output file: {}", result.output_path.display()),
        format!("Actual DPI: {}", result.selected_dpi),
        format!("Actual JPEG quality: {}", result.selected_quality),
    ];

    if let Some(size_limit_bytes) = result.size_limit_bytes {
        lines.push(format!(
            "Target limit: {:.2} MB",
            size_limit_bytes as f64 / 1024.0 / 1024.0
        ));
    }

    lines.extend([
        format!(
            "Input size: {:.2} MB",
            result.original_size as f64 / 1024.0 / 1024.0
        ),
        format!(
            "Output size: {:.2} MB",
            result.output_size as f64 / 1024.0 / 1024.0
        ),
        format!(
            "Size change: {:.2} MB ({:.2}%)",
            result.saved_bytes() as f64 / 1024.0 / 1024.0,
            result.saved_ratio()
        ),
    ]);

    lines.join("\n")
}

fn resolve_input_pdf_path(pdf_path: &Path) -> anyhow::Result<PathBuf> {
    let input_path = absolutize_path(&expand_home_path(pdf_path))?;
    if !input_path.is_file() {
        anyhow::bail!("Input PDF not found: {}", input_path.display())
    }

    if input_path
        .extension()
        .and_then(|suffix| suffix.to_str())
        .map(|suffix| suffix.eq_ignore_ascii_case("pdf"))
        != Some(true)
    {
        anyhow::bail!("Input file must have a .pdf extension.")
    }

    Ok(input_path)
}

fn expand_home(value: &str) -> PathBuf {
    if value == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home);
        }
    }

    if let Some(rest) = value.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }

    PathBuf::from(value)
}

fn expand_home_path(value: &Path) -> PathBuf {
    value
        .to_str()
        .map(expand_home)
        .unwrap_or_else(|| value.to_path_buf())
}

fn absolutize_path(path: &Path) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn paths_refer_to_same_file(input_path: &Path, output_path: &Path) -> anyhow::Result<bool> {
    let input = input_path.canonicalize()?;
    let output = output_path
        .canonicalize()
        .unwrap_or_else(|_| output_path.to_path_buf());
    Ok(input == output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn parses_size_units_to_bytes() {
        assert_eq!(parse_size_to_bytes("5MB").unwrap(), 5 * 1024 * 1024);
        assert_eq!(parse_size_to_bytes("800KB").unwrap(), 800 * 1024);
        assert_eq!(parse_size_to_bytes("1048576B").unwrap(), 1_048_576);
        assert_eq!(parse_size_to_bytes("1.5MB").unwrap(), 1_572_864);
    }

    #[test]
    fn rejects_invalid_size_text() {
        assert!(parse_size_to_bytes("").is_err());
        assert!(parse_size_to_bytes("MB").is_err());
        assert!(parse_size_to_bytes("0MB").is_err());
        assert!(parse_size_to_bytes("abc").is_err());
    }

    #[test]
    fn validates_jpeg_quality() {
        assert!(validate_jpeg_quality(0).is_err());
        assert_eq!(validate_jpeg_quality(1).unwrap(), 1);
        assert_eq!(validate_jpeg_quality(75).unwrap(), 75);
        assert_eq!(validate_jpeg_quality(100).unwrap(), 100);
        assert!(validate_jpeg_quality(101).is_err());
    }

    #[test]
    fn validates_dpi_range_order() {
        assert!(validate_dpi_range(72, 300).is_ok());
        assert!(validate_dpi_range(300, 72).is_err());
    }

    #[test]
    fn rejects_zero_dpi_range_values() {
        assert!(validate_dpi_range(0, 300).is_err());
        assert!(validate_dpi_range(72, 0).is_err());
    }

    #[test]
    fn builds_default_output_path_for_target_size() {
        let input = Path::new("/tmp/passport.pdf");
        let output = build_output_path(input, None, None, Some(5 * 1024 * 1024), false).unwrap();
        assert_eq!(
            output,
            Path::new("/tmp/passport.compressed.target-5.00MB.pdf")
        );
    }

    #[test]
    fn builds_default_output_path_for_fixed_dpi() {
        let input = Path::new("/tmp/passport.pdf");
        let output = build_output_path(input, None, Some(200), None, false).unwrap();
        assert_eq!(output, Path::new("/tmp/passport.compressed.200dpi.pdf"));
    }

    #[test]
    fn builds_default_output_path_with_gray_suffix() {
        let input = Path::new("/tmp/passport.pdf");
        let output = build_output_path(input, None, Some(200), None, true).unwrap();
        assert_eq!(
            output,
            Path::new("/tmp/passport.compressed.200dpi-gray.pdf")
        );
    }

    #[test]
    fn rejects_default_output_without_dpi_or_size() {
        let input = Path::new("/tmp/passport.pdf");
        let error = build_output_path(input, None, None, None, false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("No DPI provided"));
    }

    #[test]
    fn prepare_rejects_missing_and_conflicting_output_params() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("passport.pdf");
        fs::write(&input, b"%PDF-1.7\n").unwrap();

        let missing = prepare_compression_task(&input, None, None, 75, false, None, 72, 300)
            .unwrap_err()
            .to_string();
        assert!(missing.contains("Must provide either --dpi or --size"));

        let conflicting =
            prepare_compression_task(&input, Some(200), Some("5MB"), 75, false, None, 72, 300)
                .unwrap_err()
                .to_string();
        assert!(conflicting.contains("mutually exclusive"));
    }

    #[test]
    fn prepare_rejects_output_same_as_input() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("passport.pdf");
        fs::write(&input, b"%PDF-1.7\n").unwrap();

        let error = prepare_compression_task(
            &input,
            Some(200),
            None,
            75,
            false,
            Some(input.to_str().unwrap()),
            72,
            300,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Output path cannot be the same as input path"));
    }

    #[test]
    fn formats_result_summary() {
        let result = CompressionResult {
            input_path: Path::new("/tmp/in.pdf").to_path_buf(),
            output_path: Path::new("/tmp/out.pdf").to_path_buf(),
            selected_dpi: 155,
            selected_quality: 75,
            size_limit_bytes: Some(5 * 1024 * 1024),
            original_size: 12_582_912,
            output_size: 5_201_920,
        };

        let summary = format_result_summary(&result);
        assert_eq!(
            summary,
            "Input file: /tmp/in.pdf\n\
Output file: /tmp/out.pdf\n\
Actual DPI: 155\n\
Actual JPEG quality: 75\n\
Target limit: 5.00 MB\n\
Input size: 12.00 MB\n\
Output size: 4.96 MB\n\
Size change: 7.04 MB (58.66%)"
        );
    }
}

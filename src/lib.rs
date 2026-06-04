//! ScanPress — compress scanned PDFs by re-rendering pages as JPEG images.
//!
//! This crate provides two compression modes:
//! - **Fixed DPI**: re-render pages at a user-specified DPI.
//! - **Target size**: binary-search for the optimal DPI (and optionally JPEG
//!   quality) to keep the output within a size budget.

pub mod compress;
pub mod config;

pub use compress::{run_compression, ProgressCallback, ProgressMsg};
pub use config::{
    build_output_path, format_result_summary, parse_size_to_bytes, prepare_compression_task,
    validate_dpi_range, validate_jpeg_quality, CompressionResult, CompressionTaskConfig,
    DEFAULT_MAX_DPI, DEFAULT_MIN_DPI, DEFAULT_MIN_QUALITY,
};

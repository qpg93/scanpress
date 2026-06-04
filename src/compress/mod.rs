pub mod pdf_builder;
pub mod renderer;
pub mod searcher;

use crate::config::{CompressionResult, CompressionTaskConfig};
use anyhow::{Context, Result};
use mupdf::Document;
use std::fs;
use std::path::Path;

/// Message variants for the progress callback.
pub enum ProgressMsg<'a> {
    /// Updates the progress bar's current message label.
    Label(&'a str),
    /// Prints a persistent log line (e.g. above the progress bar).
    Line(&'a str),
}

/// Callback type for reporting compression progress.
///
/// # Parameters
/// - `current`: current step (0-indexed).
/// - `total`: total number of steps.
/// - `msg`: the progress message (label update or persistent line).
pub type ProgressCallback = Box<dyn FnMut(u32, u32, ProgressMsg) + Send>;

pub struct CompressionProgressTracker {
    current: u32,
    total: u32,
    callback: Option<ProgressCallback>,
}

impl CompressionProgressTracker {
    pub fn new(total: u32, callback: Option<ProgressCallback>) -> Self {
        Self {
            current: 0,
            total,
            callback,
        }
    }

    pub fn emit(&mut self, message: &str) {
        if let Some(callback) = self.callback.as_deref_mut() {
            callback(self.current, self.total, ProgressMsg::Label(message));
        }
    }

    pub fn log(&mut self, message: &str) {
        if let Some(callback) = self.callback.as_deref_mut() {
            callback(self.current, self.total, ProgressMsg::Line(message));
        }
    }

    pub fn advance(&mut self, message: &str) {
        self.current = self.current.saturating_add(1).min(self.total);
        self.emit(message);
    }

    pub fn extend_total(&mut self, additional_steps: u32) {
        self.total = self.total.saturating_add(additional_steps);
    }

    pub fn finish(&mut self, message: &str) {
        self.current = self.total;
        self.emit(message);
    }
}

pub fn get_pdf_page_count(input_path: impl AsRef<Path>) -> Result<u32> {
    let document = Document::open(input_path.as_ref())
        .map_err(|e| anyhow::anyhow!("Unable to open PDF: {e}"))?;
    document
        .page_count()
        .map(|c| c as u32)
        .map_err(|e| anyhow::anyhow!("Unable to read page count: {e}"))
}

/// Run the full compression pipeline.
///
/// Opens the input PDF, renders every page as a JPEG at the requested
/// (or auto-selected) DPI, then assembles a new PDF and writes it to disk.
///
/// When a `size_limit_bytes` is set in the config, a binary search
/// determines the highest DPI (and optionally the best JPEG quality) that
/// keeps the output within the budget.
pub fn run_compression(
    task_config: &CompressionTaskConfig,
    progress_callback: Option<ProgressCallback>,
) -> Result<CompressionResult> {
    let original_size = fs::metadata(&task_config.input_path)
        .with_context(|| {
            format!(
                "Failed to read input file size: {}",
                task_config.input_path.display()
            )
        })?
        .len();

    let document = Document::open(task_config.input_path.as_path())
        .with_context(|| format!("Unable to open PDF: {}", task_config.input_path.display()))?;
    let page_count = document
        .page_count()
        .with_context(|| "Unable to read page count".to_string())? as u32;

    let total = estimate_progress_total(task_config, page_count);
    let mut tracker = CompressionProgressTracker::new(total, progress_callback);

    tracker.emit("Starting PDF compression...");
    let (selected_dpi, selected_quality, output_bytes) = if let Some(dpi) = task_config.dpi {
        let bytes = pdf_builder::build_pdf_bytes(
            &document,
            page_count,
            dpi,
            task_config.jpeg_quality,
            task_config.grayscale,
            Some(&mut tracker),
        )?;
        (dpi, task_config.jpeg_quality, bytes)
    } else if let Some(size_limit_bytes) = task_config.size_limit_bytes {
        let fallback_steps = estimate_quality_progress_total(task_config, page_count);
        searcher::compress_to_target_size(
            &document,
            page_count,
            task_config,
            size_limit_bytes,
            fallback_steps,
            &mut tracker,
        )?
    } else {
        anyhow::bail!("Must provide either DPI or target size.")
    };

    fs::write(&task_config.output_path, &output_bytes).with_context(|| {
        format!(
            "Failed to write output PDF: {}",
            task_config.output_path.display()
        )
    })?;
    tracker.finish("Compression complete");

    Ok(CompressionResult {
        input_path: task_config.input_path.clone(),
        output_path: task_config.output_path.clone(),
        selected_dpi,
        selected_quality,
        size_limit_bytes: task_config.size_limit_bytes,
        original_size,
        output_size: output_bytes.len() as u64,
    })
}

pub fn estimate_binary_search_attempts(min: u32, max: u32) -> u32 {
    if min >= max {
        return 1;
    }
    let range = max - min + 1;
    (range - 1).ilog2() + 1
}

fn estimate_progress_total(task_config: &CompressionTaskConfig, page_count: u32) -> u32 {
    if task_config.dpi.is_some() {
        page_count
    } else {
        page_count.saturating_mul(estimate_binary_search_attempts(
            task_config.min_dpi,
            task_config.max_dpi,
        ))
    }
}

fn estimate_quality_progress_total(task_config: &CompressionTaskConfig, page_count: u32) -> u32 {
    let (min_quality, max_quality) = searcher::quality_fallback_range(task_config.jpeg_quality);
    page_count.saturating_mul(estimate_binary_search_attempts(
        min_quality as u32,
        max_quality as u32,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    #[test]
    fn estimates_binary_search_attempts_for_inclusive_ranges() {
        assert_eq!(estimate_binary_search_attempts(72, 300), 8);
        assert_eq!(estimate_binary_search_attempts(40, 75), 6);
        assert_eq!(estimate_binary_search_attempts(10, 10), 1);
    }

    #[test]
    fn progress_tracker_emits_advances_clamps_and_finishes() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let ec = events.clone();
        let callback = move |current, total, msg: ProgressMsg| {
            if let ProgressMsg::Label(s) = msg {
                ec.lock().unwrap().push((current, total, s.to_string()));
            }
        };
        let mut tracker = CompressionProgressTracker::new(3, Some(Box::new(callback)));

        tracker.emit("start");
        tracker.advance("first");
        tracker.advance("second");
        tracker.advance("third");
        tracker.advance("too far");
        tracker.finish("done");
        drop(tracker);

        let events = Arc::into_inner(events).unwrap().into_inner().unwrap();
        assert_eq!(
            events,
            vec![
                (0, 3, "start".to_string()),
                (1, 3, "first".to_string()),
                (2, 3, "second".to_string()),
                (3, 3, "third".to_string()),
                (3, 3, "too far".to_string()),
                (3, 3, "done".to_string()),
            ]
        );
    }

    #[test]
    fn progress_tracker_extends_total_before_additional_work() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let ec = events.clone();
        let callback = move |current, total, msg: ProgressMsg| {
            if let ProgressMsg::Label(s) = msg {
                ec.lock().unwrap().push((current, total, s.to_string()));
            }
        };
        let mut tracker = CompressionProgressTracker::new(2, Some(Box::new(callback)));

        tracker.advance("first");
        tracker.advance("second");
        tracker.extend_total(3);
        tracker.emit("fallback");
        tracker.advance("fallback");
        drop(tracker);

        let events = Arc::into_inner(events).unwrap().into_inner().unwrap();
        assert_eq!(
            events,
            vec![
                (1, 2, "first".to_string()),
                (2, 2, "second".to_string()),
                (2, 5, "fallback".to_string()),
                (3, 5, "fallback".to_string()),
            ]
        );
    }

    #[test]
    fn target_size_progress_total_counts_only_guaranteed_dpi_phase() {
        let task_config = CompressionTaskConfig {
            input_path: Path::new("/tmp/input.pdf").to_path_buf(),
            output_path: Path::new("/tmp/output.pdf").to_path_buf(),
            dpi: None,
            size_limit_bytes: Some(1024),
            jpeg_quality: 75,
            grayscale: false,
            min_dpi: 72,
            max_dpi: 300,
        };

        assert_eq!(estimate_progress_total(&task_config, 2), 16);
    }
}

use crate::compress::pdf_builder;
use crate::compress::CompressionProgressTracker;
use crate::config::CompressionTaskConfig;
use anyhow::Result;
use mupdf::Document;

fn check_target_size_reached(size_limit_bytes: u64, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() {
        anyhow::bail!("Unable to reach target size. No valid PDF output was generated.");
    }

    if bytes.len() as u64 <= size_limit_bytes {
        Ok(())
    } else {
        anyhow::bail!(
            "Unable to reach target size. Even at minimum DPI and JPEG quality the PDF is {:.2} MB, exceeding the {:.2} MB target.",
            bytes.len() as f64 / 1024.0 / 1024.0,
            size_limit_bytes as f64 / 1024.0 / 1024.0
        )
    }
}

pub(crate) fn quality_fallback_range(initial_quality: u8) -> (u8, u8) {
    let min_quality = if initial_quality < crate::config::DEFAULT_MIN_QUALITY {
        1
    } else {
        crate::config::DEFAULT_MIN_QUALITY
    };
    (min_quality, initial_quality)
}

pub(crate) fn compress_to_target_size(
    document: &Document,
    page_count: u32,
    task_config: &CompressionTaskConfig,
    size_limit_bytes: u64,
    fallback_steps: u32,
    tracker: &mut CompressionProgressTracker,
) -> Result<(u32, u8, Vec<u8>)> {
    let (dpi, dpi_bytes) =
        search_best_dpi(document, page_count, task_config, size_limit_bytes, tracker)?;
    if dpi_bytes.len() as u64 <= size_limit_bytes {
        return Ok((dpi, task_config.jpeg_quality, dpi_bytes));
    }

    tracker.extend_total(fallback_steps);
    tracker.emit("Trying lower JPEG quality...");

    let (quality, quality_bytes) = search_best_quality(
        document,
        page_count,
        task_config,
        task_config.min_dpi,
        size_limit_bytes,
        tracker,
    )?;
    check_target_size_reached(size_limit_bytes, &quality_bytes)?;

    Ok((task_config.min_dpi, quality, quality_bytes))
}

fn search_best_dpi(
    document: &Document,
    page_count: u32,
    task_config: &CompressionTaskConfig,
    size_limit_bytes: u64,
    tracker: &mut CompressionProgressTracker,
) -> Result<(u32, Vec<u8>)> {
    let mut low = task_config.min_dpi;
    let mut high = task_config.max_dpi;
    let mut best_dpi = task_config.min_dpi;
    let mut best_bytes: Option<Vec<u8>> = None;

    while low <= high {
        let dpi = low + (high - low) / 2;
        tracker.emit(&format!("Trying DPI {}...", dpi));
        let bytes = pdf_builder::build_pdf_bytes(
            document,
            page_count,
            dpi,
            task_config.jpeg_quality,
            task_config.grayscale,
            Some(tracker),
        )?;

        let bytes_len = bytes.len() as u64;
        let size_mb = bytes_len as f64 / (1024.0 * 1024.0);
        let target_mb = size_limit_bytes as f64 / (1024.0 * 1024.0);
        let pct = bytes_len as f64 / size_limit_bytes as f64 * 100.0;

        if bytes_len <= size_limit_bytes {
            let msg = format!(
                "DPI {} → {:.2} MB (✓ within {:.2} MB)",
                dpi, size_mb, target_mb
            );
            tracker.emit(&msg);
            tracker.log(&msg);
            best_dpi = dpi;
            best_bytes = Some(bytes);
            low = dpi.saturating_add(1);
        } else {
            let msg = format!(
                "DPI {} → {:.2} MB ({:+.1}% over {:.2} MB) — over target",
                dpi,
                size_mb,
                pct - 100.0,
                target_mb
            );
            tracker.emit(&msg);
            tracker.log(&msg);
            high = dpi - 1;
        }
    }

    Ok((best_dpi, best_bytes.unwrap_or_default()))
}

fn search_best_quality(
    document: &Document,
    page_count: u32,
    task_config: &CompressionTaskConfig,
    dpi: u32,
    size_limit_bytes: u64,
    tracker: &mut CompressionProgressTracker,
) -> Result<(u8, Vec<u8>)> {
    let (min_quality, max_quality) = quality_fallback_range(task_config.jpeg_quality);
    let mut low = min_quality as u32;
    let mut high = max_quality as u32;
    let mut best_quality = min_quality;
    let mut best_bytes: Option<Vec<u8>> = None;

    while low <= high {
        let quality = (low + (high - low) / 2) as u8;
        tracker.emit(&format!("Trying JPEG quality {}...", quality));
        let bytes = pdf_builder::build_pdf_bytes(
            document,
            page_count,
            dpi,
            quality,
            task_config.grayscale,
            Some(tracker),
        )?;

        let bytes_len = bytes.len() as u64;
        let size_mb = bytes_len as f64 / (1024.0 * 1024.0);
        let target_mb = size_limit_bytes as f64 / (1024.0 * 1024.0);
        let pct = bytes_len as f64 / size_limit_bytes as f64 * 100.0;

        if bytes_len <= size_limit_bytes {
            let msg = format!(
                "Quality {} → {:.2} MB (✓ within {:.2} MB)",
                quality, size_mb, target_mb
            );
            tracker.emit(&msg);
            tracker.log(&msg);
            best_quality = quality;
            best_bytes = Some(bytes);
            low = quality as u32 + 1;
        } else {
            let msg = format!(
                "Quality {} → {:.2} MB ({:+.1}% over {:.2} MB) — over target",
                quality,
                size_mb,
                pct - 100.0,
                target_mb
            );
            tracker.emit(&msg);
            tracker.log(&msg);
            high = quality as u32 - 1;
        }
    }

    Ok((best_quality, best_bytes.unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_target_size_when_best_trial_is_still_too_large() {
        let error = check_target_size_reached(100, &[0; 101])
            .unwrap_err()
            .to_string();

        assert!(error.contains("Unable to reach target size"));
    }

    #[test]
    fn rejects_target_size_when_no_pdf_bytes_were_generated() {
        let error = check_target_size_reached(100, &[]).unwrap_err().to_string();

        assert!(error.contains("Unable to reach target size"));
    }

    #[test]
    fn quality_fallback_range_remains_valid_below_default_minimum() {
        assert_eq!(quality_fallback_range(30), (1, 30));
        assert_eq!(quality_fallback_range(40), (40, 40));
        assert_eq!(quality_fallback_range(75), (40, 75));
    }
}

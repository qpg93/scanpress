use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use scanpress::{
    format_result_summary, prepare_compression_task, run_compression, ProgressCallback,
    ProgressMsg, DEFAULT_MAX_DPI, DEFAULT_MIN_DPI,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Parser)]
#[command(name = "scanpress", version = "0.1.0 (Qing Peng)")]
#[command(about = "Compress scanned PDFs — reduce file size while preserving readability.")]
struct Args {
    /// Input PDF file(s); pass multiple files for batch processing
    #[arg(required_unless_present = "dir")]
    pdf: Vec<PathBuf>,

    /// Directory containing PDF files to process
    #[arg(long, conflicts_with = "pdf")]
    dir: Option<PathBuf>,

    /// Recursively scan --dir for PDFs in subdirectories
    #[arg(long, requires = "dir")]
    recursive: bool,

    /// DPI for re-rendering pages, e.g. 150, 200, 300
    #[arg(long)]
    dpi: Option<u32>,

    /// Target output size, e.g. 5MB, 800KB; the tool auto-selects a suitable DPI
    #[arg(long)]
    size: Option<String>,

    /// JPEG quality, range 1–100, default 75
    #[arg(long, default_value_t = 75)]
    quality: u8,

    /// Convert to grayscale (usually yields smaller files)
    #[arg(long)]
    gray: bool,

    /// Output PDF file path; auto-generated if omitted.
    /// In batch mode, this is treated as an output directory.
    #[arg(short, long)]
    output: Option<String>,

    /// Minimum DPI to consider during auto-compression
    #[arg(long, default_value_t = DEFAULT_MIN_DPI)]
    min_dpi: u32,

    /// Maximum DPI to consider during auto-compression
    #[arg(long, default_value_t = DEFAULT_MAX_DPI)]
    max_dpi: u32,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let files: Vec<PathBuf> = if let Some(dir) = &args.dir {
        let dir = resolve_dir_path(dir)?;
        collect_pdfs(&dir, args.recursive)?
    } else {
        args.pdf.clone()
    };

    let batch_mode = files.len() > 1 || args.dir.is_some();

    if batch_mode {
        run_batch(&files, &args)
    } else {
        run_single(&files[0], &args)
    }
}

fn run_single(file: &Path, args: &Args) -> anyhow::Result<()> {
    let task_config = prepare_compression_task(
        file,
        args.dpi,
        args.size.as_deref(),
        args.quality,
        args.gray,
        args.output.as_deref(),
        args.min_dpi,
        args.max_dpi,
    )?;

    let bar = Arc::new(ProgressBar::new(0));
    bar.set_style(
        ProgressStyle::with_template("[{bar:40}] {percent:>3}% | {elapsed_precise} | {msg}")
            .unwrap(),
    );
    bar.enable_steady_tick(Duration::from_millis(100));

    let bar_clone = bar.clone();
    let progress: ProgressCallback = Box::new(move |current, total, msg| {
        bar_clone.set_length(total as u64);
        bar_clone.set_position(current as u64);
        match msg {
            ProgressMsg::Label(s) => bar_clone.set_message(s.to_string()),
            ProgressMsg::Line(s) => bar_clone.suspend(|| eprintln!("{}", s)),
        }
    });

    let result = run_compression(&task_config, Some(progress))?;
    bar.finish_and_clear();
    println!("{}", format_result_summary(&result));
    Ok(())
}

fn run_batch(files: &[PathBuf], args: &Args) -> anyhow::Result<()> {
    let out_dir = resolve_output_dir(args.output.as_deref());
    let mut success = 0u32;
    let mut failed = 0u32;

    for (i, file) in files.iter().enumerate() {
        let mut config = match prepare_compression_task(
            file,
            args.dpi,
            args.size.as_deref(),
            args.quality,
            args.gray,
            None,
            args.min_dpi,
            args.max_dpi,
        ) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "[{}/{}] {} — Error: {}",
                    i + 1,
                    files.len(),
                    file.display(),
                    e
                );
                failed += 1;
                continue;
            }
        };

        if let Some(out_dir) = &out_dir {
            let filename = config
                .output_path
                .file_name()
                .expect("output path should have a filename");
            config.output_path = out_dir.join(filename);
        }

        let bar = Arc::new(ProgressBar::new(0));
        bar.set_style(
            ProgressStyle::with_template("[{bar:40}] {percent:>3}% | {elapsed_precise} | {msg}")
                .unwrap(),
        );
        bar.enable_steady_tick(Duration::from_millis(100));
        bar.set_message(format!(
            "[{}/{}] {}",
            i + 1,
            files.len(),
            file.file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default()
        ));

        let bar_clone = bar.clone();
        let progress: ProgressCallback = Box::new(move |current, total, msg| {
            bar_clone.set_length(total as u64);
            bar_clone.set_position(current as u64);
            match msg {
                ProgressMsg::Label(s) => {
                    let current = bar_clone.message();
                    let base = current.split(" | ").next().unwrap_or(&current).to_string();
                    bar_clone.set_message(format!("{} | {}", base, s));
                }
                ProgressMsg::Line(s) => bar_clone.suspend(|| eprintln!("{}", s)),
            }
        });

        match run_compression(&config, Some(progress)) {
            Ok(result) => {
                bar.finish_and_clear();
                println!("{}", format_result_summary(&result));
                println!();
                success += 1;
            }
            Err(e) => {
                bar.finish_and_clear();
                eprintln!("  Error: {}", e);
                failed += 1;
            }
        }
    }

    println!(
        "Batch complete: {} succeeded, {} failed ({} total)",
        success,
        failed,
        files.len()
    );
    Ok(())
}

fn resolve_dir_path(dir: &Path) -> anyhow::Result<PathBuf> {
    let path = if dir.is_relative() {
        std::env::current_dir()?.join(dir)
    } else {
        dir.to_path_buf()
    };
    if !path.is_dir() {
        anyhow::bail!("Directory not found: {}", path.display());
    }
    Ok(path)
}

fn resolve_output_dir(output: Option<&str>) -> Option<PathBuf> {
    let s = output?;
    let path = if s == "~" {
        PathBuf::from(std::env::var("HOME").unwrap_or_default())
    } else if let Some(rest) = s.strip_prefix("~/") {
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(rest)
    } else {
        PathBuf::from(s)
    };
    let path = if path.is_relative() {
        std::env::current_dir().ok().map(|cwd| cwd.join(path))
    } else {
        Some(path)
    }?;
    let _ = std::fs::create_dir_all(&path);
    Some(path)
}

fn collect_pdfs(dir: &Path, recursive: bool) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_pdfs_rec(dir, recursive, &mut files)?;
    files.sort();
    if files.is_empty() {
        anyhow::bail!("No PDF files found in {}", dir.display());
    }
    Ok(files)
}

fn collect_pdfs_rec(dir: &Path, recursive: bool, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                collect_pdfs_rec(&path, recursive, files)?;
            }
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
        {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_pdfs_from_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.pdf"), b"%PDF").unwrap();
        std::fs::write(dir.path().join("b.pdf"), b"%PDF").unwrap();
        std::fs::write(dir.path().join("note.txt"), b"text").unwrap();

        let files = collect_pdfs(dir.path(), false).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn collects_recursive_pdfs() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(dir.path().join("a.pdf"), b"%PDF").unwrap();
        std::fs::write(sub.join("b.pdf"), b"%PDF").unwrap();

        let files = collect_pdfs(dir.path(), true).unwrap();
        assert_eq!(files.len(), 2);

        let files_top = collect_pdfs(dir.path(), false).unwrap();
        assert_eq!(files_top.len(), 1);
    }

    #[test]
    fn rejects_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let err = collect_pdfs(dir.path(), false).unwrap_err().to_string();
        assert!(err.contains("No PDF files found"));
    }

    #[test]
    fn resolves_home_directory() {
        let home = std::env::var("HOME").unwrap();
        let result = resolve_output_dir(Some("~"));
        assert_eq!(result, Some(PathBuf::from(&home)));
    }

    #[test]
    fn resolves_home_subdirectory() {
        let home = std::env::var("HOME").unwrap();
        let result = resolve_output_dir(Some("~/documents"));
        assert_eq!(result, Some(PathBuf::from(&home).join("documents")));
    }

    #[test]
    fn resolves_absolute_directory() {
        let result = resolve_output_dir(Some("/tmp/scanpress-out"));
        assert_eq!(result, Some(PathBuf::from("/tmp/scanpress-out")));
    }
}

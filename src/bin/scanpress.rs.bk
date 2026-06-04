use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use scanpress::{
    format_result_summary, prepare_compression_task, run_compression, ProgressCallback,
    ProgressMsg, DEFAULT_MAX_DPI, DEFAULT_MIN_DPI,
};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Parser)]
#[command(name = "scanpress", version = "0.1.0 (Qing Peng)")]
#[command(about = "Compress scanned PDFs — reduce file size while preserving readability.")]
struct Args {
    /// Input PDF file path
    pdf: PathBuf,

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

    /// Output PDF file path; auto-generated if omitted
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
    let task_config = prepare_compression_task(
        &args.pdf,
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
    bar.enable_steady_tick(std::time::Duration::from_millis(100));

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

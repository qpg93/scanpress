use scanpress::{prepare_compression_task, run_compression, CompressionResult};
use std::path::PathBuf;

fn test_pdf(name: &str) -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    assert!(p.exists(), "test PDF not found: {}", p.display());
    p
}

struct TestCase {
    input: &'static str,
    dpi: Option<u32>,
    size: Option<&'static str>,
    quality: u8,
    grayscale: bool,
    min_dpi: u32,
    max_dpi: u32,
    expect_smaller: bool,
    expect_dpi: Option<u32>,
    expect_quality: Option<u8>,
}

fn run_case(case: TestCase) -> (CompressionResult, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("output.pdf");
    let task = prepare_compression_task(
        test_pdf(case.input),
        case.dpi,
        case.size,
        case.quality,
        case.grayscale,
        Some(output.to_str().unwrap()),
        case.min_dpi,
        case.max_dpi,
    )
    .unwrap();
    let result = run_compression(&task, None).unwrap();

    if case.expect_smaller {
        assert!(
            result.output_size < result.original_size,
            "output ({}) should be smaller than input ({})",
            result.output_size,
            result.original_size
        );
    }
    if let Some(dpi) = case.expect_dpi {
        assert_eq!(result.selected_dpi, dpi, "unexpected selected DPI");
    }
    if let Some(q) = case.expect_quality {
        assert_eq!(result.selected_quality, q, "unexpected selected quality");
    }
    assert!(result.output_path.exists(), "output file should exist");
    assert!(
        result.output_path.metadata().unwrap().len() > 0,
        "output file should not be empty"
    );

    (result, dir)
}

// ── Fixed DPI ──────────────────────────────────────────

#[ignore = "slow — renders entire PDF"]
#[test]
fn hmj_fixed_dpi_200_no_gray() {
    run_case(TestCase {
        input: "PassportAllPages_HMJ.pdf",
        dpi: Some(200),
        size: None,
        quality: 75,
        grayscale: false,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: Some(200),
        expect_quality: Some(75),
    });
}

#[ignore = "slow — renders entire PDF"]
#[test]
fn hmj_fixed_dpi_200_gray() {
    let (r, _) = run_case(TestCase {
        input: "PassportAllPages_HMJ.pdf",
        dpi: Some(200),
        size: None,
        quality: 75,
        grayscale: true,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: Some(200),
        expect_quality: Some(75),
    });
    let gray_size = r.output_size;

    let (r, _) = run_case(TestCase {
        input: "PassportAllPages_HMJ.pdf",
        dpi: Some(200),
        size: None,
        quality: 75,
        grayscale: false,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: Some(200),
        expect_quality: Some(75),
    });
    assert!(
        gray_size < r.output_size,
        "grayscale ({gray_size}) should produce smaller output than RGB ({})",
        r.output_size
    );
}

#[ignore = "slow — renders entire PDF"]
#[test]
fn pq_fixed_dpi_200_no_gray() {
    run_case(TestCase {
        input: "PassportAllPages_PQ.pdf",
        dpi: Some(200),
        size: None,
        quality: 75,
        grayscale: false,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: Some(200),
        expect_quality: Some(75),
    });
}

#[ignore = "slow — renders entire PDF"]
#[test]
fn hmj_fixed_dpi_300_no_gray() {
    let (r200, _) = run_case(TestCase {
        input: "PassportAllPages_HMJ.pdf",
        dpi: Some(200),
        size: None,
        quality: 75,
        grayscale: false,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: Some(200),
        expect_quality: Some(75),
    });

    let (r300, _) = run_case(TestCase {
        input: "PassportAllPages_HMJ.pdf",
        dpi: Some(300),
        size: None,
        quality: 75,
        grayscale: false,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: false,
        expect_dpi: Some(300),
        expect_quality: Some(75),
    });
    assert!(
        r300.output_size > r200.output_size,
        "300 DPI ({}) should produce larger output than 200 DPI ({})",
        r300.output_size,
        r200.output_size
    );
}

// ── Target Size ────────────────────────────────────────

#[ignore = "slow — binary search renders PDF multiple times"]
#[test]
fn hmj_target_5mb_no_gray() {
    let target = 5 * 1024 * 1024;
    let (r, _) = run_case(TestCase {
        input: "PassportAllPages_HMJ.pdf",
        dpi: None,
        size: Some("5MB"),
        quality: 75,
        grayscale: false,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: None,
        expect_quality: None,
    });
    assert!(
        r.output_size <= target + 100_000,
        "output ({}) should not substantially exceed target ({target})",
        r.output_size
    );
}

#[ignore = "slow — binary search renders PDF multiple times"]
#[test]
fn hmj_target_2mb_no_gray() {
    let target = 2 * 1024 * 1024;
    let (r, _) = run_case(TestCase {
        input: "PassportAllPages_HMJ.pdf",
        dpi: None,
        size: Some("2MB"),
        quality: 75,
        grayscale: false,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: None,
        expect_quality: None,
    });
    assert!(
        r.output_size <= target + 100_000,
        "output ({}) should not substantially exceed target ({target})",
        r.output_size
    );
}

#[ignore = "slow — binary search renders PDF multiple times"]
#[test]
fn hmj_target_5mb_gray() {
    let target = 5 * 1024 * 1024;
    let (r, _) = run_case(TestCase {
        input: "PassportAllPages_HMJ.pdf",
        dpi: None,
        size: Some("5MB"),
        quality: 75,
        grayscale: true,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: None,
        expect_quality: None,
    });
    assert!(
        r.output_size <= target + 100_000,
        "output ({}) should not substantially exceed target ({target})",
        r.output_size
    );
}

#[ignore = "slow — binary search renders PDF multiple times"]
#[test]
fn hmj_target_2mb_gray() {
    let target = 2 * 1024 * 1024;
    let (r, _) = run_case(TestCase {
        input: "PassportAllPages_HMJ.pdf",
        dpi: None,
        size: Some("2MB"),
        quality: 75,
        grayscale: true,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: None,
        expect_quality: None,
    });
    assert!(
        r.output_size <= target + 100_000,
        "output ({}) should not substantially exceed target ({target})",
        r.output_size
    );
}

#[ignore = "slow — binary search renders PDF multiple times"]
#[test]
fn pq_target_5mb_no_gray() {
    let target = 5 * 1024 * 1024;
    let (r, _) = run_case(TestCase {
        input: "PassportAllPages_PQ.pdf",
        dpi: None,
        size: Some("5MB"),
        quality: 75,
        grayscale: false,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: None,
        expect_quality: None,
    });
    assert!(
        r.output_size <= target + 100_000,
        "output ({}) should not substantially exceed target ({target})",
        r.output_size
    );
}

#[ignore = "slow — binary search renders PDF multiple times"]
#[test]
fn pq_target_2mb_gray() {
    let target = 2 * 1024 * 1024;
    let (r, _) = run_case(TestCase {
        input: "PassportAllPages_PQ.pdf",
        dpi: None,
        size: Some("2MB"),
        quality: 75,
        grayscale: true,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: None,
        expect_quality: None,
    });
    assert!(
        r.output_size <= target + 100_000,
        "output ({}) should not substantially exceed target ({target})",
        r.output_size
    );
}

// ── Error Cases ────────────────────────────────────────

#[ignore = "slow — binary search renders PDF multiple times"]
#[test]
fn hmj_impossible_small_target() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("output.pdf");
    let task = prepare_compression_task(
        test_pdf("PassportAllPages_HMJ.pdf"),
        None,
        Some("1KB"),
        75,
        false,
        Some(output.to_str().unwrap()),
        72,
        300,
    )
    .unwrap();
    let err = run_compression(&task, None).unwrap_err().to_string();
    assert!(
        err.contains("Unable to reach target size"),
        "error should mention target size: {err}"
    );
}

#[ignore = "slow — binary search renders PDF multiple times"]
#[test]
fn pq_impossible_small_target() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("output.pdf");
    let task = prepare_compression_task(
        test_pdf("PassportAllPages_PQ.pdf"),
        None,
        Some("1KB"),
        75,
        false,
        Some(output.to_str().unwrap()),
        72,
        300,
    )
    .unwrap();
    let err = run_compression(&task, None).unwrap_err().to_string();
    assert!(
        err.contains("Unable to reach target size"),
        "error should mention target size: {err}"
    );
}

// ── Edge Cases ─────────────────────────────────────────

#[ignore = "slow — binary search renders PDF multiple times"]
#[test]
fn hmj_target_5mb_low_quality_10() {
    let target = 5 * 1024 * 1024;
    let (r, _) = run_case(TestCase {
        input: "PassportAllPages_HMJ.pdf",
        dpi: None,
        size: Some("5MB"),
        quality: 10,
        grayscale: false,
        min_dpi: 72,
        max_dpi: 300,
        expect_smaller: true,
        expect_dpi: None,
        expect_quality: None,
    });
    assert!(
        r.output_size <= target + 100_000,
        "output ({}) should not substantially exceed target ({target})",
        r.output_size
    );
}

#[ignore = "slow — binary search renders PDF multiple times"]
#[test]
fn hmj_target_5mb_custom_range_low_max() {
    let target = 5 * 1024 * 1024;
    let (r, _) = run_case(TestCase {
        input: "PassportAllPages_HMJ.pdf",
        dpi: None,
        size: Some("5MB"),
        quality: 75,
        grayscale: false,
        min_dpi: 36,
        max_dpi: 100,
        expect_smaller: true,
        expect_dpi: None,
        expect_quality: None,
    });
    assert!(
        r.output_size <= target + 100_000,
        "output ({}) should not substantially exceed target ({target})",
        r.output_size
    );
    assert!(
        r.selected_dpi <= 100,
        "DPI should not exceed custom max of 100, got {}",
        r.selected_dpi
    );
}

#[ignore = "slow — binary search renders PDF multiple times"]
#[test]
fn pq_target_5mb_custom_range_high_min() {
    let target = 5 * 1024 * 1024;
    let (r, _) = run_case(TestCase {
        input: "PassportAllPages_PQ.pdf",
        dpi: None,
        size: Some("5MB"),
        quality: 75,
        grayscale: false,
        min_dpi: 200,
        max_dpi: 600,
        expect_smaller: true,
        expect_dpi: None,
        expect_quality: None,
    });
    assert!(
        r.output_size <= target + 100_000,
        "output ({}) should not substantially exceed target ({target})",
        r.output_size
    );
    assert!(
        r.selected_dpi >= 200,
        "DPI should not be below custom min of 200, got {}",
        r.selected_dpi
    );
}

// ── Result Summary ─────────────────────────────────────

#[test]
fn result_summary_contains_all_fields() {
    use scanpress::CompressionResult;
    let r = CompressionResult {
        input_path: PathBuf::from("/tmp/in.pdf"),
        output_path: PathBuf::from("/tmp/out.pdf"),
        selected_dpi: 180,
        selected_quality: 75,
        size_limit_bytes: Some(5_242_880),
        original_size: 10_000_000,
        output_size: 4_000_000,
    };
    let s = scanpress::format_result_summary(&r);
    assert!(s.contains("Input file"));
    assert!(s.contains("Output file"));
    assert!(s.contains("Actual DPI"));
    assert!(s.contains("Actual JPEG quality"));
    assert!(s.contains("Target limit"));
    assert!(s.contains("Input size"));
    assert!(s.contains("Output size"));
    assert!(s.contains("Size change"));
}

// ── Output Naming ──────────────────────────────────────

#[test]
fn dpi_mode_default_output_naming() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("scan.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n").unwrap();
    let path = scanpress::build_output_path(&pdf, None, Some(200), None, false).unwrap();
    assert_eq!(
        path.file_name().unwrap().to_str().unwrap(),
        "scan.compressed.200dpi.pdf"
    );
}

#[test]
fn size_mode_default_output_naming() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("scan.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n").unwrap();
    let path =
        scanpress::build_output_path(&pdf, None, None, Some(5 * 1024 * 1024), false).unwrap();
    assert_eq!(
        path.file_name().unwrap().to_str().unwrap(),
        "scan.compressed.target-5.00MB.pdf"
    );
}

#[test]
fn size_bytes_parsing() {
    assert_eq!(
        scanpress::parse_size_to_bytes("5MB").unwrap(),
        5 * 1024 * 1024
    );
    assert_eq!(
        scanpress::parse_size_to_bytes("1.5MB").unwrap(),
        1_572_864
    );
    assert_eq!(
        scanpress::parse_size_to_bytes("800KB").unwrap(),
        819200
    );
    assert_eq!(
        scanpress::parse_size_to_bytes("1048576B").unwrap(),
        1_048_576
    );
}

#[test]
fn jpeg_quality_validation() {
    assert!(scanpress::validate_jpeg_quality(0).is_err());
    assert!(scanpress::validate_jpeg_quality(1).is_ok());
    assert!(scanpress::validate_jpeg_quality(100).is_ok());
    assert!(scanpress::validate_jpeg_quality(101).is_err());
}

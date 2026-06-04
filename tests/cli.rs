use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("scanpress").unwrap()
}

fn fake_pdf(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, b"%PDF-1.4\n").unwrap();
    path
}

#[test]
fn help_includes_all_flags() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--dpi"))
        .stdout(predicate::str::contains("--size"))
        .stdout(predicate::str::contains("--quality"))
        .stdout(predicate::str::contains("--gray"))
        .stdout(predicate::str::contains("--min-dpi"))
        .stdout(predicate::str::contains("--max-dpi"))
        .stdout(predicate::str::contains("--output"));
}

#[test]
fn rejects_neither_dpi_nor_size() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = fake_pdf(dir.path(), "test.pdf");
    cmd()
        .arg(pdf.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("--dpi").or(predicate::str::contains("--size")));
}

#[test]
fn rejects_dpi_and_size_together() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = fake_pdf(dir.path(), "test.pdf");
    cmd()
        .arg(pdf.to_str().unwrap())
        .arg("--dpi")
        .arg("200")
        .arg("--size")
        .arg("5MB")
        .assert()
        .failure()
        .stderr(predicate::str::contains("mutually exclusive"));
}

#[test]
fn rejects_missing_input_file() {
    cmd()
        .arg("/tmp/nonexistent_xyz_test_12345.pdf")
        .arg("--dpi")
        .arg("200")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Input PDF not found"));
}

#[test]
fn rejects_non_pdf_extension() {
    let dir = tempfile::tempdir().unwrap();
    let txt = dir.path().join("test.txt");
    std::fs::write(&txt, "not a pdf").unwrap();
    cmd()
        .arg(txt.to_str().unwrap())
        .arg("--dpi")
        .arg("200")
        .assert()
        .failure()
        .stderr(predicate::str::contains(".pdf"));
}

#[test]
fn rejects_zero_dpi() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = fake_pdf(dir.path(), "test.pdf");
    cmd()
        .arg(pdf.to_str().unwrap())
        .arg("--dpi")
        .arg("0")
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be a positive integer"));
}

#[test]
fn rejects_invalid_size_format() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = fake_pdf(dir.path(), "test.pdf");
    cmd()
        .arg(pdf.to_str().unwrap())
        .arg("--size")
        .arg("abc")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid size format"));
}

#[test]
fn rejects_quality_below_range() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = fake_pdf(dir.path(), "test.pdf");
    cmd()
        .arg(pdf.to_str().unwrap())
        .arg("--dpi")
        .arg("200")
        .arg("--quality")
        .arg("0")
        .assert()
        .failure()
        .stderr(predicate::str::contains("JPEG quality"));
}

#[test]
fn rejects_quality_above_range() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = fake_pdf(dir.path(), "test.pdf");
    cmd()
        .arg(pdf.to_str().unwrap())
        .arg("--dpi")
        .arg("200")
        .arg("--quality")
        .arg("101")
        .assert()
        .failure()
        .stderr(predicate::str::contains("JPEG quality"));
}

#[test]
fn rejects_min_dpi_greater_than_max_dpi() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = fake_pdf(dir.path(), "test.pdf");
    cmd()
        .arg(pdf.to_str().unwrap())
        .arg("--size")
        .arg("5MB")
        .arg("--min-dpi")
        .arg("300")
        .arg("--max-dpi")
        .arg("72")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be greater than"));
}

#[test]
fn rejects_output_same_as_input() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = fake_pdf(dir.path(), "test.pdf");
    cmd()
        .arg(pdf.to_str().unwrap())
        .arg("--dpi")
        .arg("200")
        .arg("--output")
        .arg(pdf.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Output path cannot be the same as input path"));
}

#[test]
fn version_flag_works() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"))
        .stdout(predicate::str::contains("scanpress"));
}

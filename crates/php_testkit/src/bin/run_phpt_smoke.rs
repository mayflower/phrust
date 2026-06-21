//! Selected PHPT smoke runner for Phase 4.

use php_testkit::phpt::{PhptDisposition, PhptExpectation, PhptFile, expectf_matches};
use serde::Serialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug)]
struct Options {
    fixtures_root: PathBuf,
    out_dir: PathBuf,
    rust_vm: Option<PathBuf>,
    extra_phpt: Vec<PathBuf>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PhptStatus {
    Pass,
    Fail,
    Skipped,
    KnownGap,
}

#[derive(Serialize)]
struct PhptSmokeResult {
    path: String,
    test: Option<String>,
    status: PhptStatus,
    message: Option<String>,
    generated_file: Option<String>,
    exit_code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
}

#[derive(Serialize)]
struct PhptSmokeReport {
    fixtures_root: String,
    total: usize,
    pass: usize,
    fail: usize,
    skipped: usize,
    known_gap: usize,
    results: Vec<PhptSmokeResult>,
}

fn main() {
    let code = match run() {
        Ok(report) if report.fail == 0 => 0,
        Ok(report) => {
            eprintln!("PHPT smoke failed for {} fixture(s)", report.fail);
            1
        }
        Err(error) => {
            eprintln!("{error}");
            2
        }
    };
    if code != 0 {
        std::process::exit(code);
    }
}

fn run() -> Result<PhptSmokeReport, String> {
    let options = parse_args(env::args().skip(1))?;
    fs::create_dir_all(&options.out_dir).map_err(|error| {
        format!(
            "failed to create PHPT report directory {}: {error}",
            options.out_dir.display()
        )
    })?;
    let fixtures = discover_phpt_files(&options.fixtures_root, &options.extra_phpt)?;
    let mut results = Vec::new();
    for path in fixtures {
        results.push(run_fixture(&path, &options));
    }
    let report = PhptSmokeReport {
        fixtures_root: options.fixtures_root.display().to_string(),
        total: results.len(),
        pass: results
            .iter()
            .filter(|result| result.status == PhptStatus::Pass)
            .count(),
        fail: results
            .iter()
            .filter(|result| result.status == PhptStatus::Fail)
            .count(),
        skipped: results
            .iter()
            .filter(|result| result.status == PhptStatus::Skipped)
            .count(),
        known_gap: results
            .iter()
            .filter(|result| result.status == PhptStatus::KnownGap)
            .count(),
        results,
    };
    let report_json = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;
    let report_path = options.out_dir.join("phpt-smoke-report.json");
    fs::write(&report_path, report_json)
        .map_err(|error| format!("failed to write PHPT smoke report: {error}"))?;
    println!(
        "[ok] PHPT smoke report: total={} pass={} fail={} skip={} known_gap={} path={}",
        report.total,
        report.pass,
        report.fail,
        report.skipped,
        report.known_gap,
        report_path.display()
    );
    Ok(report)
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut fixtures_root = PathBuf::from("fixtures/phpt_smoke");
    let mut out_dir = PathBuf::from("target/phase4/phpt-smoke");
    let mut rust_vm = env::var_os("PHP_VM_CLI").map(PathBuf::from);
    let mut extra_phpt = Vec::new();
    let args = args.into_iter().collect::<Vec<_>>();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--fixtures" => {
                index += 1;
                fixtures_root = PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--fixtures requires a path".to_string())?,
                );
            }
            "--out" => {
                index += 1;
                out_dir = PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path".to_string())?,
                );
            }
            "--rust-vm" => {
                index += 1;
                rust_vm = Some(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--rust-vm requires a path".to_string())?,
                ));
            }
            "--extra-phpt" => {
                index += 1;
                extra_phpt.push(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--extra-phpt requires a path".to_string())?,
                ));
            }
            "--help" | "-h" => {
                return Err(
                    "Usage: run-phpt-smoke [--fixtures fixtures/phpt_smoke] [--out target/phase4/phpt-smoke] [--rust-vm target/debug/php-vm] [--extra-phpt path]"
                        .to_string(),
                );
            }
            other => return Err(format!("unknown argument `{other}`")),
        }
        index += 1;
    }
    Ok(Options {
        fixtures_root,
        out_dir,
        rust_vm,
        extra_phpt,
    })
}

fn discover_phpt_files(root: &Path, extra: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    collect_phpt_files(root, &mut paths)?;
    for path in extra {
        if path.is_dir() {
            collect_phpt_files(path, &mut paths)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("phpt") {
            paths.push(path.clone());
        } else {
            return Err(format!(
                "extra PHPT path is not a .phpt file: {}",
                path.display()
            ));
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn collect_phpt_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_phpt_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("phpt") {
            out.push(path);
        }
    }
    Ok(())
}

fn run_fixture(path: &Path, options: &Options) -> PhptSmokeResult {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            return base_result(
                path,
                None,
                PhptStatus::Fail,
                Some(format!("failed to read PHPT fixture: {error}")),
            );
        }
    };
    let phpt = PhptFile::parse(&source);
    let test = phpt.section("TEST").map(ToOwned::to_owned);
    match phpt.disposition() {
        PhptDisposition::Run => {}
        PhptDisposition::Skip(reason) => {
            return base_result(path, test, PhptStatus::Skipped, Some(reason));
        }
        PhptDisposition::KnownGap(reason) => {
            return base_result(path, test, PhptStatus::KnownGap, Some(reason));
        }
    }

    let Some(file_body) = phpt.file_body() else {
        return base_result(
            path,
            test,
            PhptStatus::Fail,
            Some("missing required --FILE-- section".to_string()),
        );
    };
    let Some(expectation) = phpt.expectation() else {
        return base_result(
            path,
            test,
            PhptStatus::Fail,
            Some("expected exactly one of --EXPECT-- or --EXPECTF--".to_string()),
        );
    };

    let generated_file = options
        .out_dir
        .join("generated")
        .join(format!("{}.php", sanitized_stem(path)));
    if let Some(parent) = generated_file.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        return base_result(
            path,
            test,
            PhptStatus::Fail,
            Some(format!("failed to create generated fixture dir: {error}")),
        );
    }
    if let Err(error) = fs::write(&generated_file, file_body) {
        return base_result(
            path,
            test,
            PhptStatus::Fail,
            Some(format!("failed to write generated PHP file: {error}")),
        );
    }

    let output = match run_rust_vm(&generated_file, options) {
        Ok(output) => output,
        Err(error) => {
            return PhptSmokeResult {
                path: path.display().to_string(),
                test,
                status: PhptStatus::Fail,
                message: Some(error),
                generated_file: Some(generated_file.display().to_string()),
                exit_code: None,
                stdout: None,
                stderr: None,
            };
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let actual = trim_single_trailing_newline(&stdout);
    let matched = match expectation {
        PhptExpectation::Exact(expected) => actual == expected,
        PhptExpectation::Format(pattern) => expectf_matches(pattern, actual),
    };
    let success = output.status.success() && matched;
    PhptSmokeResult {
        path: path.display().to_string(),
        test,
        status: if success {
            PhptStatus::Pass
        } else {
            PhptStatus::Fail
        },
        message: if success {
            None
        } else if !output.status.success() {
            Some("Rust VM exited with non-zero status".to_string())
        } else {
            Some("PHPT expectation did not match Rust VM stdout".to_string())
        },
        generated_file: Some(generated_file.display().to_string()),
        exit_code: output.status.code(),
        stdout: Some(stdout),
        stderr: Some(stderr),
    }
}

fn base_result(
    path: &Path,
    test: Option<String>,
    status: PhptStatus,
    message: Option<String>,
) -> PhptSmokeResult {
    PhptSmokeResult {
        path: path.display().to_string(),
        test,
        status,
        message,
        generated_file: None,
        exit_code: None,
        stdout: None,
        stderr: None,
    }
}

fn run_rust_vm(path: &Path, options: &Options) -> Result<Output, String> {
    if let Some(rust_vm) = &options.rust_vm
        && rust_vm.is_file()
    {
        return Command::new(rust_vm)
            .arg("run")
            .arg(path)
            .output()
            .map_err(|error| format!("failed to execute {}: {error}", rust_vm.display()));
    }
    let default_vm = Path::new("target/debug/php-vm");
    if default_vm.is_file() {
        return Command::new(default_vm)
            .arg("run")
            .arg(path)
            .output()
            .map_err(|error| format!("failed to execute {}: {error}", default_vm.display()));
    }
    Command::new("cargo")
        .args(["run", "-p", "php_vm_cli", "--", "run"])
        .arg(path)
        .output()
        .map_err(|error| format!("failed to execute cargo fallback: {error}"))
}

fn sanitized_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("fixture")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn trim_single_trailing_newline(value: &str) -> &str {
    value
        .strip_suffix("\r\n")
        .or_else(|| value.strip_suffix('\n'))
        .unwrap_or(value)
}

//! Selected PHPT smoke runner for runtime and runtime-semantics.

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
    allowlist: Option<PathBuf>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PhptStatus {
    Pass,
    Fail,
    Skipped,
    KnownGap,
    ExpectedFail,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AllowlistDisposition {
    Run,
    Skip,
    KnownGap,
    ExpectedFail,
}

#[derive(Clone, Debug)]
struct PhptCase {
    path: PathBuf,
    category: Option<String>,
    disposition: AllowlistDisposition,
    reason: Option<String>,
}

#[derive(Serialize)]
struct PhptSmokeResult {
    path: String,
    category: Option<String>,
    test: Option<String>,
    status: PhptStatus,
    message: Option<String>,
    reason: Option<String>,
    generated_file: Option<String>,
    exit_code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
}

#[derive(Serialize)]
struct PhptSmokeReport {
    fixtures_root: String,
    allowlist: Option<String>,
    total: usize,
    pass: usize,
    fail: usize,
    skipped: usize,
    known_gap: usize,
    expected_fail: usize,
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
    let fixtures = discover_phpt_files(&options)?;
    let mut results = Vec::new();
    for case in fixtures {
        results.push(run_fixture(&case, &options));
    }
    let report = PhptSmokeReport {
        fixtures_root: options.fixtures_root.display().to_string(),
        allowlist: options
            .allowlist
            .as_ref()
            .map(|path| path.display().to_string()),
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
        expected_fail: results
            .iter()
            .filter(|result| result.status == PhptStatus::ExpectedFail)
            .count(),
        results,
    };
    let report_json = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;
    let report_path = options.out_dir.join("phpt-smoke-report.json");
    fs::write(&report_path, report_json)
        .map_err(|error| format!("failed to write PHPT smoke report: {error}"))?;
    println!(
        "[ok] PHPT smoke report: total={} pass={} fail={} skip={} known_gap={} expected_fail={} path={}",
        report.total,
        report.pass,
        report.fail,
        report.skipped,
        report.known_gap,
        report.expected_fail,
        report_path.display()
    );
    Ok(report)
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut fixtures_root = PathBuf::from("fixtures/phpt_smoke");
    let mut out_dir = PathBuf::from("target/runtime/phpt-smoke");
    let mut rust_vm = env::var_os("PHP_VM_CLI").map(PathBuf::from);
    let mut extra_phpt = Vec::new();
    let mut allowlist = None;
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
            "--allowlist" => {
                index += 1;
                allowlist = Some(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--allowlist requires a path".to_string())?,
                ));
            }
            "--help" | "-h" => {
                return Err(
                    "Usage: run-phpt-smoke [--fixtures fixtures/phpt_smoke] [--out target/runtime/phpt-smoke] [--rust-vm target/debug/php-vm] [--extra-phpt path] [--allowlist fixtures/runtime_semantics/phpt_allowlist.toml]"
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
        allowlist,
    })
}

fn discover_phpt_files(options: &Options) -> Result<Vec<PhptCase>, String> {
    if let Some(allowlist) = &options.allowlist {
        return read_allowlist(allowlist);
    }

    let mut paths = Vec::new();
    collect_phpt_files(&options.fixtures_root, &mut paths)?;
    for path in &options.extra_phpt {
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
    Ok(paths
        .into_iter()
        .map(|path| PhptCase {
            path,
            category: None,
            disposition: AllowlistDisposition::Run,
            reason: None,
        })
        .collect())
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

fn read_allowlist(path: &Path) -> Result<Vec<PhptCase>, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read PHPT allowlist {}: {error}", path.display()))?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut entries = Vec::new();
    let mut current = AllowlistRecord::default();
    let mut in_entry = false;

    for (line_index, raw_line) in source.lines().enumerate() {
        let line = raw_line
            .split_once('#')
            .map_or(raw_line, |(head, _)| head)
            .trim();
        if line.is_empty() {
            continue;
        }
        if line == "[[test]]" {
            if in_entry {
                entries.push(current.into_case(base)?);
                current = AllowlistRecord::default();
            }
            in_entry = true;
            continue;
        }
        if !in_entry {
            return Err(format!(
                "{}:{}: expected [[test]] before key",
                path.display(),
                line_index + 1
            ));
        }
        let (key, value) = line.split_once('=').ok_or_else(|| {
            format!(
                "{}:{}: expected `key = \"value\"`",
                path.display(),
                line_index + 1
            )
        })?;
        current.set(
            key.trim(),
            parse_toml_string(value.trim(), path, line_index + 1)?,
        )?;
    }

    if in_entry {
        entries.push(current.into_case(base)?);
    }
    if entries.is_empty() {
        return Err(format!(
            "PHPT allowlist {} has no [[test]] entries",
            path.display()
        ));
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

#[derive(Default)]
struct AllowlistRecord {
    path: Option<String>,
    category: Option<String>,
    disposition: Option<String>,
    reason: Option<String>,
}

impl AllowlistRecord {
    fn set(&mut self, key: &str, value: String) -> Result<(), String> {
        match key {
            "path" => self.path = Some(value),
            "category" => self.category = Some(value),
            "disposition" => self.disposition = Some(value),
            "reason" => self.reason = Some(value),
            other => return Err(format!("unsupported PHPT allowlist key `{other}`")),
        }
        Ok(())
    }

    fn into_case(self, base: &Path) -> Result<PhptCase, String> {
        let path = self
            .path
            .ok_or_else(|| "PHPT allowlist entry is missing `path`".to_string())?;
        let category = self
            .category
            .ok_or_else(|| format!("PHPT allowlist entry `{path}` is missing `category`"))?;
        let disposition = match self.disposition.as_deref().unwrap_or("run") {
            "run" => AllowlistDisposition::Run,
            "skip" => AllowlistDisposition::Skip,
            "known_gap" => AllowlistDisposition::KnownGap,
            "expected_fail" => AllowlistDisposition::ExpectedFail,
            other => {
                return Err(format!(
                    "PHPT allowlist entry `{path}` has unsupported disposition `{other}`"
                ));
            }
        };
        if disposition != AllowlistDisposition::Run
            && self.reason.as_deref().unwrap_or("").is_empty()
        {
            return Err(format!(
                "PHPT allowlist entry `{path}` must include a reason for non-run disposition"
            ));
        }
        let path = PathBuf::from(path);
        Ok(PhptCase {
            path: if path.is_absolute() {
                path
            } else {
                base.join(path)
            },
            category: Some(category),
            disposition,
            reason: self.reason,
        })
    }
}

fn parse_toml_string(value: &str, path: &Path, line: usize) -> Result<String, String> {
    let Some(inner) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(format!(
            "{}:{line}: expected a double-quoted string value",
            path.display()
        ));
    };
    Ok(inner.replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn run_fixture(case: &PhptCase, options: &Options) -> PhptSmokeResult {
    let path = &case.path;
    match case.disposition {
        AllowlistDisposition::Skip => {
            return base_result(
                case,
                None,
                PhptStatus::Skipped,
                Some("allowlist skip".to_string()),
            );
        }
        AllowlistDisposition::KnownGap => {
            return base_result(
                case,
                None,
                PhptStatus::KnownGap,
                Some("allowlist known gap".to_string()),
            );
        }
        AllowlistDisposition::Run | AllowlistDisposition::ExpectedFail => {}
    }

    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            return base_result(
                case,
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
            return base_result(case, test, PhptStatus::Skipped, Some(reason));
        }
        PhptDisposition::KnownGap(reason) => {
            return base_result(case, test, PhptStatus::KnownGap, Some(reason));
        }
    }

    let Some(file_body) = phpt.file_body() else {
        return base_result(
            case,
            test,
            PhptStatus::Fail,
            Some("missing required --FILE-- section".to_string()),
        );
    };
    let Some(expectation) = phpt.expectation() else {
        return base_result(
            case,
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
            case,
            test,
            PhptStatus::Fail,
            Some(format!("failed to create generated fixture dir: {error}")),
        );
    }
    if let Err(error) = fs::write(&generated_file, file_body) {
        return base_result(
            case,
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
                category: case.category.clone(),
                test,
                status: PhptStatus::Fail,
                message: Some(error),
                reason: case.reason.clone(),
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
    let status = match (&case.disposition, success) {
        (AllowlistDisposition::ExpectedFail, false) => PhptStatus::ExpectedFail,
        (AllowlistDisposition::ExpectedFail, true) => PhptStatus::Fail,
        (_, true) => PhptStatus::Pass,
        (_, false) => PhptStatus::Fail,
    };
    PhptSmokeResult {
        path: path.display().to_string(),
        category: case.category.clone(),
        test,
        status,
        message: if case.disposition == AllowlistDisposition::ExpectedFail && !success {
            Some("expected failure matched allowlist".to_string())
        } else if case.disposition == AllowlistDisposition::ExpectedFail && success {
            Some("PHPT unexpectedly passed; remove expected_fail or update coverage".to_string())
        } else if success {
            None
        } else if !output.status.success() {
            Some("Rust VM exited with non-zero status".to_string())
        } else {
            Some("PHPT expectation did not match Rust VM stdout".to_string())
        },
        reason: case.reason.clone(),
        generated_file: Some(generated_file.display().to_string()),
        exit_code: output.status.code(),
        stdout: Some(stdout),
        stderr: Some(stderr),
    }
}

fn base_result(
    case: &PhptCase,
    test: Option<String>,
    status: PhptStatus,
    message: Option<String>,
) -> PhptSmokeResult {
    PhptSmokeResult {
        path: case.path.display().to_string(),
        category: case.category.clone(),
        test,
        status,
        message,
        reason: case.reason.clone(),
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

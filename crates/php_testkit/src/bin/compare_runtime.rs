//! Runtime fixture differential runner for runtime.

use php_testkit::normalize_output::normalize_runtime_stderr;
use php_testkit::runtime_fixture::{
    RuntimeComparisonResult, RuntimeComparisonStatus, RuntimeFixture, RuntimeFixtureExpectation,
    RuntimeFixtureKind, RuntimeSideResult,
};
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
}

#[derive(Serialize)]
struct RuntimeReport {
    fixtures_root: String,
    total: usize,
    pass: usize,
    fail: usize,
    skipped: usize,
    known_gap: usize,
    results: Vec<RuntimeComparisonResult>,
}

fn main() {
    let code = match run() {
        Ok(report) => {
            if report.fail == 0 {
                0
            } else {
                eprintln!("runtime comparison failed for {} fixture(s)", report.fail);
                1
            }
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

fn run() -> Result<RuntimeReport, String> {
    let options = parse_args(env::args().skip(1))?;
    fs::create_dir_all(&options.out_dir).map_err(|error| {
        format!(
            "failed to create report directory {}: {error}",
            options.out_dir.display()
        )
    })?;
    let fixtures = discover_fixtures(&options.fixtures_root)?;
    let mut results = Vec::new();
    for fixture in fixtures {
        let result = compare_fixture(&fixture, &options);
        write_result(&options.out_dir, &result)?;
        results.push(result);
    }
    let report = RuntimeReport {
        fixtures_root: options.fixtures_root.display().to_string(),
        total: results.len(),
        pass: results
            .iter()
            .filter(|result| result.status == RuntimeComparisonStatus::Pass)
            .count(),
        fail: results
            .iter()
            .filter(|result| result.status == RuntimeComparisonStatus::Fail)
            .count(),
        skipped: results
            .iter()
            .filter(|result| result.status == RuntimeComparisonStatus::Skipped)
            .count(),
        known_gap: results
            .iter()
            .filter(|result| result.status == RuntimeComparisonStatus::KnownGap)
            .count(),
        results,
    };
    let report_json = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;
    fs::write(options.out_dir.join("runtime-report.json"), report_json)
        .map_err(|error| format!("failed to write runtime report: {error}"))?;
    println!(
        "[ok] runtime comparison report: total={} pass={} fail={} skip={} known_gap={} path={}",
        report.total,
        report.pass,
        report.fail,
        report.skipped,
        report.known_gap,
        options.out_dir.join("runtime-report.json").display()
    );
    Ok(report)
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut fixtures_root = PathBuf::from("fixtures/runtime");
    let mut out_dir = PathBuf::from("target/runtime/runtime-diff");
    let mut rust_vm = env::var_os("PHP_VM_CLI").map(PathBuf::from);
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
            "--help" | "-h" => {
                return Err(
                    "Usage: compare-runtime [--fixtures fixtures/runtime] [--out target/runtime/runtime-diff] [--rust-vm target/debug/php-vm]"
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
    })
}

fn discover_fixtures(root: &Path) -> Result<Vec<RuntimeFixture>, String> {
    let mut paths = Vec::new();
    collect_php_files(root, &mut paths)?;
    paths.sort();
    Ok(paths
        .into_iter()
        .map(RuntimeFixture::new)
        .map(|mut fixture| {
            if fixture
                .path
                .to_string_lossy()
                .contains("/valid/includes/lib/")
            {
                fixture.expect = RuntimeFixtureExpectation::Skip;
            }
            fixture
        })
        .collect())
}

fn collect_php_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_php_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("php") {
            out.push(path);
        }
    }
    Ok(())
}

fn compare_fixture(fixture: &RuntimeFixture, options: &Options) -> RuntimeComparisonResult {
    if fixture.expect == RuntimeFixtureExpectation::Skip {
        return result(
            fixture,
            None,
            None,
            RuntimeComparisonStatus::Skipped,
            Vec::new(),
            fixture.known_gap_id.clone(),
            Some("fixture metadata requested skip".to_string()),
        );
    }

    let rust = run_rust_vm(fixture, options);
    let rust_side = rust
        .as_ref()
        .ok()
        .map(|output| side_result(output, &fixture.path));
    let diagnostic_ids = rust
        .as_ref()
        .ok()
        .map(|output| extract_diagnostic_ids(&String::from_utf8_lossy(&output.stderr)))
        .unwrap_or_default();

    if fixture.expect == RuntimeFixtureExpectation::KnownGap
        || fixture.kind == RuntimeFixtureKind::KnownGap
    {
        let known_gap_id = fixture
            .known_gap_id
            .clone()
            .or_else(|| diagnostic_ids.first().cloned());
        return result(
            fixture,
            run_reference_side(fixture).ok().flatten(),
            rust_side,
            RuntimeComparisonStatus::KnownGap,
            diagnostic_ids,
            known_gap_id,
            rust.err(),
        );
    }

    if fixture.expect == RuntimeFixtureExpectation::Fail
        || fixture.kind == RuntimeFixtureKind::Invalid
    {
        let status = match rust.as_ref() {
            Ok(output) if !output.status.success() => RuntimeComparisonStatus::Pass,
            Ok(_) => RuntimeComparisonStatus::Fail,
            Err(_) => RuntimeComparisonStatus::Fail,
        };
        let message = if status == RuntimeComparisonStatus::Fail {
            Some("fixture was expected to fail on the Rust runtime".to_string())
        } else {
            None
        };
        return result(
            fixture,
            run_reference_side(fixture).ok().flatten(),
            rust_side,
            status,
            diagnostic_ids,
            fixture.known_gap_id.clone(),
            message.or_else(|| rust.err()),
        );
    }

    let reference = match run_reference_side(fixture) {
        Ok(reference) => reference,
        Err(message) => {
            return result(
                fixture,
                None,
                rust_side,
                RuntimeComparisonStatus::Fail,
                diagnostic_ids,
                None,
                Some(message),
            );
        }
    };
    let Some(reference) = reference else {
        let status = if fixture.php_ref_required {
            RuntimeComparisonStatus::Fail
        } else {
            RuntimeComparisonStatus::Skipped
        };
        return result(
            fixture,
            None,
            rust_side,
            status,
            diagnostic_ids,
            None,
            Some("REFERENCE_PHP is not set".to_string()),
        );
    };
    let status = match (&reference, &rust_side) {
        (reference, Some(rust)) if same_side(reference, rust) => RuntimeComparisonStatus::Pass,
        _ => RuntimeComparisonStatus::Fail,
    };
    let message = if status == RuntimeComparisonStatus::Fail {
        Some(diff_message(&reference, rust_side.as_ref()))
    } else {
        None
    };
    result(
        fixture,
        Some(reference),
        rust_side,
        status,
        diagnostic_ids,
        None,
        message.or_else(|| rust.err()),
    )
}

fn run_reference_side(fixture: &RuntimeFixture) -> Result<Option<RuntimeSideResult>, String> {
    let Some(php_bin) = env::var_os("REFERENCE_PHP").map(PathBuf::from) else {
        return Ok(None);
    };
    if !php_bin.is_file() {
        return Err(format!(
            "REFERENCE_PHP is not a file: {}",
            php_bin.display()
        ));
    }
    let output = Command::new(&php_bin)
        .arg(&fixture.path)
        .args(&fixture.args)
        .env_clear()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("NO_COLOR", "1")
        .env("PHP_INI_SCAN_DIR", "")
        .output()
        .map_err(|error| format!("failed to execute {}: {error}", php_bin.display()))?;
    Ok(Some(side_result_with_php(
        &output,
        &fixture.path,
        Some(&php_bin),
    )))
}

fn run_rust_vm(fixture: &RuntimeFixture, options: &Options) -> Result<Output, String> {
    if let Some(path) = &options.rust_vm
        && path.is_file()
    {
        let mut command = Command::new(path);
        command.arg("run").arg(&fixture.path);
        if !fixture.args.is_empty() {
            command.arg("--").args(&fixture.args);
        }
        return command
            .output()
            .map_err(|error| format!("failed to execute {}: {error}", path.display()));
    }
    let default_vm = Path::new("target/debug/php-vm");
    if default_vm.is_file() {
        let mut command = Command::new(default_vm);
        command.arg("run").arg(&fixture.path);
        if !fixture.args.is_empty() {
            command.arg("--").args(&fixture.args);
        }
        return command
            .output()
            .map_err(|error| format!("failed to execute {}: {error}", default_vm.display()));
    }
    let mut command = Command::new("cargo");
    command.args(["run", "-p", "php_vm_cli", "--", "run"]);
    command.arg(&fixture.path);
    if !fixture.args.is_empty() {
        command.arg("--").args(&fixture.args);
    }
    command
        .output()
        .map_err(|error| format!("failed to execute cargo run -p php_vm_cli: {error}"))
}

fn side_result(output: &Output, file: &Path) -> RuntimeSideResult {
    side_result_with_php(output, file, None)
}

fn side_result_with_php(output: &Output, file: &Path, php_bin: Option<&Path>) -> RuntimeSideResult {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    RuntimeSideResult {
        exit_code: output.status.code(),
        stdout,
        stderr_normalized: normalize_runtime_stderr(&stderr, file, php_bin),
    }
}

fn same_side(reference: &RuntimeSideResult, rust: &RuntimeSideResult) -> bool {
    reference.exit_code == rust.exit_code
        && reference.stdout == rust.stdout
        && reference.stderr_normalized == rust.stderr_normalized
}

fn diff_message(reference: &RuntimeSideResult, rust: Option<&RuntimeSideResult>) -> String {
    let Some(rust) = rust else {
        return "Rust runtime did not produce output".to_string();
    };
    let mut parts = Vec::new();
    if reference.exit_code != rust.exit_code {
        parts.push(format!(
            "exit_code reference={:?} rust={:?}",
            reference.exit_code, rust.exit_code
        ));
    }
    if reference.stdout != rust.stdout {
        parts.push(format!(
            "stdout reference={:?} rust={:?}",
            reference.stdout, rust.stdout
        ));
    }
    if reference.stderr_normalized != rust.stderr_normalized {
        parts.push(format!(
            "stderr reference={:?} rust={:?}",
            reference.stderr_normalized, rust.stderr_normalized
        ));
    }
    parts.join("; ")
}

fn result(
    fixture: &RuntimeFixture,
    reference: Option<RuntimeSideResult>,
    rust: Option<RuntimeSideResult>,
    status: RuntimeComparisonStatus,
    diagnostic_ids: Vec<String>,
    known_gap_id: Option<String>,
    message: Option<String>,
) -> RuntimeComparisonResult {
    RuntimeComparisonResult {
        file: fixture.display_path(),
        reference,
        rust,
        status,
        diagnostic_ids,
        known_gap_id,
        message,
    }
}

fn write_result(out_dir: &Path, result: &RuntimeComparisonResult) -> Result<(), String> {
    let filename = result
        .file
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    let json = result.to_pretty_json().map_err(|error| error.to_string())?;
    fs::write(out_dir.join(format!("{filename}.json")), json)
        .map_err(|error| format!("failed to write fixture result: {error}"))
}

fn extract_diagnostic_ids(stderr: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = stderr;
    while let Some(index) = rest.find("\"id\":\"") {
        let after = &rest[index + "\"id\":\"".len()..];
        let Some(end) = after.find('"') else {
            break;
        };
        let id = after[..end].to_string();
        if !ids.contains(&id) {
            ids.push(id);
        }
        rest = &after[end + 1..];
    }
    for token in stderr.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')) {
        if token.starts_with("E_") && !ids.iter().any(|id| id == token) {
            ids.push(token.to_string());
        }
    }
    ids
}

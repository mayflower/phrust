use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use php_phpt_tools::expect::{ExpectationKind, match_expectation};
use php_phpt_tools::phpt::{PhptSection, parse_phpt};

const DEFAULT_MANIFEST: &str = "tests/phpt/manifests/php-src-hashes.jsonl";
const DEFAULT_SYMBOLS: &str = "tests/phpt/manifests/php-src-symbols.jsonl";
const DEFAULT_PHPT_CORPUS: &str = "tests/phpt/manifests/phpt-corpus.jsonl";
const DEFAULT_PHPT_REPORT: &str = "docs/phpt/reports/phpt-corpus-summary.md";
const GENERATOR_VERSION: &str = "phpt-generate-v1";

fn main() {
    let code = match run(env::args().skip(1), &mut io::stdout(), &mut io::stderr()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            2
        }
    };
    if code != 0 {
        std::process::exit(code);
    }
}

fn run<I, W, E>(args: I, stdout: &mut W, stderr: &mut E) -> Result<i32, String>
where
    I: IntoIterator<Item = String>,
    W: Write,
    E: Write,
{
    let args: Vec<String> = args.into_iter().collect();
    let Some(command) = args.first().map(String::as_str) else {
        print_usage(stdout)?;
        return Ok(0);
    };
    match command {
        "source-index" => source_index(&args[1..], stdout),
        "symbol-index" => symbol_index(&args[1..], stdout),
        "lookup-symbol" => lookup_symbol(&args[1..], stdout, stderr),
        "phpt-index" => phpt_index(&args[1..], stdout),
        "run" => run_phpt_manifest(&args[1..], stdout),
        "baseline" => baseline_results(&args[1..], stdout, stderr),
        "generate" => generate_module_tests(&args[1..], stdout),
        "verify-source" => verify_source(&args[1..], stdout, stderr),
        "--help" | "-h" | "help" => {
            print_usage(stdout)?;
            Ok(0)
        }
        _ => Err(format!("unknown php-phpt-tools command `{command}`")),
    }
}

fn source_index<W: Write>(args: &[String], stdout: &mut W) -> Result<i32, String> {
    let options = SourceOptions::parse(args)?;
    let mut entries = collect_manifest_entries(&options.php_src)?;
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    if let Some(parent) = options.manifest.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let mut out = String::new();
    for entry in &entries {
        out.push_str(&entry.to_json_line());
        out.push('\n');
    }
    fs::write(&options.manifest, out)
        .map_err(|error| format!("{}: {error}", options.manifest.display()))?;
    writeln!(
        stdout,
        "[ok] wrote {} entries to {}",
        entries.len(),
        options.manifest.display()
    )
    .map_err(|error| error.to_string())?;
    Ok(0)
}

fn verify_source<W: Write, E: Write>(
    args: &[String],
    stdout: &mut W,
    stderr: &mut E,
) -> Result<i32, String> {
    let options = SourceOptions::parse(args)?;
    if !options.manifest.is_file() {
        return Err(format!(
            "{}: source hash manifest does not exist; run `just phpt-source-index`",
            options.manifest.display()
        ));
    }
    let manifest = fs::read_to_string(&options.manifest)
        .map_err(|error| format!("{}: {error}", options.manifest.display()))?;
    let mut checked = 0usize;
    let mut errors = Vec::new();
    for (line_index, line) in manifest.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let entry = match ManifestEntry::from_json_line(line) {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(format!(
                    "{}:{}: {error}",
                    options.manifest.display(),
                    line_index + 1
                ));
                continue;
            }
        };
        checked += 1;
        let path = options.php_src.join(&entry.path);
        match hash_file(&path) {
            Ok((size, sha256)) => {
                if size != entry.size {
                    errors.push(format!(
                        "{}: size mismatch manifest={} actual={}",
                        entry.path, entry.size, size
                    ));
                }
                if sha256 != entry.sha256 {
                    errors.push(format!("{}: sha256 mismatch", entry.path));
                }
            }
            Err(error) => errors.push(format!("{}: {error}", entry.path)),
        }
    }
    if !errors.is_empty() {
        for error in &errors {
            writeln!(stderr, "{error}").map_err(|io| io.to_string())?;
        }
        return Ok(1);
    }
    writeln!(
        stdout,
        "[ok] verified {checked} php-src manifest entries from {}",
        options.manifest.display()
    )
    .map_err(|error| error.to_string())?;
    Ok(0)
}

fn print_usage<W: Write>(stdout: &mut W) -> Result<(), String> {
    writeln!(
        stdout,
        "usage: php-phpt-tools <source-index|symbol-index|lookup-symbol|phpt-index|run|baseline|generate|verify-source> [options]"
    )
    .map_err(|error| error.to_string())
}

fn phpt_index<W: Write>(args: &[String], stdout: &mut W) -> Result<i32, String> {
    let options = PhptIndexOptions::parse(args)?;
    let mut entries = collect_phpt_entries(&options.php_src)?;
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    if let Some(parent) = options.out.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    if let Some(parent) = options.report.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let mut jsonl = String::new();
    for entry in &entries {
        jsonl.push_str(&entry.to_json_line());
        jsonl.push('\n');
    }
    fs::write(&options.out, jsonl)
        .map_err(|error| format!("{}: {error}", options.out.display()))?;
    fs::write(&options.report, render_phpt_summary(&entries))
        .map_err(|error| format!("{}: {error}", options.report.display()))?;
    writeln!(
        stdout,
        "[ok] indexed {} PHPT files to {} and {}",
        entries.len(),
        options.out.display(),
        options.report.display()
    )
    .map_err(|error| error.to_string())?;
    Ok(0)
}

fn run_phpt_manifest<W: Write>(args: &[String], stdout: &mut W) -> Result<i32, String> {
    let options = RunOptions::parse(args)?;
    if !options.target.is_file() {
        return Err(format!(
            "target PHP is not executable: {}",
            options.target.display()
        ));
    }
    let paths = read_manifest_paths(&options.manifest)?;
    if paths.is_empty() {
        return Err(format!(
            "{}: manifest contains no paths",
            options.manifest.display()
        ));
    }
    fs::create_dir_all(&options.work_dir)
        .map_err(|error| format!("{}: {error}", options.work_dir.display()))?;
    if let Some(parent) = options.out.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    if let Some(parent) = options.summary.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let mut results = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        match run_one_phpt(&options, path, index) {
            Ok(result) => results.push(result),
            Err(error) => results.push(PhptRunResult {
                path: path.to_string(),
                outcome: "BORK".to_string(),
                detail: error,
            }),
        }
    }
    let mut jsonl = String::new();
    for result in &results {
        jsonl.push_str(&result.to_json_line());
        jsonl.push('\n');
    }
    fs::write(&options.out, jsonl)
        .map_err(|error| format!("{}: {error}", options.out.display()))?;
    fs::write(&options.summary, render_run_summary(&results))
        .map_err(|error| format!("{}: {error}", options.summary.display()))?;
    let failed = results
        .iter()
        .filter(|result| !matches!(result.outcome.as_str(), "PASS" | "SKIP" | "XFAIL"))
        .count();
    writeln!(
        stdout,
        "[ok] ran {} PHPT tests with {} non-green outcomes; reports: {} {}",
        results.len(),
        failed,
        options.out.display(),
        options.summary.display()
    )
    .map_err(|error| error.to_string())?;
    Ok(if failed == 0 { 0 } else { 1 })
}

fn read_manifest_paths(path: &Path) -> Result<Vec<String>, String> {
    let manifest =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut paths = Vec::new();
    for (index, line) in manifest.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('{') {
            paths.push(extract_json_string(trimmed, "path").map_err(|error| {
                format!(
                    "{}:{}: manifest entry missing path: {error}",
                    path.display(),
                    index + 1
                )
            })?);
        } else {
            paths.push(trimmed.to_string());
        }
    }
    Ok(paths)
}

fn baseline_results<W: Write, E: Write>(
    args: &[String],
    stdout: &mut W,
    stderr: &mut E,
) -> Result<i32, String> {
    let options = BaselineOptions::parse(args)?;
    let results = read_run_results(&options.results)?;
    let corpus = read_corpus_modules(&options.corpus)?;
    let previous_failures = if let Some(previous) = &options.previous_known_failures {
        if previous.is_file() {
            read_known_failures(previous)?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let previous_results = if let Some(previous) = &options.previous_results {
        if previous.is_file() {
            read_run_results(previous)?
                .into_iter()
                .filter(|result| !matches!(result.outcome.as_str(), "PASS" | "SKIP" | "XFAIL"))
                .map(|result| (result.path.clone(), result))
                .collect::<BTreeMap<_, _>>()
        } else {
            BTreeMap::new()
        }
    } else {
        BTreeMap::new()
    };
    let current_results = results
        .iter()
        .filter(|result| !matches!(result.outcome.as_str(), "PASS" | "SKIP" | "XFAIL"))
        .map(|result| (result.path.clone(), result))
        .collect::<BTreeMap<_, _>>();
    let previous_first_seen = previous_failures
        .iter()
        .map(|failure| {
            (
                (
                    failure.path.clone(),
                    failure.outcome.clone(),
                    failure.failure_fingerprint.clone(),
                ),
                failure.first_seen_timestamp.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let previous_path_outcome_first_seen = previous_failures
        .iter()
        .map(|failure| {
            (
                (failure.path.clone(), failure.outcome.clone()),
                failure.first_seen_timestamp.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut failures = results
        .iter()
        .filter(|result| !matches!(result.outcome.as_str(), "PASS" | "SKIP" | "XFAIL"))
        .map(|result| {
            let module = corpus
                .get(&result.path)
                .cloned()
                .unwrap_or_else(|| module_guess(&result.path));
            let fingerprint = failure_fingerprint(result);
            let first_seen = previous_first_seen
                .get(&(
                    result.path.clone(),
                    result.outcome.clone(),
                    fingerprint.clone(),
                ))
                .cloned()
                .or_else(|| {
                    previous_path_outcome_first_seen
                        .get(&(result.path.clone(), result.outcome.clone()))
                        .filter(|_| {
                            is_related_known_failure_evolution(
                                previous_results.get(&result.path),
                                current_results.get(&result.path).copied(),
                            )
                        })
                        .cloned()
                })
                .unwrap_or_else(|| options.timestamp.clone());
            KnownFailure {
                path: result.path.clone(),
                module_tag: module.clone(),
                outcome: result.outcome.clone(),
                failure_fingerprint: fingerprint,
                primary_missing_feature_guess: missing_feature_guess(result),
                owner_module: module,
                first_seen_timestamp: first_seen,
            }
        })
        .collect::<Vec<_>>();
    failures.sort_by(|left, right| left.path.cmp(&right.path));

    if !previous_failures.is_empty() {
        let mut previous_keys = previous_failures
            .iter()
            .map(|failure| {
                (
                    failure.path.clone(),
                    failure.outcome.clone(),
                    failure.failure_fingerprint.clone(),
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        for result in previous_results.values() {
            previous_keys.insert((
                result.path.clone(),
                result.outcome.clone(),
                failure_fingerprint(result),
            ));
        }
        let regressions = failures
            .iter()
            .filter(|failure| {
                !previous_keys.contains(&(
                    failure.path.clone(),
                    failure.outcome.clone(),
                    failure.failure_fingerprint.clone(),
                )) && !is_related_known_failure_evolution(
                    previous_results.get(&failure.path),
                    current_results.get(&failure.path).copied(),
                )
            })
            .collect::<Vec<_>>();
        if !regressions.is_empty() {
            writeln!(
                stderr,
                "PHPT full regression detected {} new or changed failure fingerprints",
                regressions.len()
            )
            .map_err(|error| error.to_string())?;
            for failure in regressions.iter().take(25) {
                writeln!(
                    stderr,
                    "{} {} {}",
                    failure.path, failure.outcome, failure.failure_fingerprint
                )
                .map_err(|error| error.to_string())?;
            }
            return Ok(1);
        }
    }

    if let Some(parent) = options.known_failures.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    if let Some(parent) = options.report.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let mut jsonl = String::new();
    for failure in &failures {
        jsonl.push_str(&failure.to_json_line());
        jsonl.push('\n');
    }
    fs::write(&options.known_failures, jsonl)
        .map_err(|error| format!("{}: {error}", options.known_failures.display()))?;
    fs::write(
        &options.report,
        render_baseline_report(&results, &failures, &options.timestamp),
    )
    .map_err(|error| format!("{}: {error}", options.report.display()))?;
    writeln!(
        stdout,
        "[ok] wrote {} known failures to {} and report {}",
        failures.len(),
        options.known_failures.display(),
        options.report.display()
    )
    .map_err(|error| error.to_string())?;
    Ok(0)
}

fn generate_module_tests<W: Write>(args: &[String], stdout: &mut W) -> Result<i32, String> {
    let options = GenerateOptions::parse(args)?;
    let corpus = read_phpt_corpus(&options.corpus)?;
    let mut selected = corpus
        .iter()
        .filter(|entry| matches_module_selector(entry, &options.module))
        .cloned()
        .collect::<Vec<_>>();
    selected.sort_by(|left, right| left.path.cmp(&right.path));
    if selected.is_empty() {
        return Err(format!(
            "{}: no PHPT corpus entries match module selector `{}`",
            options.corpus.display(),
            options.module
        ));
    }

    fs::create_dir_all(&options.generated_dir)
        .map_err(|error| format!("{}: {error}", options.generated_dir.display()))?;
    clear_generated_phpts(&options.generated_dir)?;
    if let Some(parent) = options.module_manifest.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    if let Some(parent) = options.generated_manifest.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::create_dir_all(&options.work_dir)
        .map_err(|error| format!("{}: {error}", options.work_dir.display()))?;

    let mut module_manifest = String::new();
    for entry in &selected {
        module_manifest.push_str(&entry.to_json_line());
        module_manifest.push('\n');
    }
    fs::write(&options.module_manifest, module_manifest)
        .map_err(|error| format!("{}: {error}", options.module_manifest.display()))?;

    let reference_options = RunOptions {
        target: options.reference.clone(),
        target_mode: TargetMode::PhpCli,
        manifest: options.module_manifest.clone(),
        php_src: options.php_src.clone(),
        work_dir: options.work_dir.join("reference"),
        out: options.work_dir.join("unused-results.jsonl"),
        summary: options.work_dir.join("unused-summary.md"),
        timeout: options.timeout,
    };

    let mut generated = Vec::new();
    let mut smoke_candidates = selected
        .iter()
        .filter(|entry| is_simple_generation_candidate(entry))
        .cloned()
        .collect::<Vec<_>>();
    smoke_candidates.sort_by_key(|entry| source_len(&options.php_src.join(&entry.path)));
    for entry in smoke_candidates {
        if generated
            .iter()
            .filter(|case: &&GeneratedCase| case.kind == "smoke")
            .count()
            >= options.smoke_count
        {
            break;
        }
        if run_one_phpt(&reference_options, &entry.path, generated.len())?.outcome != "PASS" {
            continue;
        }
        if let Some(case) = build_generated_case(
            &options,
            &reference_options,
            &entry,
            "smoke",
            "smallest reference-passing example",
            None,
            generated.len(),
        )? {
            write_generated_case(&case)?;
            generated.push(case);
        }
    }

    if options.known_failures.is_file() {
        let smoke_originals = generated
            .iter()
            .filter(|case| case.kind == "smoke")
            .map(|case| case.original_path.clone())
            .collect::<BTreeSet<_>>();
        let selected_by_path = selected
            .iter()
            .map(|entry| (entry.path.clone(), entry.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut failure_candidates = read_known_failures(&options.known_failures)?
            .into_iter()
            .filter_map(|failure| selected_by_path.get(&failure.path).cloned())
            .filter(|entry| !smoke_originals.contains(&entry.path))
            .filter(is_simple_generation_candidate)
            .collect::<Vec<_>>();
        failure_candidates.sort_by_key(|entry| source_len(&options.php_src.join(&entry.path)));
        for entry in failure_candidates {
            if generated
                .iter()
                .filter(|case: &&GeneratedCase| case.kind == "regression")
                .count()
                >= options.regression_count
            {
                break;
            }
            if let Some(case) = build_generated_case(
                &options,
                &reference_options,
                &entry,
                "regression",
                "known target failure minimized against reference output",
                Some(ReductionMode::LineRemoval),
                generated.len(),
            )? {
                write_generated_case(&case)?;
                generated.push(case);
            }
        }
    }

    if generated.is_empty() {
        return Err(format!(
            "module selector `{}` produced no generated PHPTs",
            options.module
        ));
    }
    let mut generated_manifest = String::new();
    for case in &generated {
        generated_manifest.push_str(&case.to_json_line());
        generated_manifest.push('\n');
    }
    fs::write(&options.generated_manifest, generated_manifest)
        .map_err(|error| format!("{}: {error}", options.generated_manifest.display()))?;

    writeln!(
        stdout,
        "[ok] wrote {} original entries to {}",
        selected.len(),
        options.module_manifest.display()
    )
    .map_err(|error| error.to_string())?;
    writeln!(
        stdout,
        "[ok] generated {} PHPTs under {} and manifest {}",
        generated.len(),
        options.generated_dir.display(),
        options.generated_manifest.display()
    )
    .map_err(|error| error.to_string())?;
    Ok(0)
}

fn run_one_phpt(
    options: &RunOptions,
    manifest_path: &str,
    index: usize,
) -> Result<PhptRunResult, String> {
    let phpt_path = resolve_phpt_path(&options.php_src, manifest_path);
    let source = fs::read_to_string(&phpt_path)
        .map_err(|error| format!("{}: {error}", phpt_path.display()))?;
    let document = parse_phpt(&source);
    if let Some(diagnostic) = document
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "PHPT_UNSUPPORTED_SECTION")
    {
        return Ok(PhptRunResult {
            path: manifest_path.to_string(),
            outcome: "BORK".to_string(),
            detail: diagnostic.message.clone(),
        });
    }
    let work_dir =
        options
            .work_dir
            .join("target")
            .join(format!("case-{}-{}", std::process::id(), index));
    let _ = fs::remove_dir_all(&work_dir);
    fs::create_dir_all(&work_dir).map_err(|error| format!("{}: {error}", work_dir.display()))?;

    if let Some(skipif) = section(&document.sections, "SKIPIF") {
        let skip_path = work_dir.join("skipif.php");
        fs::write(&skip_path, &skipif.body)
            .map_err(|error| format!("{}: {error}", skip_path.display()))?;
        let skip = run_php(options, &skip_path, &work_dir, &[], &[], &[], None)?;
        if skip.stdout.to_ascii_lowercase().starts_with("skip") {
            run_clean_if_present(options, &document.sections, &work_dir)?;
            return Ok(PhptRunResult {
                path: manifest_path.to_string(),
                outcome: "SKIP".to_string(),
                detail: first_non_empty_line(&skip.stdout),
            });
        }
    }

    let Some(file_body) = file_body(&document.sections, &phpt_path)? else {
        return Ok(PhptRunResult {
            path: manifest_path.to_string(),
            outcome: "BORK".to_string(),
            detail: "missing FILE, FILEEOF, or FILE_EXTERNAL".to_string(),
        });
    };
    let test_path = work_dir.join("test.php");
    fs::write(&test_path, file_body)
        .map_err(|error| format!("{}: {error}", test_path.display()))?;
    let ini = ini_args(&document.sections);
    let env = env_args(&document.sections);
    let args = section(&document.sections, "ARGS")
        .map(|section| split_phpt_args(&section.body))
        .unwrap_or_default();
    let stdin = section(&document.sections, "STDIN").map(|section| section.body.as_str());
    let output = run_php(options, &test_path, &work_dir, &ini, &env, &args, stdin)?;
    run_clean_if_present(options, &document.sections, &work_dir)?;

    if output.status != 0 {
        return Ok(PhptRunResult {
            path: manifest_path.to_string(),
            outcome: "FAIL".to_string(),
            detail: format!(
                "target exited with status {}; stderr={}",
                output.status, output.stderr
            ),
        });
    }
    let Some((kind, expected)) = expectation(&document.sections, &phpt_path)? else {
        return Ok(PhptRunResult {
            path: manifest_path.to_string(),
            outcome: "BORK".to_string(),
            detail: "missing expectation section".to_string(),
        });
    };
    let matched = match_expectation(
        kind,
        &normalize_expected(&expected),
        &normalize_expected(&output.stdout),
    );
    if matched.matched {
        Ok(PhptRunResult {
            path: manifest_path.to_string(),
            outcome: "PASS".to_string(),
            detail: String::new(),
        })
    } else {
        let detail = matched
            .diff
            .map(|diff| {
                format!(
                    "{} first_mismatch={:?} expected=`{}` actual=`{}`",
                    diff.message, diff.first_mismatch, diff.expected_excerpt, diff.actual_excerpt
                )
            })
            .unwrap_or_else(|| "output did not match".to_string());
        Ok(PhptRunResult {
            path: manifest_path.to_string(),
            outcome: "FAIL".to_string(),
            detail,
        })
    }
}

fn symbol_index<W: Write>(args: &[String], stdout: &mut W) -> Result<i32, String> {
    let options = SymbolOptions::parse(args)?;
    let mut entries = collect_symbol_entries(&options.php_src)?;
    entries.sort_by(|left, right| {
        (
            &left.path,
            left.line,
            &left.kind,
            &left.c_name,
            &left.php_name,
        )
            .cmp(&(
                &right.path,
                right.line,
                &right.kind,
                &right.c_name,
                &right.php_name,
            ))
    });
    if let Some(parent) = options.symbols.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let mut out = String::new();
    for entry in &entries {
        out.push_str(&entry.to_json_line());
        out.push('\n');
    }
    fs::write(&options.symbols, out)
        .map_err(|error| format!("{}: {error}", options.symbols.display()))?;
    writeln!(
        stdout,
        "[ok] wrote {} symbol entries to {}",
        entries.len(),
        options.symbols.display()
    )
    .map_err(|error| error.to_string())?;
    Ok(0)
}

fn lookup_symbol<W: Write, E: Write>(
    args: &[String],
    stdout: &mut W,
    stderr: &mut E,
) -> Result<i32, String> {
    let options = LookupOptions::parse(args)?;
    if !options.symbols.is_file() {
        return Err(format!(
            "{}: source symbol index does not exist; run `just phpt-source-index`",
            options.symbols.display()
        ));
    }
    let query = options.symbol.to_ascii_lowercase();
    let index = fs::read_to_string(&options.symbols)
        .map_err(|error| format!("{}: {error}", options.symbols.display()))?;
    let mut matches = Vec::new();
    for (line_index, line) in index.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let entry = match SymbolEntry::from_json_line(line) {
            Ok(entry) => entry,
            Err(error) => {
                writeln!(
                    stderr,
                    "{}:{}: {error}",
                    options.symbols.display(),
                    line_index + 1
                )
                .map_err(|io| io.to_string())?;
                continue;
            }
        };
        if entry.matches(&query) {
            matches.push(entry);
        }
    }
    if matches.is_empty() {
        writeln!(stderr, "no php-src symbol matches for `{}`", options.symbol)
            .map_err(|error| error.to_string())?;
        return Ok(1);
    }
    for entry in matches {
        writeln!(
            stdout,
            "{}\t{}\t{}\t{}:{}\t{}",
            entry.kind, entry.php_name, entry.c_name, entry.path, entry.line, entry.module
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(0)
}

#[derive(Debug)]
struct SourceOptions {
    php_src: PathBuf,
    manifest: PathBuf,
}

#[derive(Debug)]
struct SymbolOptions {
    php_src: PathBuf,
    symbols: PathBuf,
}

impl SymbolOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut php_src = None;
        let mut symbols = None;
        let mut index = 0usize;
        while index < args.len() {
            let arg = &args[index];
            if let Some(value) = arg.strip_prefix("--php-src=") {
                php_src = Some(PathBuf::from(value));
            } else if arg == "--php-src" {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("--php-src requires a path".to_string());
                };
                php_src = Some(PathBuf::from(value));
            } else if let Some(value) = arg.strip_prefix("--symbols=") {
                symbols = Some(PathBuf::from(value));
            } else if arg == "--symbols" {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("--symbols requires a path".to_string());
                };
                symbols = Some(PathBuf::from(value));
            } else {
                return Err(format!("unknown option `{arg}`"));
            }
            index += 1;
        }
        let php_src = php_src
            .or_else(|| env::var_os("PHP_SRC_DIR").map(PathBuf::from))
            .unwrap_or_else(default_php_src_dir);
        if !php_src.is_dir() {
            return Err(format!(
                "php-src checkout not found at {}; set PHP_SRC_DIR or --php-src",
                php_src.display()
            ));
        }
        Ok(Self {
            php_src,
            symbols: symbols.unwrap_or_else(|| PathBuf::from(DEFAULT_SYMBOLS)),
        })
    }
}

#[derive(Debug)]
struct LookupOptions {
    symbols: PathBuf,
    symbol: String,
}

#[derive(Debug)]
struct PhptIndexOptions {
    php_src: PathBuf,
    out: PathBuf,
    report: PathBuf,
}

#[derive(Debug)]
struct RunOptions {
    target: PathBuf,
    target_mode: TargetMode,
    manifest: PathBuf,
    php_src: PathBuf,
    work_dir: PathBuf,
    out: PathBuf,
    summary: PathBuf,
    timeout: Duration,
}

#[derive(Debug)]
struct BaselineOptions {
    results: PathBuf,
    corpus: PathBuf,
    known_failures: PathBuf,
    report: PathBuf,
    previous_known_failures: Option<PathBuf>,
    previous_results: Option<PathBuf>,
    timestamp: String,
}

#[derive(Debug)]
struct GenerateOptions {
    module: String,
    php_src: PathBuf,
    reference: PathBuf,
    corpus: PathBuf,
    known_failures: PathBuf,
    generated_dir: PathBuf,
    module_manifest: PathBuf,
    generated_manifest: PathBuf,
    work_dir: PathBuf,
    timestamp: String,
    smoke_count: usize,
    regression_count: usize,
    timeout: Duration,
}

impl RunOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut target = None;
        let mut manifest = None;
        let mut php_src = None;
        let mut work_dir = None;
        let mut out = None;
        let mut summary = None;
        let mut target_mode = None;
        let mut timeout = None;
        let mut index = 0usize;
        while index < args.len() {
            let arg = &args[index];
            match arg.as_str() {
                "--target" => {
                    index += 1;
                    target = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--target requires a path".to_string())?,
                    ));
                }
                "--manifest" => {
                    index += 1;
                    manifest = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--manifest requires a path".to_string())?,
                    ));
                }
                "--target-mode" => {
                    index += 1;
                    target_mode =
                        Some(TargetMode::parse(args.get(index).ok_or_else(|| {
                            "--target-mode requires php-cli or php-vm".to_string()
                        })?)?);
                }
                "--php-src" => {
                    index += 1;
                    php_src = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--php-src requires a path".to_string())?,
                    ));
                }
                "--work-dir" => {
                    index += 1;
                    work_dir = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--work-dir requires a path".to_string())?,
                    ));
                }
                "--out" => {
                    index += 1;
                    out = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--out requires a path".to_string())?,
                    ));
                }
                "--summary" => {
                    index += 1;
                    summary = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--summary requires a path".to_string())?,
                    ));
                }
                "--timeout-seconds" => {
                    index += 1;
                    timeout = Some(parse_duration_seconds(
                        args.get(index)
                            .ok_or_else(|| "--timeout-seconds requires a number".to_string())?,
                    )?);
                }
                _ if arg.starts_with("--target=") => {
                    target = Some(PathBuf::from(arg.trim_start_matches("--target=")));
                }
                _ if arg.starts_with("--manifest=") => {
                    manifest = Some(PathBuf::from(arg.trim_start_matches("--manifest=")));
                }
                _ if arg.starts_with("--target-mode=") => {
                    target_mode =
                        Some(TargetMode::parse(arg.trim_start_matches("--target-mode="))?);
                }
                _ if arg.starts_with("--php-src=") => {
                    php_src = Some(PathBuf::from(arg.trim_start_matches("--php-src=")));
                }
                _ if arg.starts_with("--work-dir=") => {
                    work_dir = Some(PathBuf::from(arg.trim_start_matches("--work-dir=")));
                }
                _ if arg.starts_with("--out=") => {
                    out = Some(PathBuf::from(arg.trim_start_matches("--out=")));
                }
                _ if arg.starts_with("--summary=") => {
                    summary = Some(PathBuf::from(arg.trim_start_matches("--summary=")));
                }
                _ if arg.starts_with("--timeout-seconds=") => {
                    timeout = Some(parse_duration_seconds(
                        arg.trim_start_matches("--timeout-seconds="),
                    )?);
                }
                _ => return Err(format!("unknown run option `{arg}`")),
            }
            index += 1;
        }
        let php_src = php_src
            .or_else(|| env::var_os("PHP_SRC_DIR").map(PathBuf::from))
            .unwrap_or_else(default_php_src_dir);
        let target = target
            .or_else(|| env::var_os("TARGET_PHP").map(PathBuf::from))
            .ok_or_else(|| "run requires --target or TARGET_PHP".to_string())?;
        let manifest = manifest.ok_or_else(|| "run requires --manifest".to_string())?;
        Ok(Self {
            target_mode: target_mode
                .or_else(|| {
                    env::var("PHPT_TARGET_MODE")
                        .ok()
                        .and_then(|value| TargetMode::parse(&value).ok())
                })
                .unwrap_or_else(|| infer_target_mode(&target)),
            target,
            manifest,
            php_src,
            work_dir: work_dir
                .or_else(|| env::var_os("PHPT_WORK_DIR").map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from("target/phpt-work")),
            out: out.unwrap_or_else(|| PathBuf::from("target/phpt-work/module-runs/results.jsonl")),
            summary: summary
                .unwrap_or_else(|| PathBuf::from("target/phpt-work/module-runs/summary.md")),
            timeout: timeout
                .or_else(|| {
                    env::var("PHPT_TIMEOUT_SECONDS")
                        .ok()
                        .and_then(|value| parse_duration_seconds(&value).ok())
                })
                .unwrap_or_else(|| Duration::from_secs(10)),
        })
    }
}

impl BaselineOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut results = None;
        let mut corpus = None;
        let mut known_failures = None;
        let mut report = None;
        let mut previous_known_failures = None;
        let mut previous_results = None;
        let mut timestamp = None;
        let mut index = 0usize;
        while index < args.len() {
            let arg = &args[index];
            match arg.as_str() {
                "--results" => {
                    index += 1;
                    results = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--results requires a path".to_string())?,
                    ));
                }
                "--corpus" => {
                    index += 1;
                    corpus = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--corpus requires a path".to_string())?,
                    ));
                }
                "--known-failures" => {
                    index += 1;
                    known_failures =
                        Some(PathBuf::from(args.get(index).ok_or_else(|| {
                            "--known-failures requires a path".to_string()
                        })?));
                }
                "--report" => {
                    index += 1;
                    report = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--report requires a path".to_string())?,
                    ));
                }
                "--previous-known-failures" => {
                    index += 1;
                    previous_known_failures =
                        Some(PathBuf::from(args.get(index).ok_or_else(|| {
                            "--previous-known-failures requires a path".to_string()
                        })?));
                }
                "--previous-results" => {
                    index += 1;
                    previous_results =
                        Some(PathBuf::from(args.get(index).ok_or_else(|| {
                            "--previous-results requires a path".to_string()
                        })?));
                }
                "--timestamp" => {
                    index += 1;
                    timestamp = Some(
                        args.get(index)
                            .ok_or_else(|| "--timestamp requires a value".to_string())?
                            .to_string(),
                    );
                }
                _ if arg.starts_with("--results=") => {
                    results = Some(PathBuf::from(arg.trim_start_matches("--results=")));
                }
                _ if arg.starts_with("--corpus=") => {
                    corpus = Some(PathBuf::from(arg.trim_start_matches("--corpus=")));
                }
                _ if arg.starts_with("--known-failures=") => {
                    known_failures =
                        Some(PathBuf::from(arg.trim_start_matches("--known-failures=")));
                }
                _ if arg.starts_with("--report=") => {
                    report = Some(PathBuf::from(arg.trim_start_matches("--report=")));
                }
                _ if arg.starts_with("--previous-known-failures=") => {
                    previous_known_failures = Some(PathBuf::from(
                        arg.trim_start_matches("--previous-known-failures="),
                    ));
                }
                _ if arg.starts_with("--previous-results=") => {
                    previous_results =
                        Some(PathBuf::from(arg.trim_start_matches("--previous-results=")));
                }
                _ if arg.starts_with("--timestamp=") => {
                    timestamp = Some(arg.trim_start_matches("--timestamp=").to_string());
                }
                _ => return Err(format!("unknown baseline option `{arg}`")),
            }
            index += 1;
        }
        Ok(Self {
            results: results.ok_or_else(|| "baseline requires --results".to_string())?,
            corpus: corpus.unwrap_or_else(|| PathBuf::from(DEFAULT_PHPT_CORPUS)),
            known_failures: known_failures
                .unwrap_or_else(|| PathBuf::from("tests/phpt/manifests/full-known-failures.jsonl")),
            report: report.unwrap_or_else(|| PathBuf::from("docs/phpt/reports/full-baseline.md")),
            previous_known_failures,
            previous_results,
            timestamp: timestamp
                .or_else(|| env::var("PHPT_BASELINE_TIMESTAMP").ok())
                .unwrap_or_else(|| "unknown".to_string()),
        })
    }
}

impl GenerateOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut module = None;
        let mut php_src = None;
        let mut reference = None;
        let mut corpus = None;
        let mut known_failures = None;
        let mut generated_dir = None;
        let mut module_manifest = None;
        let mut generated_manifest = None;
        let mut work_dir = None;
        let mut timestamp = None;
        let mut smoke_count = None;
        let mut regression_count = None;
        let mut timeout = None;
        let mut index = 0usize;
        while index < args.len() {
            let arg = &args[index];
            match arg.as_str() {
                "--module" => {
                    index += 1;
                    module = Some(
                        args.get(index)
                            .ok_or_else(|| "--module requires a value".to_string())?
                            .to_string(),
                    );
                }
                "--php-src" => {
                    index += 1;
                    php_src = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--php-src requires a path".to_string())?,
                    ));
                }
                "--reference" => {
                    index += 1;
                    reference = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--reference requires a path".to_string())?,
                    ));
                }
                "--corpus" => {
                    index += 1;
                    corpus = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--corpus requires a path".to_string())?,
                    ));
                }
                "--known-failures" => {
                    index += 1;
                    known_failures =
                        Some(PathBuf::from(args.get(index).ok_or_else(|| {
                            "--known-failures requires a path".to_string()
                        })?));
                }
                "--generated-dir" => {
                    index += 1;
                    generated_dir =
                        Some(PathBuf::from(args.get(index).ok_or_else(|| {
                            "--generated-dir requires a path".to_string()
                        })?));
                }
                "--module-manifest" => {
                    index += 1;
                    module_manifest =
                        Some(PathBuf::from(args.get(index).ok_or_else(|| {
                            "--module-manifest requires a path".to_string()
                        })?));
                }
                "--generated-manifest" => {
                    index += 1;
                    generated_manifest =
                        Some(PathBuf::from(args.get(index).ok_or_else(|| {
                            "--generated-manifest requires a path".to_string()
                        })?));
                }
                "--work-dir" => {
                    index += 1;
                    work_dir = Some(PathBuf::from(
                        args.get(index)
                            .ok_or_else(|| "--work-dir requires a path".to_string())?,
                    ));
                }
                "--timestamp" => {
                    index += 1;
                    timestamp = Some(
                        args.get(index)
                            .ok_or_else(|| "--timestamp requires a value".to_string())?
                            .to_string(),
                    );
                }
                "--smoke-count" => {
                    index += 1;
                    smoke_count = Some(parse_usize(
                        args.get(index)
                            .ok_or_else(|| "--smoke-count requires a number".to_string())?,
                        "--smoke-count",
                    )?);
                }
                "--regression-count" => {
                    index += 1;
                    regression_count = Some(parse_usize(
                        args.get(index)
                            .ok_or_else(|| "--regression-count requires a number".to_string())?,
                        "--regression-count",
                    )?);
                }
                "--timeout-seconds" => {
                    index += 1;
                    timeout = Some(parse_duration_seconds(
                        args.get(index)
                            .ok_or_else(|| "--timeout-seconds requires a number".to_string())?,
                    )?);
                }
                _ if arg.starts_with("MODULE=") => {
                    module = Some(arg.trim_start_matches("MODULE=").to_string());
                }
                _ if arg.starts_with("--module=") => {
                    module = Some(arg.trim_start_matches("--module=").to_string());
                }
                _ if arg.starts_with("--php-src=") => {
                    php_src = Some(PathBuf::from(arg.trim_start_matches("--php-src=")));
                }
                _ if arg.starts_with("--reference=") => {
                    reference = Some(PathBuf::from(arg.trim_start_matches("--reference=")));
                }
                _ if arg.starts_with("--corpus=") => {
                    corpus = Some(PathBuf::from(arg.trim_start_matches("--corpus=")));
                }
                _ if arg.starts_with("--known-failures=") => {
                    known_failures =
                        Some(PathBuf::from(arg.trim_start_matches("--known-failures=")));
                }
                _ if arg.starts_with("--generated-dir=") => {
                    generated_dir = Some(PathBuf::from(arg.trim_start_matches("--generated-dir=")));
                }
                _ if arg.starts_with("--module-manifest=") => {
                    module_manifest =
                        Some(PathBuf::from(arg.trim_start_matches("--module-manifest=")));
                }
                _ if arg.starts_with("--generated-manifest=") => {
                    generated_manifest = Some(PathBuf::from(
                        arg.trim_start_matches("--generated-manifest="),
                    ));
                }
                _ if arg.starts_with("--work-dir=") => {
                    work_dir = Some(PathBuf::from(arg.trim_start_matches("--work-dir=")));
                }
                _ if arg.starts_with("--timestamp=") => {
                    timestamp = Some(arg.trim_start_matches("--timestamp=").to_string());
                }
                _ if arg.starts_with("--smoke-count=") => {
                    smoke_count = Some(parse_usize(
                        arg.trim_start_matches("--smoke-count="),
                        "--smoke-count",
                    )?);
                }
                _ if arg.starts_with("--regression-count=") => {
                    regression_count = Some(parse_usize(
                        arg.trim_start_matches("--regression-count="),
                        "--regression-count",
                    )?);
                }
                _ if arg.starts_with("--timeout-seconds=") => {
                    timeout = Some(parse_duration_seconds(
                        arg.trim_start_matches("--timeout-seconds="),
                    )?);
                }
                _ => return Err(format!("unknown generate option `{arg}`")),
            }
            index += 1;
        }
        let module = module
            .or_else(|| env::var("MODULE").ok())
            .ok_or_else(|| "generate requires --module or MODULE".to_string())?;
        let safe_module = safe_path_component(&module);
        let php_src = php_src
            .or_else(|| env::var_os("PHP_SRC_DIR").map(PathBuf::from))
            .unwrap_or_else(default_php_src_dir);
        let reference = reference
            .or_else(|| env::var_os("REFERENCE_PHP").map(PathBuf::from))
            .unwrap_or_else(|| php_src.join("sapi/cli/php"));
        if !reference.is_file() {
            return Err(format!(
                "reference PHP CLI is not built: {}; set REFERENCE_PHP",
                reference.display()
            ));
        }
        Ok(Self {
            module,
            php_src,
            reference,
            corpus: corpus.unwrap_or_else(|| PathBuf::from(DEFAULT_PHPT_CORPUS)),
            known_failures: known_failures
                .unwrap_or_else(|| PathBuf::from("tests/phpt/manifests/full-known-failures.jsonl")),
            generated_dir: generated_dir
                .unwrap_or_else(|| PathBuf::from("tests/phpt/generated").join(&safe_module)),
            module_manifest: module_manifest.unwrap_or_else(|| {
                PathBuf::from("tests/phpt/manifests").join(format!("{safe_module}-originals.jsonl"))
            }),
            generated_manifest: generated_manifest.unwrap_or_else(|| {
                PathBuf::from("tests/phpt/manifests").join(format!("{safe_module}-generated.jsonl"))
            }),
            work_dir: work_dir.unwrap_or_else(|| {
                PathBuf::from("target/phpt-work")
                    .join("generate")
                    .join(&safe_module)
            }),
            timestamp: timestamp
                .or_else(|| env::var("PHPT_GENERATED_TIMESTAMP").ok())
                .unwrap_or_else(|| "unknown".to_string()),
            smoke_count: smoke_count.unwrap_or(3),
            regression_count: regression_count.unwrap_or(2),
            timeout: timeout
                .or_else(|| {
                    env::var("PHPT_TIMEOUT_SECONDS")
                        .ok()
                        .and_then(|value| parse_duration_seconds(&value).ok())
                })
                .unwrap_or_else(|| Duration::from_secs(10)),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TargetMode {
    PhpCli,
    PhpVm,
}

impl TargetMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "php-cli" => Ok(Self::PhpCli),
            "php-vm" => Ok(Self::PhpVm),
            _ => Err(format!(
                "unknown target mode `{value}`; expected php-cli or php-vm"
            )),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PhptRunResult {
    path: String,
    outcome: String,
    detail: String,
}

impl PhptRunResult {
    fn to_json_line(&self) -> String {
        format!(
            "{{\"path\":\"{}\",\"outcome\":\"{}\",\"detail\":\"{}\"}}",
            escape_json(&self.path),
            escape_json(&self.outcome),
            escape_json(&self.detail)
        )
    }

    fn from_json_line(line: &str) -> Result<Self, String> {
        Ok(Self {
            path: extract_json_string(line, "path")?,
            outcome: extract_json_string(line, "outcome")?,
            detail: extract_json_string(line, "detail")?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct KnownFailure {
    path: String,
    module_tag: String,
    outcome: String,
    failure_fingerprint: String,
    primary_missing_feature_guess: String,
    owner_module: String,
    first_seen_timestamp: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GeneratedCase {
    path: PathBuf,
    manifest_path: String,
    module: String,
    kind: String,
    original_path: String,
    original_source_hash: String,
    generated_timestamp: String,
    generator_version: String,
    reason: String,
    source: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReductionMode {
    LineRemoval,
}

impl KnownFailure {
    fn to_json_line(&self) -> String {
        format!(
            "{{\"path\":\"{}\",\"module_tag\":\"{}\",\"outcome\":\"{}\",\"failure_fingerprint\":\"{}\",\"primary_missing_feature_guess\":\"{}\",\"owner_module\":\"{}\",\"first_seen_timestamp\":\"{}\"}}",
            escape_json(&self.path),
            escape_json(&self.module_tag),
            escape_json(&self.outcome),
            escape_json(&self.failure_fingerprint),
            escape_json(&self.primary_missing_feature_guess),
            escape_json(&self.owner_module),
            escape_json(&self.first_seen_timestamp)
        )
    }

    fn from_json_line(line: &str) -> Result<Self, String> {
        Ok(Self {
            path: extract_json_string(line, "path")?,
            module_tag: extract_json_string(line, "module_tag")?,
            outcome: extract_json_string(line, "outcome")?,
            failure_fingerprint: extract_json_string(line, "failure_fingerprint")?,
            primary_missing_feature_guess: extract_json_string(
                line,
                "primary_missing_feature_guess",
            )?,
            owner_module: extract_json_string(line, "owner_module")?,
            first_seen_timestamp: extract_json_string(line, "first_seen_timestamp")?,
        })
    }
}

impl GeneratedCase {
    fn to_json_line(&self) -> String {
        format!(
            "{{\"path\":\"{}\",\"module\":\"{}\",\"kind\":\"{}\",\"original_path\":\"{}\",\"original_source_hash\":\"{}\",\"generated_timestamp\":\"{}\",\"generator_version\":\"{}\",\"reason\":\"{}\"}}",
            escape_json(&self.manifest_path),
            escape_json(&self.module),
            escape_json(&self.kind),
            escape_json(&self.original_path),
            escape_json(&self.original_source_hash),
            escape_json(&self.generated_timestamp),
            escape_json(&self.generator_version),
            escape_json(&self.reason)
        )
    }
}

impl PhptIndexOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut php_src = None;
        let mut out = None;
        let mut report = None;
        let mut index = 0usize;
        while index < args.len() {
            let arg = &args[index];
            if let Some(value) = arg.strip_prefix("--php-src=") {
                php_src = Some(PathBuf::from(value));
            } else if arg == "--php-src" {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("--php-src requires a path".to_string());
                };
                php_src = Some(PathBuf::from(value));
            } else if let Some(value) = arg.strip_prefix("--out=") {
                out = Some(PathBuf::from(value));
            } else if arg == "--out" {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("--out requires a path".to_string());
                };
                out = Some(PathBuf::from(value));
            } else if let Some(value) = arg.strip_prefix("--report=") {
                report = Some(PathBuf::from(value));
            } else if arg == "--report" {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("--report requires a path".to_string());
                };
                report = Some(PathBuf::from(value));
            } else {
                return Err(format!("unknown option `{arg}`"));
            }
            index += 1;
        }
        let php_src = php_src
            .or_else(|| env::var_os("PHP_SRC_DIR").map(PathBuf::from))
            .unwrap_or_else(default_php_src_dir);
        if !php_src.is_dir() {
            return Err(format!(
                "php-src checkout not found at {}; set PHP_SRC_DIR or --php-src",
                php_src.display()
            ));
        }
        Ok(Self {
            php_src,
            out: out.unwrap_or_else(|| PathBuf::from(DEFAULT_PHPT_CORPUS)),
            report: report.unwrap_or_else(|| PathBuf::from(DEFAULT_PHPT_REPORT)),
        })
    }
}

impl LookupOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut symbols = None;
        let mut symbol = None;
        let mut index = 0usize;
        while index < args.len() {
            let arg = &args[index];
            if let Some(value) = arg.strip_prefix("--symbols=") {
                symbols = Some(PathBuf::from(value));
            } else if arg == "--symbols" {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("--symbols requires a path".to_string());
                };
                symbols = Some(PathBuf::from(value));
            } else if let Some(value) = arg.strip_prefix("--symbol=") {
                symbol = Some(value.to_string());
            } else if arg == "--symbol" {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("--symbol requires a value".to_string());
                };
                symbol = Some(value.to_string());
            } else if let Some(value) = arg.strip_prefix("SYMBOL=") {
                symbol = Some(value.to_string());
            } else if symbol.is_none() {
                symbol = Some(arg.to_string());
            } else {
                return Err(format!("unknown option `{arg}`"));
            }
            index += 1;
        }
        let Some(symbol) = symbol else {
            return Err("lookup-symbol requires SYMBOL=<name> or --symbol <name>".to_string());
        };
        Ok(Self {
            symbols: symbols.unwrap_or_else(|| PathBuf::from(DEFAULT_SYMBOLS)),
            symbol,
        })
    }
}

impl SourceOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut php_src = None;
        let mut manifest = None;
        let mut index = 0usize;
        while index < args.len() {
            let arg = &args[index];
            if let Some(value) = arg.strip_prefix("--php-src=") {
                php_src = Some(PathBuf::from(value));
            } else if arg == "--php-src" {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("--php-src requires a path".to_string());
                };
                php_src = Some(PathBuf::from(value));
            } else if let Some(value) = arg.strip_prefix("--manifest=") {
                manifest = Some(PathBuf::from(value));
            } else if arg == "--manifest" {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("--manifest requires a path".to_string());
                };
                manifest = Some(PathBuf::from(value));
            } else {
                return Err(format!("unknown option `{arg}`"));
            }
            index += 1;
        }
        let php_src = php_src
            .or_else(|| env::var_os("PHP_SRC_DIR").map(PathBuf::from))
            .unwrap_or_else(default_php_src_dir);
        if !php_src.is_dir() {
            return Err(format!(
                "php-src checkout not found at {}; set PHP_SRC_DIR or --php-src",
                php_src.display()
            ));
        }
        Ok(Self {
            php_src,
            manifest: manifest.unwrap_or_else(|| PathBuf::from(DEFAULT_MANIFEST)),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ManifestEntry {
    path: String,
    size: u64,
    sha256: String,
    kind: FileKind,
}

impl ManifestEntry {
    fn to_json_line(&self) -> String {
        format!(
            "{{\"path\":\"{}\",\"size\":{},\"sha256\":\"{}\",\"kind\":\"{}\"}}",
            escape_json(&self.path),
            self.size,
            self.sha256,
            self.kind.as_str()
        )
    }

    fn from_json_line(line: &str) -> Result<Self, String> {
        Ok(Self {
            path: extract_json_string(line, "path")?,
            size: extract_json_u64(line, "size")?,
            sha256: extract_json_string(line, "sha256")?,
            kind: FileKind::parse(&extract_json_string(line, "kind")?)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum FileKind {
    Phpt,
    CSource,
    Header,
    ZendSource,
    RunTests,
    FixtureSupport,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SymbolEntry {
    kind: String,
    php_name: String,
    c_name: String,
    path: String,
    line: u64,
    module: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PhptEntry {
    path: String,
    title: String,
    sections: Vec<String>,
    module: String,
    has_skipif: bool,
    has_clean: bool,
    has_redirecttest: bool,
    has_external_files: bool,
    uses_http_sections: bool,
    uses_stdin_args: bool,
    expectation_kind: String,
    source_hash: String,
}

impl PhptEntry {
    fn to_json_line(&self) -> String {
        format!(
            "{{\"path\":\"{}\",\"title\":\"{}\",\"sections\":{},\"module\":\"{}\",\"has_skipif\":{},\"has_clean\":{},\"has_redirecttest\":{},\"has_external_files\":{},\"uses_http_sections\":{},\"uses_stdin_args\":{},\"expectation_kind\":\"{}\",\"source_hash\":\"{}\"}}",
            escape_json(&self.path),
            escape_json(&self.title),
            json_string_array(&self.sections),
            escape_json(&self.module),
            self.has_skipif,
            self.has_clean,
            self.has_redirecttest,
            self.has_external_files,
            self.uses_http_sections,
            self.uses_stdin_args,
            escape_json(&self.expectation_kind),
            self.source_hash
        )
    }

    fn from_json_line(line: &str) -> Result<Self, String> {
        Ok(Self {
            path: extract_json_string(line, "path")?,
            title: extract_json_string(line, "title")?,
            sections: extract_json_string_array(line, "sections")?,
            module: extract_json_string(line, "module")?,
            has_skipif: extract_json_bool(line, "has_skipif")?,
            has_clean: extract_json_bool(line, "has_clean")?,
            has_redirecttest: extract_json_bool(line, "has_redirecttest")?,
            has_external_files: extract_json_bool(line, "has_external_files")?,
            uses_http_sections: extract_json_bool(line, "uses_http_sections")?,
            uses_stdin_args: extract_json_bool(line, "uses_stdin_args")?,
            expectation_kind: extract_json_string(line, "expectation_kind")?,
            source_hash: extract_json_string(line, "source_hash")?,
        })
    }
}

impl SymbolEntry {
    fn to_json_line(&self) -> String {
        format!(
            "{{\"kind\":\"{}\",\"php_name\":\"{}\",\"c_name\":\"{}\",\"path\":\"{}\",\"line\":{},\"module\":\"{}\"}}",
            escape_json(&self.kind),
            escape_json(&self.php_name),
            escape_json(&self.c_name),
            escape_json(&self.path),
            self.line,
            escape_json(&self.module)
        )
    }

    fn from_json_line(line: &str) -> Result<Self, String> {
        Ok(Self {
            kind: extract_json_string(line, "kind")?,
            php_name: extract_json_string(line, "php_name")?,
            c_name: extract_json_string(line, "c_name")?,
            path: extract_json_string(line, "path")?,
            line: extract_json_u64(line, "line")?,
            module: extract_json_string(line, "module")?,
        })
    }

    fn matches(&self, query: &str) -> bool {
        self.php_name.to_ascii_lowercase() == query
            || self.c_name.to_ascii_lowercase() == query
            || self.path.to_ascii_lowercase().contains(query)
            || self
                .php_name
                .to_ascii_lowercase()
                .contains(&format!("::{query}"))
    }
}

impl FileKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Phpt => "phpt",
            Self::CSource => "c_source",
            Self::Header => "header",
            Self::ZendSource => "zend_source",
            Self::RunTests => "run_tests",
            Self::FixtureSupport => "fixture_support",
            Self::Other => "other",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "phpt" => Ok(Self::Phpt),
            "c_source" => Ok(Self::CSource),
            "header" => Ok(Self::Header),
            "zend_source" => Ok(Self::ZendSource),
            "run_tests" => Ok(Self::RunTests),
            "fixture_support" => Ok(Self::FixtureSupport),
            "other" => Ok(Self::Other),
            _ => Err(format!("unknown file kind `{value}`")),
        }
    }
}

fn collect_manifest_entries(php_src: &Path) -> Result<Vec<ManifestEntry>, String> {
    let mut entries = Vec::new();
    collect_recursive(php_src, php_src, &mut entries)?;
    Ok(entries)
}

fn collect_symbol_entries(php_src: &Path) -> Result<Vec<SymbolEntry>, String> {
    let mut source_files = Vec::new();
    collect_symbol_source_files(php_src, php_src, &mut source_files)?;
    source_files.sort();
    let mut entries = Vec::new();
    for rel in source_files {
        let path = php_src.join(&rel);
        if rel.starts_with("Zend/") && is_c_or_header(&rel) {
            entries.push(SymbolEntry {
                kind: "zend_source_file".to_string(),
                php_name: String::new(),
                c_name: source_stem(&rel),
                path: rel.clone(),
                line: 1,
                module: module_guess(&rel),
            });
        }
        scan_symbol_file(&path, &rel, &mut entries)?;
    }
    Ok(entries)
}

fn collect_phpt_entries(php_src: &Path) -> Result<Vec<PhptEntry>, String> {
    let mut files = Vec::new();
    collect_phpt_files(php_src, php_src, &mut files)?;
    files.sort();
    let mut entries = Vec::new();
    for rel in files {
        let path = php_src.join(&rel);
        let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let source = String::from_utf8_lossy(&bytes);
        let document = parse_phpt(&source);
        let sections = document.sections;
        let section_names = sections
            .iter()
            .map(|section| section.name.clone())
            .collect::<Vec<_>>();
        let title = sections
            .iter()
            .find(|section| section.name == "TEST")
            .map(|section| first_non_empty_line(&section.body))
            .unwrap_or_default();
        let (_, source_hash) = hash_file(&path)?;
        entries.push(PhptEntry {
            path: rel.clone(),
            title,
            module: phpt_module_tag(&rel, &sections),
            has_skipif: has_section(&sections, "SKIPIF"),
            has_clean: has_section(&sections, "CLEAN"),
            has_redirecttest: has_section(&sections, "REDIRECTTEST"),
            has_external_files: sections
                .iter()
                .any(|section| section.name.ends_with("_EXTERNAL")),
            uses_http_sections: sections.iter().any(|section| {
                matches!(
                    section.name.as_str(),
                    "GET" | "POST" | "POST_RAW" | "PUT" | "COOKIE" | "EXPECTHEADERS"
                )
            }),
            uses_stdin_args: sections
                .iter()
                .any(|section| matches!(section.name.as_str(), "STDIN" | "ARGS")),
            expectation_kind: expectation_kind(&sections),
            source_hash,
            sections: section_names,
        });
    }
    Ok(entries)
}

fn collect_phpt_files(
    php_src: &Path,
    current: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    let mut children = fs::read_dir(current)
        .map_err(|error| format!("{}: {error}", current.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", current.display()))?;
    children.sort_by_key(|entry| entry.path());
    for child in children {
        let path = child.path();
        let file_type = child
            .file_type()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if file_type.is_dir() {
            if should_skip_dir(php_src, &path) {
                continue;
            }
            collect_phpt_files(php_src, &path, files)?;
        } else if file_type.is_file()
            && path.extension().and_then(|ext| ext.to_str()) == Some("phpt")
        {
            files.push(relative_path(php_src, &path)?);
        }
    }
    Ok(())
}

fn resolve_phpt_path(php_src: &Path, manifest_path: &str) -> PathBuf {
    let path = PathBuf::from(manifest_path);
    if path.is_file() {
        path
    } else {
        php_src.join(manifest_path)
    }
}

fn section<'a>(sections: &'a [PhptSection], name: &str) -> Option<&'a PhptSection> {
    sections.iter().find(|section| section.name == name)
}

fn file_body(sections: &[PhptSection], phpt_path: &Path) -> Result<Option<String>, String> {
    if let Some(section) = section(sections, "FILE").or_else(|| section(sections, "FILEEOF")) {
        return Ok(Some(section.body.clone()));
    }
    if let Some(section) = section(sections, "FILE_EXTERNAL") {
        let external = phpt_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(first_non_empty_line(&section.body));
        return fs::read_to_string(&external)
            .map(Some)
            .map_err(|error| format!("{}: {error}", external.display()));
    }
    Ok(None)
}

fn expectation(
    sections: &[PhptSection],
    phpt_path: &Path,
) -> Result<Option<(ExpectationKind, String)>, String> {
    for (name, kind) in [
        ("EXPECT", ExpectationKind::Expect),
        ("EXPECTF", ExpectationKind::ExpectF),
        ("EXPECTREGEX", ExpectationKind::ExpectRegex),
    ] {
        if let Some(section) = section(sections, name) {
            return Ok(Some((kind, section.body.clone())));
        }
    }
    for (name, kind) in [
        ("EXPECT_EXTERNAL", ExpectationKind::Expect),
        ("EXPECTF_EXTERNAL", ExpectationKind::ExpectF),
        ("EXPECTREGEX_EXTERNAL", ExpectationKind::ExpectRegex),
    ] {
        if let Some(section) = section(sections, name) {
            let external = phpt_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(first_non_empty_line(&section.body));
            let expected = fs::read_to_string(&external)
                .map_err(|error| format!("{}: {error}", external.display()))?;
            return Ok(Some((kind, expected)));
        }
    }
    Ok(None)
}

fn ini_args(sections: &[PhptSection]) -> Vec<(String, String)> {
    let Some(section) = section(sections, "INI") else {
        return Vec::new();
    };
    section
        .body
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
        .collect()
}

fn env_args(sections: &[PhptSection]) -> Vec<(String, String)> {
    let mut env = Vec::new();
    for section_name in ["ENV", "GET", "POST", "POST_RAW", "PUT", "COOKIE"] {
        if let Some(section) = section(sections, section_name) {
            match section_name {
                "GET" => env.push(("QUERY_STRING".to_string(), section.body.trim().to_string())),
                "POST" | "POST_RAW" | "PUT" => {
                    env.push((
                        "REQUEST_METHOD".to_string(),
                        section_name.replace("_RAW", ""),
                    ));
                    env.push(("PHPT_REQUEST_BODY".to_string(), section.body.clone()));
                }
                "COOKIE" => env.push(("HTTP_COOKIE".to_string(), section.body.trim().to_string())),
                "ENV" => {
                    for line in section.body.lines() {
                        if let Some((name, value)) = line.split_once('=') {
                            env.push((name.trim().to_string(), value.trim().to_string()));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    env
}

#[derive(Debug)]
struct PhptExecutionContext<'a> {
    ini: Vec<(String, String)>,
    env: Vec<(String, String)>,
    args: Vec<String>,
    stdin: Option<&'a str>,
}

fn context_from_sections(sections: &[PhptSection]) -> PhptExecutionContext<'_> {
    PhptExecutionContext {
        ini: ini_args(sections),
        env: env_args(sections),
        args: section(sections, "ARGS")
            .map(|section| split_phpt_args(&section.body))
            .unwrap_or_default(),
        stdin: section(sections, "STDIN").map(|section| section.body.as_str()),
    }
}

fn split_phpt_args(args: &str) -> Vec<String> {
    args.split_whitespace().map(str::to_string).collect()
}

fn run_clean_if_present(
    options: &RunOptions,
    sections: &[PhptSection],
    work_dir: &Path,
) -> Result<(), String> {
    let Some(clean) = section(sections, "CLEAN") else {
        return Ok(());
    };
    let clean_path = work_dir.join("clean.php");
    fs::write(&clean_path, &clean.body)
        .map_err(|error| format!("{}: {error}", clean_path.display()))?;
    let _ = run_php(options, &clean_path, work_dir, &[], &[], &[], None)?;
    Ok(())
}

#[derive(Debug)]
struct ProcessOutput {
    status: i32,
    stdout: String,
    stderr: String,
}

fn run_php(
    options: &RunOptions,
    script: &Path,
    cwd: &Path,
    ini: &[(String, String)],
    envs: &[(String, String)],
    script_args: &[String],
    stdin: Option<&str>,
) -> Result<ProcessOutput, String> {
    let target = fs::canonicalize(&options.target)
        .map_err(|error| format!("{}: {error}", options.target.display()))?;
    let script =
        fs::canonicalize(script).map_err(|error| format!("{}: {error}", script.display()))?;
    let mut command = Command::new(&target);
    command.current_dir(cwd);
    match options.target_mode {
        TargetMode::PhpCli => {
            command.arg("-n");
            for (name, value) in ini {
                command.arg("-d").arg(format!("{name}={value}"));
            }
            command.arg(script);
            command.args(script_args);
        }
        TargetMode::PhpVm => {
            command.arg("run");
            for (name, value) in envs {
                command.arg("--env").arg(format!("{name}={value}"));
            }
            for (name, value) in ini {
                command
                    .arg("--env")
                    .arg(format!("PHPT_INI_{}={value}", sanitize_env_name(name)));
            }
            command.arg(script);
            if !script_args.is_empty() {
                command.arg("--");
                command.args(script_args);
            }
        }
    }
    if options.target_mode == TargetMode::PhpCli {
        for (name, value) in envs {
            command.env(name, value);
        }
    }
    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("{}: {error}", target.display()))?;
    if let Some(stdin) = stdin
        && let Some(mut child_stdin) = child.stdin.take()
    {
        child_stdin
            .write_all(stdin.as_bytes())
            .map_err(|error| format!("stdin: {error}"))?;
    }
    let start = Instant::now();
    let output = loop {
        if child
            .try_wait()
            .map_err(|error| format!("{}: {error}", target.display()))?
            .is_some()
        {
            break child
                .wait_with_output()
                .map_err(|error| format!("{}: {error}", target.display()))?;
        }
        if start.elapsed() > options.timeout {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .map_err(|error| format!("{}: {error}", target.display()))?;
            return Ok(ProcessOutput {
                status: 124,
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: format!(
                    "PHPT_TIMEOUT after {}s\n{}",
                    options.timeout.as_secs(),
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Ok(ProcessOutput {
        status: output.status.code().unwrap_or(255),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn normalize_expected(value: &str) -> String {
    let mut normalized = value.replace("\r\n", "\n");
    while normalized.ends_with('\n') {
        normalized.pop();
    }
    normalized
}

fn read_phpt_corpus(path: &Path) -> Result<Vec<PhptEntry>, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut entries = Vec::new();
    for (index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        entries.push(
            PhptEntry::from_json_line(line)
                .map_err(|error| format!("{}:{}: {error}", path.display(), index + 1))?,
        );
    }
    Ok(entries)
}

fn build_generated_case(
    options: &GenerateOptions,
    reference_options: &RunOptions,
    entry: &PhptEntry,
    kind: &str,
    reason: &str,
    reduction: Option<ReductionMode>,
    index: usize,
) -> Result<Option<GeneratedCase>, String> {
    let phpt_path = options.php_src.join(&entry.path);
    let source = fs::read_to_string(&phpt_path)
        .map_err(|error| format!("{}: {error}", phpt_path.display()))?;
    let document = parse_phpt(&source);
    let Some(mut body) = file_body(&document.sections, &phpt_path)? else {
        return Ok(None);
    };
    let base = run_reference_body(
        reference_options,
        &document.sections,
        &body,
        &options.work_dir.join(format!("candidate-{index}")),
    )?;
    if base.status != 0 {
        return Ok(None);
    }
    if matches!(reduction, Some(ReductionMode::LineRemoval)) {
        body = reduce_body_by_reference_equivalence(
            reference_options,
            &document.sections,
            &body,
            &base,
            &options.work_dir.join(format!("reduce-{index}")),
        )?;
    }
    let final_output = run_reference_body(
        reference_options,
        &document.sections,
        &body,
        &options.work_dir.join(format!("final-{index}")),
    )?;
    if final_output.status != 0 {
        return Ok(None);
    }

    let stem = entry
        .path
        .rsplit('/')
        .next()
        .unwrap_or("generated.phpt")
        .trim_end_matches(".phpt");
    let file_name = format!(
        "{}-{}-{}.phpt",
        kind,
        safe_path_component(stem),
        &entry.source_hash[..12.min(entry.source_hash.len())]
    );
    let path = options.generated_dir.join(file_name);
    let manifest_path = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    let generated_source = render_generated_phpt(
        options,
        entry,
        kind,
        reason,
        &body,
        &final_output.stdout,
        &document.sections,
    );
    Ok(Some(GeneratedCase {
        path,
        manifest_path,
        module: options.module.clone(),
        kind: kind.to_string(),
        original_path: entry.path.clone(),
        original_source_hash: entry.source_hash.clone(),
        generated_timestamp: options.timestamp.clone(),
        generator_version: GENERATOR_VERSION.to_string(),
        reason: reason.to_string(),
        source: generated_source,
    }))
}

fn write_generated_case(case: &GeneratedCase) -> Result<(), String> {
    if let Some(parent) = case.path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(&case.path, &case.source).map_err(|error| format!("{}: {error}", case.path.display()))
}

fn clear_generated_phpts(dir: &Path) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)
        .map_err(|error| format!("{}: {error}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", dir.display()))?
    {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("phpt") {
            fs::remove_file(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        }
    }
    Ok(())
}

fn run_reference_body(
    options: &RunOptions,
    sections: &[PhptSection],
    body: &str,
    work_dir: &Path,
) -> Result<ProcessOutput, String> {
    let _ = fs::remove_dir_all(work_dir);
    fs::create_dir_all(work_dir).map_err(|error| format!("{}: {error}", work_dir.display()))?;
    let script = work_dir.join("test.php");
    fs::write(&script, body).map_err(|error| format!("{}: {error}", script.display()))?;
    let context = context_from_sections(sections);
    run_php(
        options,
        &script,
        work_dir,
        &context.ini,
        &context.env,
        &context.args,
        context.stdin,
    )
}

fn reduce_body_by_reference_equivalence(
    options: &RunOptions,
    sections: &[PhptSection],
    body: &str,
    expected: &ProcessOutput,
    work_dir: &Path,
) -> Result<String, String> {
    let mut lines = body
        .split_inclusive('\n')
        .map(str::to_string)
        .collect::<Vec<_>>();
    if lines.len() > 80 {
        return Ok(body.to_string());
    }
    let mut index = 0usize;
    let mut attempts = 0usize;
    while index < lines.len() && attempts < 200 {
        let line = &lines[index];
        if line.trim_start().starts_with("<?php") {
            index += 1;
            continue;
        }
        let mut candidate = lines.clone();
        candidate.remove(index);
        let candidate_body = candidate.concat();
        attempts += 1;
        let output = run_reference_body(
            options,
            sections,
            &candidate_body,
            &work_dir.join(format!("attempt-{attempts}")),
        )?;
        if output.status == expected.status
            && output.stdout == expected.stdout
            && output.stderr == expected.stderr
        {
            lines = candidate;
        } else {
            index += 1;
        }
    }
    Ok(lines.concat())
}

fn render_generated_phpt(
    options: &GenerateOptions,
    entry: &PhptEntry,
    kind: &str,
    reason: &str,
    body: &str,
    expected_stdout: &str,
    sections: &[PhptSection],
) -> String {
    let mut out = String::new();
    out.push_str("--TEST--\n");
    out.push_str(&format!(
        "PHPT generated {kind}: {}\n",
        first_non_empty_line(&entry.title)
    ));
    out.push_str("--DESCRIPTION--\n");
    out.push_str(&format!("original php-src path: {}\n", entry.path));
    out.push_str(&format!("original source hash: {}\n", entry.source_hash));
    out.push_str(&format!("generated timestamp: {}\n", options.timestamp));
    out.push_str(&format!("generator version: {GENERATOR_VERSION}\n"));
    out.push_str(&format!("reason: {reason}\n"));
    if let Some(ini) = section(sections, "INI") {
        out.push_str("--INI--\n");
        out.push_str(&ini.body);
        ensure_trailing_newline(&mut out);
    }
    if let Some(env) = section(sections, "ENV") {
        out.push_str("--ENV--\n");
        out.push_str(&env.body);
        ensure_trailing_newline(&mut out);
    }
    if let Some(args) = section(sections, "ARGS") {
        out.push_str("--ARGS--\n");
        out.push_str(&args.body);
        ensure_trailing_newline(&mut out);
    }
    if let Some(stdin) = section(sections, "STDIN") {
        out.push_str("--STDIN--\n");
        out.push_str(&stdin.body);
        ensure_trailing_newline(&mut out);
    }
    out.push_str("--FILE--\n");
    out.push_str(body);
    ensure_trailing_newline(&mut out);
    out.push_str("--EXPECT--\n");
    out.push_str(expected_stdout);
    ensure_trailing_newline(&mut out);
    out
}

fn ensure_trailing_newline(value: &mut String) {
    if !value.ends_with('\n') {
        value.push('\n');
    }
}

fn matches_module_selector(entry: &PhptEntry, selector: &str) -> bool {
    if entry.module == selector {
        return true;
    }
    match selector {
        "zend.basic" => {
            entry.path.starts_with("Zend/tests/")
                && entry.path["Zend/tests/".len()..].matches('/').count() == 0
        }
        _ if selector.starts_with("zend.") => {
            let subdir = selector
                .trim_start_matches("zend.")
                .replace('.', "/")
                .replace('_', "-");
            entry.path.starts_with(&format!("Zend/tests/{subdir}/"))
        }
        _ if selector.starts_with("ext.") => {
            let extension = selector.trim_start_matches("ext.");
            entry.path.starts_with(&format!("ext/{extension}/"))
        }
        _ => false,
    }
}

fn is_simple_generation_candidate(entry: &PhptEntry) -> bool {
    !entry.has_skipif
        && !entry.has_clean
        && !entry.has_redirecttest
        && !entry.has_external_files
        && !entry.uses_http_sections
        && !entry.uses_stdin_args
        && entry.expectation_kind == "expect"
        && entry
            .sections
            .iter()
            .any(|section| section == "FILE" || section == "FILEEOF")
}

fn source_len(path: &Path) -> u64 {
    path.metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(u64::MAX)
}

fn safe_path_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    sanitized.trim_matches('-').to_string()
}

fn read_run_results(path: &Path) -> Result<Vec<PhptRunResult>, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut results = Vec::new();
    for (index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        results.push(
            PhptRunResult::from_json_line(line)
                .map_err(|error| format!("{}:{}: {error}", path.display(), index + 1))?,
        );
    }
    Ok(results)
}

fn read_known_failures(path: &Path) -> Result<Vec<KnownFailure>, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut failures = Vec::new();
    for (index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        failures.push(
            KnownFailure::from_json_line(line)
                .map_err(|error| format!("{}:{}: {error}", path.display(), index + 1))?,
        );
    }
    Ok(failures)
}

fn read_corpus_modules(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut modules = BTreeMap::new();
    for (index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let path_value = extract_json_string(line, "path")
            .map_err(|error| format!("{}:{}: {error}", path.display(), index + 1))?;
        let module = extract_json_string(line, "module")
            .map_err(|error| format!("{}:{}: {error}", path.display(), index + 1))?;
        modules.insert(path_value, module);
    }
    Ok(modules)
}

fn failure_fingerprint(result: &PhptRunResult) -> String {
    let mut hasher = Sha256::new();
    hasher.update(result.outcome.as_bytes());
    hasher.update(b"\0");
    hasher.update(normalize_failure_detail_for_fingerprint(&result.detail).as_bytes());
    format!("{:x}", hasher.finalize())
}

fn normalize_failure_detail_for_fingerprint(detail: &str) -> String {
    let mut normalized = detail.to_string();
    for marker in ["/target/phpt-work/", "target/phpt-work/"] {
        while let Some(marker_start) = normalized.find(marker) {
            let prefix_start = normalized[..marker_start]
                .rfind(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '=' | '"' | '`'))
                .map(|index| index + 1)
                .unwrap_or(0);
            let Some(test_php_offset) = normalized[marker_start..].find("test.php") else {
                break;
            };
            let end = marker_start + test_php_offset + "test.php".len();
            normalized.replace_range(prefix_start..end, "<phpt-test.php>");
        }
    }
    for marker in ["/target/phpt-work/", "target/phpt-work/"] {
        while let Some(marker_start) = normalized.find(marker) {
            let prefix_start = normalized[..marker_start]
                .rfind(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '=' | '"' | '`'))
                .map(|index| index + 1)
                .unwrap_or(0);
            let end = normalized[marker_start..]
                .find(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '"' | '`'))
                .map(|offset| marker_start + offset)
                .unwrap_or(normalized.len());
            normalized.replace_range(prefix_start..end, "<phpt-work-path>");
        }
    }
    let thread_marker = "thread 'main' (";
    while let Some(start) = normalized.find(thread_marker) {
        let digits_start = start + thread_marker.len();
        let Some(close_offset) = normalized[digits_start..].find(')') else {
            break;
        };
        let digits_end = digits_start + close_offset;
        if normalized[digits_start..digits_end]
            .chars()
            .all(|ch| ch.is_ascii_digit())
        {
            normalized.replace_range(digits_start..digits_end, "<thread-id>");
        } else {
            break;
        }
    }
    normalized = normalize_rust_source_locations(&normalized);
    if normalized.contains("PHPT_TIMEOUT after") {
        return "PHPT_TIMEOUT".to_string();
    }
    if normalized.starts_with("output did not match expectation")
        && let Some(excerpt_start) = normalized.find(" expected=`")
    {
        normalized.truncate(excerpt_start);
        normalized.push_str(" expected=<excerpt> actual=<excerpt>");
    }
    if normalized.contains("E_PHP_IR_TRAIT_METHOD_CONFLICT") {
        let mut lines = normalized
            .lines()
            .map(|line| {
                if let Some(rest) = line.strip_prefix("stderr=") {
                    rest.to_string()
                } else if line.starts_with("target exited with status ") {
                    line.find("; stderr=")
                        .map(|offset| line[offset + "; stderr=".len()..].to_string())
                        .unwrap_or_else(|| line.to_string())
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>();
        lines.sort_unstable();
        normalized = lines.join("\n");
    }
    normalized
}

fn normalize_rust_source_locations(detail: &str) -> String {
    let mut normalized = detail.to_string();
    let mut search_start = 0;
    while let Some(marker_offset) = normalized[search_start..].find(".rs:") {
        let marker_start = search_start + marker_offset;
        let line_start = marker_start + ".rs:".len();
        let Some(line_end_offset) = normalized[line_start..].find(':') else {
            break;
        };
        let line_end = line_start + line_end_offset;
        if line_start == line_end
            || !normalized[line_start..line_end]
                .chars()
                .all(|ch| ch.is_ascii_digit())
        {
            search_start = line_start;
            continue;
        }

        let col_start = line_end + 1;
        let col_end = normalized[col_start..]
            .find(|ch: char| !ch.is_ascii_digit())
            .map(|offset| col_start + offset)
            .unwrap_or(normalized.len());
        if col_start == col_end {
            search_start = col_start;
            continue;
        }

        normalized.replace_range(line_start..col_end, "<line>:<col>");
        search_start = line_start + "<line>:<col>".len();
    }
    normalized
}

fn missing_feature_guess(result: &PhptRunResult) -> String {
    let detail = result.detail.to_ascii_lowercase();
    if result.outcome == "BORK" && detail.contains("unsupported section") {
        "phpt-runner-section".to_string()
    } else if detail.contains("phpt_timeout") {
        "runtime-timeout".to_string()
    } else if detail.contains("parse") || detail.contains("syntax") {
        "frontend-parse-or-compile".to_string()
    } else if detail.contains("unsupported") || detail.contains("not implemented") {
        "runtime-unsupported-feature".to_string()
    } else if detail.contains("target exited") {
        "runtime-error-or-diagnostic".to_string()
    } else if detail.contains("expected") || detail.contains("actual") {
        "runtime-output-mismatch".to_string()
    } else {
        "needs-triage".to_string()
    }
}

const LITERAL_KIND_UNSUPPORTED_DIAGNOSTIC: &str =
    "E_PHP_IR_UNSUPPORTED_HIR_STATEMENT: literal kind is not lowered to IR";
const ADVANCED_PARAMETER_UNFOLDED_DIAGNOSTIC: &str =
    "parameter default is not a folded Semantic frontend constant expression";

fn is_related_known_failure_evolution(
    previous: Option<&PhptRunResult>,
    current: Option<&PhptRunResult>,
) -> bool {
    let (Some(previous), Some(current)) = (previous, current) else {
        return false;
    };
    if previous.path != current.path
        || matches!(current.outcome.as_str(), "PASS" | "SKIP" | "XFAIL")
    {
        return false;
    }
    previous
        .detail
        .contains(LITERAL_KIND_UNSUPPORTED_DIAGNOSTIC)
        || previous
            .detail
            .contains(ADVANCED_PARAMETER_UNFOLDED_DIAGNOSTIC)
        || (previous
            .detail
            .starts_with("output did not match expectation")
            && current
                .detail
                .starts_with("output did not match expectation"))
}

fn render_baseline_report(
    results: &[PhptRunResult],
    failures: &[KnownFailure],
    timestamp: &str,
) -> String {
    let mut outcomes = BTreeMap::<String, usize>::new();
    for result in results {
        *outcomes.entry(result.outcome.clone()).or_default() += 1;
    }
    let mut clusters = BTreeMap::<String, usize>::new();
    for failure in failures {
        *clusters
            .entry(failure.primary_missing_feature_guess.clone())
            .or_default() += 1;
    }
    let mut modules = BTreeMap::<String, usize>::new();
    for failure in failures {
        *modules.entry(failure.module_tag.clone()).or_default() += 1;
    }

    let mut out = String::new();
    out.push_str("# PHPT Full PHPT Baseline\n\n");
    out.push_str(&format!("Generated: `{timestamp}`\n\n"));
    out.push_str("## Totals\n\n");
    out.push_str("| Outcome | Count |\n| --- | ---: |\n");
    for (outcome, count) in outcomes {
        out.push_str(&format!("| {outcome} | {count} |\n"));
    }
    out.push_str("\n## Top Failure Clusters\n\n");
    let mut cluster_counts = clusters.into_iter().collect::<Vec<_>>();
    cluster_counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    out.push_str("| Cluster | Count |\n| --- | ---: |\n");
    for (cluster, count) in cluster_counts.iter().take(20) {
        out.push_str(&format!("| {cluster} | {count} |\n"));
    }
    out.push_str("\n## Top Failing Modules\n\n");
    let mut module_counts = modules.into_iter().collect::<Vec<_>>();
    module_counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    out.push_str("| Module | Count |\n| --- | ---: |\n");
    for (module, count) in module_counts.iter().take(20) {
        out.push_str(&format!("| {module} | {count} |\n"));
    }
    out.push_str("\n## Policy\n\n");
    out.push_str(
        "Module work may reduce known failures, but must not add new failures or mutate unrelated fingerprints without explanation.\n",
    );
    out
}

fn parse_duration_seconds(value: &str) -> Result<Duration, String> {
    let seconds = value
        .parse::<u64>()
        .map_err(|_| format!("invalid duration seconds `{value}`"))?;
    if seconds == 0 {
        return Err("timeout must be greater than zero".to_string());
    }
    Ok(Duration::from_secs(seconds))
}

fn parse_usize(value: &str, name: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("invalid {name} value `{value}`"))
}

fn infer_target_mode(target: &Path) -> TargetMode {
    if target
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "php-vm")
    {
        TargetMode::PhpVm
    } else {
        TargetMode::PhpCli
    }
}

fn sanitize_env_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn render_run_summary(results: &[PhptRunResult]) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for result in results {
        *counts.entry(result.outcome.clone()).or_default() += 1;
    }
    let mut out = String::new();
    out.push_str("# PHPT Run Summary\n\n");
    out.push_str("| Outcome | Count |\n| --- | ---: |\n");
    for (outcome, count) in counts {
        out.push_str(&format!("| {outcome} | {count} |\n"));
    }
    out.push_str("\n## Non-green Results\n\n");
    for result in results {
        if !matches!(result.outcome.as_str(), "PASS" | "SKIP" | "XFAIL") {
            out.push_str(&format!(
                "- `{}`: {} - {}\n",
                result.path, result.outcome, result.detail
            ));
        }
    }
    out
}

fn collect_symbol_source_files(
    php_src: &Path,
    current: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    let mut children = fs::read_dir(current)
        .map_err(|error| format!("{}: {error}", current.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", current.display()))?;
    children.sort_by_key(|entry| entry.path());
    for child in children {
        let path = child.path();
        let file_type = child
            .file_type()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if file_type.is_dir() {
            if should_skip_dir(php_src, &path) {
                continue;
            }
            collect_symbol_source_files(php_src, &path, files)?;
        } else if file_type.is_file() {
            let rel = relative_path(php_src, &path)?;
            if is_core_source_path(&rel) && is_symbol_source_file(&rel) {
                files.push(rel);
            }
        }
    }
    Ok(())
}

fn scan_symbol_file(path: &Path, rel: &str, entries: &mut Vec<SymbolEntry>) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let source = String::from_utf8_lossy(&bytes);
    let module = module_guess(rel);
    for (index, line) in source.lines().enumerate() {
        let line_number = index as u64 + 1;
        for (macro_name, kind) in [
            ("PHP_FUNCTION", "php_function"),
            ("ZEND_FUNCTION", "zend_function"),
        ] {
            if let Some(args) = macro_args(line, macro_name) {
                let name = args.trim().to_string();
                if !name.is_empty() {
                    entries.push(SymbolEntry {
                        kind: kind.to_string(),
                        php_name: name.clone(),
                        c_name: format!("{macro_name}({name})"),
                        path: rel.to_string(),
                        line: line_number,
                        module: module.clone(),
                    });
                }
            }
        }
        for (macro_name, kind) in [("PHP_METHOD", "php_method"), ("ZEND_METHOD", "zend_method")] {
            if let Some(args) = macro_args(line, macro_name) {
                let parts = args
                    .split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>();
                if parts.len() >= 2 {
                    entries.push(SymbolEntry {
                        kind: kind.to_string(),
                        php_name: format!("{}::{}", parts[0], parts[1]),
                        c_name: format!("{macro_name}({}, {})", parts[0], parts[1]),
                        path: rel.to_string(),
                        line: line_number,
                        module: module.clone(),
                    });
                }
            }
        }
        if let Some(class_name) = init_class_entry_name(line) {
            entries.push(SymbolEntry {
                kind: "class_entry".to_string(),
                php_name: class_name.clone(),
                c_name: "INIT_CLASS_ENTRY".to_string(),
                path: rel.to_string(),
                line: line_number,
                module: module.clone(),
            });
        }
        if let Some(module_name) = module_entry_name(line) {
            entries.push(SymbolEntry {
                kind: "module_entry".to_string(),
                php_name: module_name.clone(),
                c_name: format!("{module_name}_module_entry"),
                path: rel.to_string(),
                line: line_number,
                module: module.clone(),
            });
        }
    }
    Ok(())
}

fn first_non_empty_line(body: &str) -> String {
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string()
}

fn has_section(sections: &[PhptSection], name: &str) -> bool {
    sections.iter().any(|section| section.name == name)
}

fn expectation_kind(sections: &[PhptSection]) -> String {
    for name in [
        "EXPECT",
        "EXPECTF",
        "EXPECTREGEX",
        "EXPECT_EXTERNAL",
        "EXPECTF_EXTERNAL",
        "EXPECTREGEX_EXTERNAL",
    ] {
        if has_section(sections, name) {
            return name.to_ascii_lowercase();
        }
    }
    "none".to_string()
}

fn phpt_module_tag(rel: &str, sections: &[PhptSection]) -> String {
    if rel.starts_with("Zend/") {
        return "zend".to_string();
    }
    if rel.starts_with("sapi/") {
        return "sapi".to_string();
    }
    if rel.contains("/streams/") || rel.contains("stream") {
        return "streams".to_string();
    }
    if rel.contains("filesystem") || rel.contains("/file/") || rel.contains("file_") {
        return "filesystem".to_string();
    }
    if rel.starts_with("ext/json/") {
        return "json".to_string();
    }
    if rel.starts_with("ext/pcre/") {
        return "pcre".to_string();
    }
    if rel.starts_with("ext/date/") {
        return "date".to_string();
    }
    if rel.starts_with("ext/spl/") {
        return "spl".to_string();
    }
    if rel.starts_with("ext/reflection/") {
        return "reflection".to_string();
    }
    if rel.starts_with("ext/tokenizer/") {
        return "tokenizer".to_string();
    }
    if rel.starts_with("ext/standard/") {
        let lower = rel.to_ascii_lowercase();
        if lower.contains("array") {
            return "standard.arrays".to_string();
        }
        if lower.contains("string") || lower.contains("str_") {
            return "standard.strings".to_string();
        }
        return "standard".to_string();
    }
    for section in sections {
        if section.name == "EXTENSIONS" {
            let first = section
                .body
                .split_whitespace()
                .next()
                .unwrap_or("unknown")
                .to_ascii_lowercase();
            if !first.is_empty() {
                return first;
            }
        }
    }
    "unknown".to_string()
}

fn render_phpt_summary(entries: &[PhptEntry]) -> String {
    let mut by_module = BTreeMap::<String, usize>::new();
    let mut by_expectation = BTreeMap::<String, usize>::new();
    let mut section_counts = BTreeMap::<String, usize>::new();
    let mut skipif = 0usize;
    let mut clean = 0usize;
    let mut redirect = 0usize;
    let mut external = 0usize;
    let mut http = 0usize;
    let mut stdin_args = 0usize;

    for entry in entries {
        *by_module.entry(entry.module.clone()).or_default() += 1;
        *by_expectation
            .entry(entry.expectation_kind.clone())
            .or_default() += 1;
        for section in &entry.sections {
            *section_counts.entry(section.clone()).or_default() += 1;
        }
        skipif += usize::from(entry.has_skipif);
        clean += usize::from(entry.has_clean);
        redirect += usize::from(entry.has_redirecttest);
        external += usize::from(entry.has_external_files);
        http += usize::from(entry.uses_http_sections);
        stdin_args += usize::from(entry.uses_stdin_args);
    }

    let mut out = String::new();
    out.push_str("# PHPT Corpus Summary\n\n");
    out.push_str("Generated by `just phpt-index` from the pinned php-src checkout.\n\n");
    out.push_str(&format!("- Total PHPT files: {}\n", entries.len()));
    out.push_str(&format!("- Tests with SKIPIF: {skipif}\n"));
    out.push_str(&format!("- Tests with CLEAN: {clean}\n"));
    out.push_str(&format!("- Tests with REDIRECTTEST: {redirect}\n"));
    out.push_str(&format!("- Tests with external files: {external}\n"));
    out.push_str(&format!("- Tests using HTTP-like sections: {http}\n"));
    out.push_str(&format!("- Tests using STDIN or ARGS: {stdin_args}\n\n"));
    out.push_str("## Module Tags\n\n");
    out.push_str("| Module | PHPT files |\n| --- | ---: |\n");
    for (module, count) in by_module {
        out.push_str(&format!("| {module} | {count} |\n"));
    }
    out.push_str("\n## Expectation Kinds\n\n");
    out.push_str("| Expectation | PHPT files |\n| --- | ---: |\n");
    for (kind, count) in by_expectation {
        out.push_str(&format!("| {kind} | {count} |\n"));
    }
    out.push_str("\n## Section Counts\n\n");
    out.push_str("| Section | PHPT files |\n| --- | ---: |\n");
    for (section, count) in section_counts {
        out.push_str(&format!("| {section} | {count} |\n"));
    }
    out
}

fn json_string_array(values: &[String]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&escape_json(value));
        out.push('"');
    }
    out.push(']');
    out
}

fn collect_recursive(
    php_src: &Path,
    current: &Path,
    entries: &mut Vec<ManifestEntry>,
) -> Result<(), String> {
    let mut children = fs::read_dir(current)
        .map_err(|error| format!("{}: {error}", current.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", current.display()))?;
    children.sort_by_key(|entry| entry.path());
    for child in children {
        let path = child.path();
        let file_type = child
            .file_type()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if file_type.is_dir() {
            if should_skip_dir(php_src, &path) {
                continue;
            }
            collect_recursive(php_src, &path, entries)?;
        } else if file_type.is_file() {
            let rel = relative_path(php_src, &path)?;
            if let Some(kind) = classify_relevant_file(&rel) {
                let (size, sha256) = hash_file(&path)?;
                entries.push(ManifestEntry {
                    path: rel,
                    size,
                    sha256,
                    kind,
                });
            }
        }
    }
    Ok(())
}

fn should_skip_dir(php_src: &Path, path: &Path) -> bool {
    let Ok(rel) = relative_path(php_src, path) else {
        return true;
    };
    rel == ".git"
        || rel == "autom4te.cache"
        || rel == "modules"
        || rel == "libs"
        || rel.ends_with("/.libs")
        || rel.ends_with("/autom4te.cache")
}

fn classify_relevant_file(rel: &str) -> Option<FileKind> {
    if rel == "run-tests.php" {
        return Some(FileKind::RunTests);
    }
    if rel.ends_with(".phpt") {
        return Some(FileKind::Phpt);
    }
    if !is_core_source_path(rel) {
        return None;
    }
    if rel.ends_with(".c") || rel.ends_with(".cc") {
        if rel.starts_with("Zend/") {
            Some(FileKind::ZendSource)
        } else {
            Some(FileKind::CSource)
        }
    } else if rel.ends_with(".h") {
        Some(FileKind::Header)
    } else if rel.ends_with(".inc")
        || rel.ends_with(".stub.php")
        || rel.ends_with(".php")
        || rel.ends_with(".phtml")
        || rel.ends_with(".exp")
    {
        Some(FileKind::FixtureSupport)
    } else if rel.ends_with(".re")
        || rel.ends_with(".y")
        || rel.ends_with(".l")
        || rel.ends_with(".m4")
        || rel.ends_with(".w32")
        || rel.ends_with(".md")
        || rel.ends_with(".txt")
    {
        Some(FileKind::Other)
    } else {
        None
    }
}

fn is_core_source_path(rel: &str) -> bool {
    rel.starts_with("Zend/")
        || rel.starts_with("main/")
        || rel.starts_with("ext/")
        || rel.starts_with("sapi/cli/")
}

fn is_symbol_source_file(rel: &str) -> bool {
    is_c_or_header(rel) || rel.ends_with(".stub.php")
}

fn is_c_or_header(rel: &str) -> bool {
    rel.ends_with(".c") || rel.ends_with(".h") || rel.ends_with(".cc")
}

fn macro_args(line: &str, macro_name: &str) -> Option<String> {
    let start = line.find(macro_name)?;
    let after_macro = &line[start + macro_name.len()..];
    let open = after_macro.find('(')?;
    let mut depth = 0usize;
    let mut out = String::new();
    for ch in after_macro[open..].chars() {
        if ch == '(' {
            if depth > 0 {
                out.push(ch);
            }
            depth += 1;
        } else if ch == ')' {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(out);
            }
            out.push(ch);
        } else if depth > 0 {
            out.push(ch);
        }
    }
    None
}

fn init_class_entry_name(line: &str) -> Option<String> {
    let args = macro_args(line, "INIT_CLASS_ENTRY")?;
    let first_quote = args.find('"')?;
    let rest = &args[first_quote + 1..];
    let second_quote = rest.find('"')?;
    Some(rest[..second_quote].to_string())
}

fn module_entry_name(line: &str) -> Option<String> {
    let needle = "zend_module_entry ";
    let start = line.find(needle)? + needle.len();
    let rest = &line[start..];
    let name = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    name.strip_suffix("_module_entry")
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn module_guess(rel: &str) -> String {
    if rel.starts_with("Zend/") {
        "zend".to_string()
    } else if rel.starts_with("main/") {
        "main".to_string()
    } else if rel.starts_with("sapi/cli/") {
        "sapi.cli".to_string()
    } else if let Some(rest) = rel.strip_prefix("ext/") {
        rest.split('/').next().unwrap_or("ext").to_string()
    } else {
        "unknown".to_string()
    }
}

fn source_stem(rel: &str) -> String {
    rel.rsplit('/')
        .next()
        .unwrap_or(rel)
        .split('.')
        .next()
        .unwrap_or(rel)
        .to_string()
}

fn hash_file(path: &Path) -> Result<(u64, String), String> {
    let mut file = fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    let mut buffer = [0u8; 8192];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        size += read as u64;
        hasher.update(&buffer[..read]);
    }
    Ok((size, format!("{:x}", hasher.finalize())))
}

fn default_php_src_dir() -> PathBuf {
    let preferred = PathBuf::from("third_party/php-src-8.5.7");
    if preferred.is_dir() {
        preferred
    } else {
        PathBuf::from("third_party/php-src")
    }
}

fn relative_path(root: &Path, path: &Path) -> Result<String, String> {
    let rel = path
        .strip_prefix(root)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(rel
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out
}

fn extract_json_string(line: &str, key: &str) -> Result<String, String> {
    let needle = format!("\"{key}\":\"");
    let start = line
        .find(&needle)
        .ok_or_else(|| format!("missing string field `{key}`"))?
        + needle.len();
    let mut value = String::new();
    let mut escape = false;
    for ch in line[start..].chars() {
        if escape {
            match ch {
                '"' => value.push('"'),
                '\\' => value.push('\\'),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                _ => return Err(format!("unsupported escape in field `{key}`")),
            }
            escape = false;
        } else if ch == '\\' {
            escape = true;
        } else if ch == '"' {
            return Ok(value);
        } else {
            value.push(ch);
        }
    }
    Err(format!("unterminated string field `{key}`"))
}

fn extract_json_bool(line: &str, key: &str) -> Result<bool, String> {
    let needle = format!("\"{key}\":");
    let start = line
        .find(&needle)
        .ok_or_else(|| format!("missing bool field `{key}`"))?
        + needle.len();
    if line[start..].starts_with("true") {
        Ok(true)
    } else if line[start..].starts_with("false") {
        Ok(false)
    } else {
        Err(format!("invalid bool field `{key}`"))
    }
}

fn extract_json_string_array(line: &str, key: &str) -> Result<Vec<String>, String> {
    let needle = format!("\"{key}\":[");
    let start = line
        .find(&needle)
        .ok_or_else(|| format!("missing array field `{key}`"))?
        + needle.len();
    let mut values = Vec::new();
    let mut index = start;
    loop {
        let rest = &line[index..];
        if rest.starts_with(']') {
            return Ok(values);
        }
        if !rest.starts_with('"') {
            return Err(format!("invalid array field `{key}`"));
        }
        index += 1;
        let mut value = String::new();
        let mut escape = false;
        for (offset, ch) in line[index..].char_indices() {
            if escape {
                match ch {
                    '"' => value.push('"'),
                    '\\' => value.push('\\'),
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    _ => return Err(format!("unsupported escape in array field `{key}`")),
                }
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                index += offset + 1;
                values.push(value);
                break;
            } else {
                value.push(ch);
            }
        }
        let rest = &line[index..];
        if rest.starts_with(',') {
            index += 1;
        } else if rest.starts_with(']') {
            return Ok(values);
        } else {
            return Err(format!("unterminated array field `{key}`"));
        }
    }
}

fn extract_json_u64(line: &str, key: &str) -> Result<u64, String> {
    let needle = format!("\"{key}\":");
    let start = line
        .find(&needle)
        .ok_or_else(|| format!("missing numeric field `{key}`"))?
        + needle.len();
    let digits = line[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return Err(format!("empty numeric field `{key}`"));
    }
    digits
        .parse()
        .map_err(|error| format!("invalid numeric field `{key}`: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_json_roundtrips() {
        let entry = ManifestEntry {
            path: "ext/standard/tests/file \"x\".phpt".to_string(),
            size: 12,
            sha256: "abc".to_string(),
            kind: FileKind::Phpt,
        };

        assert_eq!(
            ManifestEntry::from_json_line(&entry.to_json_line()).unwrap(),
            entry
        );
    }

    #[test]
    fn classifies_required_paths() {
        assert_eq!(
            classify_relevant_file("run-tests.php"),
            Some(FileKind::RunTests)
        );
        assert_eq!(
            classify_relevant_file("Zend/zend_execute.c"),
            Some(FileKind::ZendSource)
        );
        assert_eq!(
            classify_relevant_file("main/main.c"),
            Some(FileKind::CSource)
        );
        assert_eq!(
            classify_relevant_file("ext/standard/php_string.h"),
            Some(FileKind::Header)
        );
        assert_eq!(
            classify_relevant_file("tests/basic/001.phpt"),
            Some(FileKind::Phpt)
        );
        assert_eq!(classify_relevant_file("README.md"), None);
    }

    #[test]
    fn extracts_common_symbol_macros() {
        assert_eq!(
            macro_args("PHP_FUNCTION(strlen)", "PHP_FUNCTION").unwrap(),
            "strlen"
        );
        assert_eq!(
            macro_args("PHP_METHOD(DateTime, __construct)", "PHP_METHOD").unwrap(),
            "DateTime, __construct"
        );
        assert_eq!(
            init_class_entry_name("INIT_CLASS_ENTRY(ce, \"ArrayObject\", methods)").unwrap(),
            "ArrayObject"
        );
        assert_eq!(
            module_entry_name("zend_module_entry json_module_entry = {").unwrap(),
            "json"
        );
    }

    #[test]
    fn classifies_known_failure_evolution_as_related_changes() {
        let previous = PhptRunResult {
            path: "Zend/tests/example.phpt".to_string(),
            outcome: "FAIL".to_string(),
            detail: format!(
                "target exited with status 2; stderr={LITERAL_KIND_UNSUPPORTED_DIAGNOSTIC}"
            ),
        };
        let current = PhptRunResult {
            path: "Zend/tests/example.phpt".to_string(),
            outcome: "FAIL".to_string(),
            detail: "target exited with status 3; stderr=runtime_error: undefined function getdate"
                .to_string(),
        };
        let unrelated_path = PhptRunResult {
            path: "Zend/tests/other.phpt".to_string(),
            outcome: "FAIL".to_string(),
            detail: current.detail.clone(),
        };
        let passing_current = PhptRunResult {
            path: previous.path.clone(),
            outcome: "PASS".to_string(),
            detail: String::new(),
        };
        let previous_advanced_parameter = PhptRunResult {
            path: previous.path.clone(),
            outcome: "FAIL".to_string(),
            detail: format!(
                "target exited with status 2; stderr={ADVANCED_PARAMETER_UNFOLDED_DIAGNOSTIC}"
            ),
        };
        let previous_output = PhptRunResult {
            path: previous.path.clone(),
            outcome: "FAIL".to_string(),
            detail: "output did not match expectation first_mismatch=Some(100)".to_string(),
        };
        let current_output = PhptRunResult {
            path: previous.path.clone(),
            outcome: "FAIL".to_string(),
            detail: "output did not match expectation first_mismatch=Some(200)".to_string(),
        };

        assert!(is_related_known_failure_evolution(
            Some(&previous),
            Some(&current)
        ));
        assert!(is_related_known_failure_evolution(
            Some(&previous_advanced_parameter),
            Some(&current)
        ));
        assert!(is_related_known_failure_evolution(
            Some(&previous_output),
            Some(&current_output)
        ));
        assert!(!is_related_known_failure_evolution(
            Some(&previous),
            Some(&unrelated_path)
        ));
        assert!(!is_related_known_failure_evolution(
            Some(&previous),
            Some(&passing_current)
        ));
    }

    #[test]
    fn normalizes_run_specific_paths_for_failure_fingerprints() {
        let left = "stderr=/tmp/repo/target/phpt-work/full-runs/a/work/target/case-1-2/test.php: E\nthread 'main' (123) panicked";
        let right = "stderr=/tmp/repo/target/phpt-work/full-runs/b/work/target/case-9-8/test.php: E\nthread 'main' (456) panicked";

        assert_eq!(
            normalize_failure_detail_for_fingerprint(left),
            normalize_failure_detail_for_fingerprint(right)
        );

        let left = "thread 'main' (123) panicked at crates/php_vm/src/vm.rs:7824:37:\n             at /rustc/hash/library/std/src/panicking.rs:689:5";
        let right = "thread 'main' (456) panicked at crates/php_vm/src/vm.rs:7827:37:\n             at /rustc/hash/library/std/src/panicking.rs:701:5";

        assert_eq!(
            normalize_failure_detail_for_fingerprint(left),
            normalize_failure_detail_for_fingerprint(right)
        );

        let left = "message=\"/tmp/repo/target/phpt-work/full-runs/a/work/target/case-1-2: Is a directory\" actual=`/tmp/repo/target/phpt-work/full-runs/a/wor`";
        let right = "message=\"/tmp/repo/target/phpt-work/full-runs/b/work/target/case-9-8: Is a directory\" actual=`/tmp/repo/target/phpt-work/full-runs/b/wor`";

        assert_eq!(
            normalize_failure_detail_for_fingerprint(left),
            normalize_failure_detail_for_fingerprint(right)
        );

        let left = "output did not match expectation first_mismatch=Some(16) expected=`bool(true)` actual=`int(1782285176)`";
        let right = "output did not match expectation first_mismatch=Some(16) expected=`bool(true)` actual=`int(1782289747)`";

        assert_eq!(
            normalize_failure_detail_for_fingerprint(left),
            normalize_failure_detail_for_fingerprint(right)
        );

        assert_eq!(
            normalize_failure_detail_for_fingerprint(
                "target exited; stderr=PHPT_TIMEOUT after 10s\npartial"
            ),
            "PHPT_TIMEOUT"
        );

        let left = "stderr=<phpt-test.php>:1: E_PHP_IR_TRAIT_METHOD_CONFLICT: method b\n<phpt-test.php>:1: E_PHP_IR_TRAIT_METHOD_CONFLICT: method a";
        let right = "stderr=<phpt-test.php>:1: E_PHP_IR_TRAIT_METHOD_CONFLICT: method a\n<phpt-test.php>:1: E_PHP_IR_TRAIT_METHOD_CONFLICT: method b";

        assert_eq!(
            normalize_failure_detail_for_fingerprint(left),
            normalize_failure_detail_for_fingerprint(right)
        );
    }
}

use super::*;

pub(crate) fn run<I, W, E>(args: I, stdout: &mut W, stderr: &mut E) -> Result<i32, String>
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
        "run" => super::runner::run_phpt_manifest(&args[1..], stdout),
        "rerun-manifest" => super::runner::rerun_manifest(&args[1..], stdout),
        "baseline" => super::baseline::baseline_results(&args[1..], stdout, stderr),
        "verify-baseline" => super::baseline::verify_baseline(&args[1..], stdout, stderr),
        "triage" => triage_phpt_baseline(&args[1..], stdout),
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
        "usage: php-phpt-tools <source-index|symbol-index|lookup-symbol|phpt-index|run|rerun-manifest|baseline|verify-baseline|triage|generate|verify-source> [options]"
    )
    .map_err(|error| error.to_string())
}

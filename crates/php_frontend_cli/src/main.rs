use php_semantics::FrontendResult;
use php_semantics::query::frontend::{FrontendOptions, analyze_file};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct Args {
    command: Option<Command>,
    file: Option<PathBuf>,
    output: Option<PathBuf>,
    format: OutputFormat,
    help: bool,
    show_spans: bool,
    show_source_map: bool,
    show_deferred: bool,
    fail_on_diagnostics: bool,
    pretty: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Analyze,
    Diagnostics,
    Symbols,
    Scopes,
    Hir,
    Snapshot,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum OutputFormat {
    Text,
    #[default]
    Json,
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(error) => {
            let _ = writeln!(io::stderr(), "{}", error.message);
            std::process::exit(error.code);
        }
    }
}

fn run() -> Result<(), CliError> {
    let args = parse_args(env::args().skip(1))?;
    if args.help {
        print_usage();
        return Ok(());
    }

    let command = args
        .command
        .ok_or_else(|| CliError::usage("a command is required"))?;
    let file = args
        .file
        .ok_or_else(|| CliError::usage("a PHP source file is required"))?;
    let source = fs::read_to_string(&file)
        .map_err(|error| CliError::read(format!("failed to read {}: {error}", file.display())))?;

    let result = analyze_file(&source, &FrontendOptions::default());
    match command {
        Command::Analyze => match args.format {
            OutputFormat::Json => println!("{}", format_json(result.to_json(), args.pretty)),
            OutputFormat::Text => print_analyze_text(&file, &result),
        },
        Command::Diagnostics => match args.format {
            OutputFormat::Json => {
                println!("{}", format_json(result.to_diagnostics_json(), args.pretty))
            }
            OutputFormat::Text => print_diagnostics_text(&result),
        },
        Command::Symbols => match args.format {
            OutputFormat::Json => {
                println!("{}", format_json(result.to_symbols_json(), args.pretty))
            }
            OutputFormat::Text => print_symbols_text(&result),
        },
        Command::Scopes => match args.format {
            OutputFormat::Json => println!("{}", format_json(result.to_scopes_json(), args.pretty)),
            OutputFormat::Text => print!("{}", result.to_scopes_text()),
        },
        Command::Hir => match args.format {
            OutputFormat::Json => println!("{}", format_json(result.to_hir_json(), args.pretty)),
            OutputFormat::Text => print_hir_text(&result),
        },
        Command::Snapshot => {
            let output = args
                .output
                .ok_or_else(|| CliError::usage("snapshot requires --output <path>"))?;
            fs::write(&output, format_json(result.to_json(), true)).map_err(|error| {
                CliError::read(format!(
                    "failed to write snapshot {}: {error}",
                    output.display()
                ))
            })?;
            println!("{}", output.display());
        }
    }

    if args.fail_on_diagnostics && result.has_errors() {
        return Err(CliError::diagnostics("diagnostics were reported"));
    }

    Ok(())
}

fn parse_args<I>(args: I) -> Result<Args, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut parsed = Args {
        format: OutputFormat::Json,
        ..Args::default()
    };
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => parsed.help = true,
            "analyze" => {
                if parsed.command.is_some() {
                    return Err(CliError::usage(format!("unexpected argument: {arg}")));
                }
                parsed.command = Some(Command::Analyze);
            }
            "diagnostics" => {
                if parsed.command.is_some() {
                    return Err(CliError::usage(format!("unexpected argument: {arg}")));
                }
                parsed.command = Some(Command::Diagnostics);
            }
            "symbols" => {
                if parsed.command.is_some() {
                    return Err(CliError::usage(format!("unexpected argument: {arg}")));
                }
                parsed.command = Some(Command::Symbols);
            }
            "scopes" => {
                if parsed.command.is_some() {
                    return Err(CliError::usage(format!("unexpected argument: {arg}")));
                }
                parsed.command = Some(Command::Scopes);
            }
            "hir" => {
                if parsed.command.is_some() {
                    return Err(CliError::usage(format!("unexpected argument: {arg}")));
                }
                parsed.command = Some(Command::Hir);
            }
            "snapshot" => {
                if parsed.command.is_some() {
                    return Err(CliError::usage(format!("unexpected argument: {arg}")));
                }
                parsed.command = Some(Command::Snapshot);
            }
            "--format" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::usage("--format requires text or json"))?;
                parsed.format = match value.as_str() {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => {
                        return Err(CliError::usage(format!(
                            "unsupported --format value: {value}"
                        )));
                    }
                };
            }
            "--output" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::usage("--output requires a path"))?;
                parsed.output = Some(PathBuf::from(value));
            }
            "--php-version-target" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::usage("--php-version-target requires 8.5"))?;
                if value != php_semantics::TARGET_PHP_VERSION {
                    return Err(CliError::usage(format!(
                        "unsupported --php-version-target value: {value}"
                    )));
                }
            }
            "--show-spans" => parsed.show_spans = true,
            "--show-source-map" => parsed.show_source_map = true,
            "--show-deferred" => parsed.show_deferred = true,
            "--fail-on-diagnostics" => parsed.fail_on_diagnostics = true,
            "--pretty" => parsed.pretty = true,
            _ if arg.starts_with('-') => {
                return Err(CliError::usage(format!("unknown argument: {arg}")));
            }
            _ => {
                if parsed.file.is_some() {
                    return Err(CliError::usage(format!("unexpected argument: {arg}")));
                }
                parsed.file = Some(PathBuf::from(arg));
            }
        }
    }

    Ok(parsed)
}

fn print_usage() {
    println!(
        "Usage:\n  php-frontend analyze <file> [--format text|json]\n  php-frontend diagnostics <file> [--format text|json]\n  php-frontend symbols <file> [--format text|json]\n  php-frontend scopes <file> [--format text|json]\n  php-frontend hir <file> [--format text|json]\n  php-frontend snapshot <file> --output <path>\n\nCommands:\n  analyze      Parse and run the Semantic frontend semantic frontend.\n  diagnostics  Print parser and semantic diagnostics.\n  symbols      Print the Semantic frontend declaration table.\n  scopes       Print the Semantic frontend lexical scope tree.\n  hir          Print HIR-oriented statements, expressions, types, and declarations.\n  snapshot     Write stable analyze JSON to a snapshot file.\n\nOptions:\n  --format text|json       Output format. Defaults to json.\n  --php-version-target 8.5 Require the Semantic frontend target version.\n  --show-spans             Keep span fields in JSON/text output when available.\n  --show-source-map        Request source-map-oriented output when available.\n  --show-deferred          Keep deferred-effect metadata when available.\n  --fail-on-diagnostics    Exit 3 if parser or semantic diagnostics are reported.\n  --pretty                 Pretty-print JSON in a stable line-oriented form.\n  --help                   Show this help.\n\nExit codes:\n  0 success\n  1 I/O error\n  2 usage error\n  3 diagnostics reported with --fail-on-diagnostics"
    );
}

fn print_analyze_text(file: &Path, result: &FrontendResult) {
    println!(
        "{}: analyzed {} byte(s), parser diagnostics={}, semantic diagnostics={}",
        file.display(),
        result.module().source_bytes(),
        result.parser_diagnostics().len(),
        result.semantic_diagnostics().len()
    );
    for diagnostic in result.semantic_diagnostics() {
        println!(
            "{}\t{}\t{}",
            diagnostic.severity().as_str(),
            diagnostic.id().as_str(),
            diagnostic.message()
        );
        for note in diagnostic.notes() {
            println!("  note: {note}");
        }
    }
}

fn print_diagnostics_text(result: &FrontendResult) {
    println!("parser diagnostics={}", result.parser_diagnostics().len());
    for diagnostic in result.semantic_diagnostics() {
        println!(
            "{}\t{}\t{}",
            diagnostic.severity().as_str(),
            diagnostic.id().as_str(),
            diagnostic.message()
        );
    }
}

fn print_symbols_text(result: &php_semantics::FrontendResult) {
    if let Some(module) = result.database().module(result.module().module_id()) {
        for declaration in module.declaration_table().entries() {
            println!(
                "{}\t{}\t{}\t{}",
                declaration.decl_id().raw(),
                declaration.symbol_id().raw(),
                declaration.kind().as_str(),
                declaration
                    .fqn()
                    .canonical(declaration.kind().duplicate_name_kind())
            );
        }
    }
}

fn print_hir_text(result: &FrontendResult) {
    if let Some(module) = result.database().module(result.module().module_id()) {
        println!(
            "hir statements={} expressions={} types={} const_exprs={} class_likes={} methods={} properties={}",
            module.statements().len(),
            module.expressions().len(),
            module.types().len(),
            module.const_exprs().len(),
            module.class_likes().len(),
            module.methods().len(),
            module.properties().len()
        );
    }
}

fn format_json(json: String, pretty: bool) -> String {
    if !pretty {
        return json;
    }
    let mut out = String::with_capacity(json.len() + 32);
    let mut indent = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for ch in json.chars() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                out.push(ch);
            }
            '{' | '[' => {
                out.push(ch);
                indent += 1;
                push_newline_indent(&mut out, indent);
            }
            '}' | ']' => {
                indent = indent.saturating_sub(1);
                push_newline_indent(&mut out, indent);
                out.push(ch);
            }
            ',' => {
                out.push(ch);
                push_newline_indent(&mut out, indent);
            }
            ':' => out.push_str(": "),
            _ => out.push(ch),
        }
    }
    out
}

fn push_newline_indent(out: &mut String, indent: usize) {
    out.push('\n');
    for _ in 0..indent {
        out.push_str("  ");
    }
}

struct CliError {
    message: String,
    code: i32,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: 2,
        }
    }

    fn read(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: 1,
        }
    }

    fn diagnostics(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: 3,
        }
    }
}

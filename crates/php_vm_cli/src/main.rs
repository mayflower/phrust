//! Phase 4 VM CLI.

use php_ir::{LoweringOptions, lower_frontend_result, verify_unit};
use php_runtime::{ExitStatus, RuntimeContext};
use php_semantics::{FrontendResult, Severity, analyze_source};
use php_source::TextRange;
use php_vm::{IncludeLoader, Vm, VmOptions};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

mod todo_phase4;

const EXIT_SUCCESS: i32 = 0;
const EXIT_COMPILE_ERROR: i32 = 2;
const EXIT_RUNTIME_ERROR: i32 = 3;
const EXIT_UNSUPPORTED: i32 = 4;
const EXIT_USAGE: i32 = 5;

fn main() {
    let code = run(env::args().skip(1), &mut io::stdout(), &mut io::stderr());
    if code != EXIT_SUCCESS {
        std::process::exit(code);
    }
}

fn run<I, W, E>(args: I, stdout: &mut W, stderr: &mut E) -> i32
where
    I: IntoIterator<Item = String>,
    W: Write,
    E: Write,
{
    match run_inner(args, stdout, stderr) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(stderr, "{error}");
            EXIT_USAGE
        }
    }
}

fn run_inner<I, W, E>(args: I, stdout: &mut W, stderr: &mut E) -> Result<i32, String>
where
    I: IntoIterator<Item = String>,
    W: Write,
    E: Write,
{
    let args: Vec<String> = args.into_iter().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage(stdout)?;
        return Ok(EXIT_SUCCESS);
    }
    let Some(command) = args.first().map(String::as_str) else {
        print_usage(stdout)?;
        return Ok(EXIT_SUCCESS);
    };

    match command {
        "compile" => compile_command(&args[1..], stdout, stderr),
        "dump-ir" => dump_ir_command(&args[1..], stdout, stderr),
        "run" => run_command(&args[1..], stdout, stderr),
        "report" => report_command(&args[1..], stdout, stderr),
        "compare" => {
            writeln!(
                stderr,
                "compare is reserved until runtime-diff fixtures are implemented"
            )
            .map_err(|error| error.to_string())?;
            Ok(EXIT_UNSUPPORTED)
        }
        _ => Err(format!("unknown php-vm command `{command}`")),
    }
}

fn compile_command<W, E>(args: &[String], stdout: &mut W, stderr: &mut E) -> Result<i32, String>
where
    W: Write,
    E: Write,
{
    let (path, json) = parse_path_and_json(args)?;
    let pipeline = match compile_pipeline(path) {
        Ok(pipeline) => pipeline,
        Err(error) => {
            writeln!(stderr, "{error}").map_err(|io| io.to_string())?;
            return Ok(EXIT_COMPILE_ERROR);
        }
    };
    if json {
        writeln!(stdout, "{}", pipeline.compile_json()).map_err(|error| error.to_string())?;
    } else if pipeline.ok() {
        writeln!(
            stdout,
            "ok path={} functions={} constants={}",
            pipeline.path,
            pipeline.lowering.unit.functions.len(),
            pipeline.lowering.unit.constants.len()
        )
        .map_err(|error| error.to_string())?;
    } else {
        write_frontend_diagnostics(stderr, &pipeline)?;
        return Ok(EXIT_COMPILE_ERROR);
    }
    Ok(if pipeline.ok() {
        EXIT_SUCCESS
    } else {
        EXIT_COMPILE_ERROR
    })
}

fn dump_ir_command<W, E>(args: &[String], stdout: &mut W, stderr: &mut E) -> Result<i32, String>
where
    W: Write,
    E: Write,
{
    let options = parse_dump_ir_args(args)?;
    let path = options.path;
    let pipeline = match compile_pipeline(path) {
        Ok(pipeline) => pipeline,
        Err(error) => {
            writeln!(stderr, "{error}").map_err(|io| io.to_string())?;
            return Ok(EXIT_COMPILE_ERROR);
        }
    };
    if !pipeline.ok() {
        write_frontend_diagnostics(stderr, &pipeline)?;
        return Ok(EXIT_COMPILE_ERROR);
    }
    if options.with_source {
        writeln!(stdout, "source path={}", path).map_err(|error| error.to_string())?;
        for (index, line) in pipeline.source.lines().enumerate() {
            writeln!(stdout, "source {:04}: {}", index + 1, line)
                .map_err(|error| error.to_string())?;
        }
        writeln!(stdout, "--- ir ---").map_err(|error| error.to_string())?;
    }
    write!(stdout, "{}", pipeline.lowering.unit.to_snapshot_text())
        .map_err(|error| error.to_string())?;
    Ok(EXIT_SUCCESS)
}

fn run_command<W, E>(args: &[String], stdout: &mut W, stderr: &mut E) -> Result<i32, String>
where
    W: Write,
    E: Write,
{
    let run_options = parse_run_args(args)?;
    let path = run_options.path;
    let pipeline = match compile_pipeline(path) {
        Ok(pipeline) => pipeline,
        Err(error) => {
            writeln!(stderr, "{error}").map_err(|io| io.to_string())?;
            return Ok(EXIT_COMPILE_ERROR);
        }
    };
    if !pipeline.ok() {
        write_frontend_diagnostics(stderr, &pipeline)?;
        return Ok(EXIT_COMPILE_ERROR);
    }
    let vm = Vm::with_options(VmOptions {
        include_loader: include_loader_for(path).ok(),
        runtime_context: RuntimeContext::controlled_cli(path, run_options.script_args),
        trace: run_options.trace,
        ..VmOptions::default()
    });
    let result = vm.execute(pipeline.lowering.unit.clone());
    stdout
        .write_all(result.output.as_bytes())
        .map_err(|error| error.to_string())?;
    write_runtime_diagnostics(stderr, path, &result.diagnostics)?;
    if run_options.trace {
        write_trace(stderr, &result.trace)?;
    }
    match result.status.exit_status() {
        ExitStatus::Success => Ok(EXIT_SUCCESS),
        ExitStatus::CompileError => {
            write_status(stderr, path, &result.status)?;
            Ok(EXIT_COMPILE_ERROR)
        }
        ExitStatus::RuntimeError | ExitStatus::Fatal => {
            write_status(stderr, path, &result.status)?;
            Ok(EXIT_RUNTIME_ERROR)
        }
        ExitStatus::Unsupported => {
            write_status(stderr, path, &result.status)?;
            Ok(EXIT_UNSUPPORTED)
        }
    }
}

fn report_command<W, E>(args: &[String], stdout: &mut W, stderr: &mut E) -> Result<i32, String>
where
    W: Write,
    E: Write,
{
    let options = parse_report_args(args)?;
    let path = options.path;
    let pipeline = match compile_pipeline(path) {
        Ok(pipeline) => pipeline,
        Err(error) => {
            writeln!(stderr, "{error}").map_err(|io| io.to_string())?;
            return Ok(EXIT_COMPILE_ERROR);
        }
    };

    let vm_result = if pipeline.ok() {
        let vm = Vm::with_options(VmOptions {
            include_loader: include_loader_for(path).ok(),
            runtime_context: RuntimeContext::controlled_cli(path, Vec::new()),
            ..VmOptions::default()
        });
        Some(vm.execute(pipeline.lowering.unit.clone()))
    } else {
        None
    };

    let report = match options.format {
        ReportFormat::Markdown => render_markdown_report(&pipeline, vm_result.as_ref()),
        ReportFormat::Html => render_html_report(&pipeline, vm_result.as_ref()),
    };
    write!(stdout, "{report}").map_err(|error| error.to_string())?;

    if !pipeline.ok() {
        write_frontend_diagnostics(stderr, &pipeline)?;
        return Ok(EXIT_COMPILE_ERROR);
    }

    let Some(vm_result) = vm_result else {
        return Ok(EXIT_COMPILE_ERROR);
    };
    match vm_result.status.exit_status() {
        ExitStatus::Success => Ok(EXIT_SUCCESS),
        ExitStatus::CompileError => Ok(EXIT_COMPILE_ERROR),
        ExitStatus::RuntimeError | ExitStatus::Fatal => Ok(EXIT_RUNTIME_ERROR),
        ExitStatus::Unsupported => Ok(EXIT_UNSUPPORTED),
    }
}

struct Pipeline {
    path: String,
    source: String,
    frontend: FrontendResult,
    lowering: php_ir::LoweringResult,
}

impl Pipeline {
    fn ok(&self) -> bool {
        !self.frontend.has_errors()
            && self.lowering.diagnostics.is_empty()
            && self.lowering.verification.is_ok()
    }

    fn compile_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\"ok\":");
        out.push_str(if self.ok() { "true" } else { "false" });
        out.push_str(",\"path\":\"");
        out.push_str(&escape_json(&self.path));
        out.push_str("\",\"source_bytes\":");
        out.push_str(&self.source.len().to_string());
        out.push_str(",\"parser_diagnostics\":");
        push_parser_diagnostics_json(&mut out, &self.path, &self.frontend);
        out.push_str(",\"semantic_diagnostics\":");
        push_semantic_diagnostics_json(&mut out, &self.path, &self.frontend);
        out.push_str(",\"lowering_diagnostics\":");
        push_lowering_diagnostics_json(&mut out, &self.path, &self.lowering);
        out.push_str(",\"ir\":{");
        out.push_str("\"version\":");
        out.push_str(&self.lowering.unit.version.to_string());
        out.push_str(",\"functions\":");
        out.push_str(&self.lowering.unit.functions.len().to_string());
        out.push_str(",\"constants\":");
        out.push_str(&self.lowering.unit.constants.len().to_string());
        out.push_str(",\"verified\":");
        out.push_str(if self.lowering.verification.is_ok() {
            "true"
        } else {
            "false"
        });
        out.push_str("}}");
        out
    }
}

fn compile_pipeline(path: &str) -> Result<Pipeline, String> {
    let source = fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    let frontend = analyze_source(&source);
    let source_path = fs::canonicalize(path)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string());
    let lowering = lower_frontend_result(
        &frontend,
        LoweringOptions {
            source_path,
            source_text: Some(source.clone()),
            ..LoweringOptions::default()
        },
    );
    if !frontend.has_errors() && lowering.verification.is_ok() {
        verify_unit(&lowering.unit).map_err(|errors| {
            format!("{path}: IR verification failed: {} error(s)", errors.len())
        })?;
    }
    Ok(Pipeline {
        path: path.to_string(),
        source,
        frontend,
        lowering,
    })
}

fn include_loader_for(path: &str) -> Result<IncludeLoader, String> {
    let path = fs::canonicalize(path).map_err(|error| format!("{path}: {error}"))?;
    let root = path
        .parent()
        .ok_or_else(|| format!("{}: missing parent directory", path.display()))?;
    IncludeLoader::for_root(root.to_path_buf())
}

fn write_frontend_diagnostics<W: Write>(stderr: &mut W, pipeline: &Pipeline) -> Result<(), String> {
    for diagnostic in pipeline.frontend.parser_diagnostics() {
        write_span_line(
            stderr,
            &pipeline.path,
            diagnostic.span,
            diagnostic.id.as_str(),
            &diagnostic.message,
        )?;
    }
    for diagnostic in pipeline.frontend.semantic_diagnostics() {
        if diagnostic.severity() == Severity::Error {
            if let Some(span) = diagnostic.span() {
                write_span_line(
                    stderr,
                    &pipeline.path,
                    span,
                    diagnostic.id().as_str(),
                    diagnostic.message(),
                )?;
            } else {
                writeln!(
                    stderr,
                    "{}: {}: {}",
                    pipeline.path,
                    diagnostic.id().as_str(),
                    diagnostic.message()
                )
                .map_err(|error| error.to_string())?;
            }
        }
    }
    for diagnostic in &pipeline.lowering.diagnostics {
        writeln!(
            stderr,
            "{}:{}..{}: {}: {}",
            pipeline.path,
            diagnostic.span.start,
            diagnostic.span.end,
            diagnostic.id,
            diagnostic.message
        )
        .map_err(|error| error.to_string())?;
    }
    if let Err(errors) = &pipeline.lowering.verification {
        writeln!(
            stderr,
            "{}: IR verification failed: {} error(s)",
            pipeline.path,
            errors.len()
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn write_status<W: Write>(
    stderr: &mut W,
    path: &str,
    status: &php_runtime::ExecutionStatus,
) -> Result<(), String> {
    writeln!(stderr, "{path}: {status}").map_err(|error| error.to_string())
}

fn write_runtime_diagnostics<W: Write>(
    stderr: &mut W,
    path: &str,
    diagnostics: &[php_runtime::RuntimeDiagnostic],
) -> Result<(), String> {
    for diagnostic in diagnostics {
        writeln!(
            stderr,
            "{path}: runtime-diagnostic: {}",
            diagnostic.to_json()
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn write_trace<W: Write>(stderr: &mut W, trace: &[String]) -> Result<(), String> {
    writeln!(stderr, "vm-trace:").map_err(|error| error.to_string())?;
    for line in trace {
        writeln!(stderr, "  {line}").map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn write_span_line<W: Write>(
    stderr: &mut W,
    path: &str,
    span: TextRange,
    id: &str,
    message: &str,
) -> Result<(), String> {
    writeln!(
        stderr,
        "{}:{}..{}: {}: {}",
        path,
        span.start().to_usize(),
        span.end().to_usize(),
        id,
        message
    )
    .map_err(|error| error.to_string())
}

fn parse_path_and_json(args: &[String]) -> Result<(&str, bool), String> {
    let mut path = None;
    let mut json = false;
    for arg in args {
        if arg == "--json" {
            json = true;
        } else if path.is_none() {
            path = Some(arg.as_str());
        } else {
            return Err(format!("unexpected compile argument `{arg}`"));
        }
    }
    let Some(path) = path else {
        return Err("compile requires <path.php>".to_string());
    };
    Ok((path, json))
}

struct DumpIrOptions<'a> {
    path: &'a str,
    with_source: bool,
}

struct RunOptions<'a> {
    path: &'a str,
    script_args: Vec<String>,
    trace: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReportFormat {
    Markdown,
    Html,
}

struct ReportOptions<'a> {
    path: &'a str,
    format: ReportFormat,
}

fn parse_dump_ir_args(args: &[String]) -> Result<DumpIrOptions<'_>, String> {
    let mut path = None;
    let mut with_source = false;
    for arg in args {
        if arg == "--with-source" {
            with_source = true;
        } else if path.is_none() {
            path = Some(arg.as_str());
        } else {
            return Err(format!("unexpected dump-ir argument `{arg}`"));
        }
    }
    let Some(path) = path else {
        return Err("dump-ir requires <path.php>".to_string());
    };
    Ok(DumpIrOptions { path, with_source })
}

fn parse_run_args(args: &[String]) -> Result<RunOptions<'_>, String> {
    let Some(_) = args.first() else {
        return Err("run requires <path.php>".to_string());
    };

    let mut path = None;
    let mut trace = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--trace" => trace = true,
            "--" => {
                let Some(path) = path else {
                    return Err("run requires <path.php> before `--`".to_string());
                };
                return Ok(RunOptions {
                    path,
                    script_args: args[index + 1..].to_vec(),
                    trace,
                });
            }
            arg if path.is_none() => path = Some(arg),
            unexpected => {
                return Err(format!(
                    "unexpected run argument `{unexpected}`; pass script arguments after `--`"
                ));
            }
        }
        index += 1;
    }
    let Some(path) = path else {
        return Err("run requires <path.php>".to_string());
    };
    Ok(RunOptions {
        path,
        script_args: Vec::new(),
        trace,
    })
}

fn parse_report_args(args: &[String]) -> Result<ReportOptions<'_>, String> {
    let mut path = None;
    let mut format = ReportFormat::Markdown;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if let Some(value) = arg.strip_prefix("--format=") {
            format = parse_report_format(value)?;
        } else if arg == "--format" {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err("report --format requires markdown or html".to_string());
            };
            format = parse_report_format(value)?;
        } else if path.is_none() {
            path = Some(arg);
        } else {
            return Err(format!("unexpected report argument `{arg}`"));
        }
        index += 1;
    }
    let Some(path) = path else {
        return Err("report requires <path.php>".to_string());
    };
    Ok(ReportOptions { path, format })
}

fn parse_report_format(value: &str) -> Result<ReportFormat, String> {
    match value {
        "markdown" | "md" => Ok(ReportFormat::Markdown),
        "html" => Ok(ReportFormat::Html),
        _ => Err(format!(
            "unsupported report format `{value}`; expected markdown or html"
        )),
    }
}

fn print_usage<W: Write>(stdout: &mut W) -> Result<(), String> {
    writeln!(
        stdout,
        "Usage:\n  php-vm compile <file> [--json]\n  php-vm dump-ir <file> [--with-source]\n  php-vm run [--trace] <file> [-- arg ...]\n  php-vm report <file> [--format markdown|html]\n  php-vm compare <file>\n\nStatus:\n  {}\n  {}\n  {}\n  {}",
        php_ir::ir_skeleton_status(),
        php_runtime::runtime_skeleton_status(),
        php_vm::vm_skeleton_status(),
        todo_phase4::cli_skeleton_status()
    )
    .map_err(|error| error.to_string())
}

fn push_parser_diagnostics_json(out: &mut String, path: &str, frontend: &FrontendResult) {
    out.push('[');
    for (index, diagnostic) in frontend.parser_diagnostics().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"path\":\"");
        out.push_str(&escape_json(path));
        out.push_str("\",\"id\":\"");
        out.push_str(diagnostic.id.as_str());
        out.push_str("\",\"message\":\"");
        out.push_str(&escape_json(&diagnostic.message));
        out.push_str("\",\"span\":");
        push_range_json(out, Some(diagnostic.span));
        out.push('}');
    }
    out.push(']');
}

fn push_semantic_diagnostics_json(out: &mut String, path: &str, frontend: &FrontendResult) {
    out.push('[');
    for (index, diagnostic) in frontend.semantic_diagnostics().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"path\":\"");
        out.push_str(&escape_json(path));
        out.push_str("\",\"id\":\"");
        out.push_str(diagnostic.id().as_str());
        out.push_str("\",\"severity\":\"");
        out.push_str(diagnostic.severity().as_str());
        out.push_str("\",\"message\":\"");
        out.push_str(&escape_json(diagnostic.message()));
        out.push_str("\",\"span\":");
        push_range_json(out, diagnostic.span());
        out.push('}');
    }
    out.push(']');
}

fn push_lowering_diagnostics_json(out: &mut String, path: &str, lowering: &php_ir::LoweringResult) {
    out.push('[');
    for (index, diagnostic) in lowering.diagnostics.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"path\":\"");
        out.push_str(&escape_json(path));
        out.push_str("\",\"id\":\"");
        out.push_str(&escape_json(&diagnostic.id));
        out.push_str("\",\"message\":\"");
        out.push_str(&escape_json(&diagnostic.message));
        out.push_str("\",\"span\":{\"start\":");
        out.push_str(&diagnostic.span.start.to_string());
        out.push_str(",\"end\":");
        out.push_str(&diagnostic.span.end.to_string());
        out.push_str("}}");
    }
    out.push(']');
}

fn render_markdown_report(pipeline: &Pipeline, vm_result: Option<&php_vm::VmResult>) -> String {
    let mut out = String::new();
    out.push_str("# PHP VM Report\n\n");
    out.push_str("## Source\n\n");
    out.push_str("- Path: `");
    out.push_str(&pipeline.path);
    out.push_str("`\n");
    out.push_str("- Source bytes: ");
    out.push_str(&pipeline.source.len().to_string());
    out.push_str("\n\n");
    push_fenced_block(&mut out, "php", &pipeline.source);

    out.push_str("## Diagnostics\n\n");
    push_diagnostics_markdown(&mut out, pipeline);

    out.push_str("## HIR Summary\n\n");
    push_hir_summary_markdown(&mut out, pipeline);

    out.push_str("## IR Dump\n\n");
    push_fenced_block(&mut out, "text", &pipeline.lowering.unit.to_snapshot_text());

    out.push_str("## VM Output\n\n");
    match vm_result {
        Some(result) => push_fenced_block(&mut out, "text", &result.output.to_string_lossy()),
        None => {
            out.push_str("VM execution skipped because compile-time diagnostics are present.\n\n")
        }
    }

    out.push_str("## Runtime Diagnostics\n\n");
    push_runtime_diagnostics_markdown(&mut out, vm_result);

    out.push_str("## Known-Gap Status\n\n");
    push_known_gap_status_markdown(&mut out, pipeline, vm_result);
    out
}

fn render_html_report(pipeline: &Pipeline, vm_result: Option<&php_vm::VmResult>) -> String {
    let mut out = String::new();
    out.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n");
    out.push_str("<title>PHP VM Report</title>\n");
    out.push_str("<style>body{font-family:system-ui,sans-serif;line-height:1.4;margin:2rem;max-width:72rem}pre{background:#f5f5f5;padding:1rem;overflow:auto}code{background:#f5f5f5;padding:.1rem .2rem}</style>\n");
    out.push_str("</head>\n<body>\n");
    out.push_str("<h1>PHP VM Report</h1>\n");
    html_section_with_pre(&mut out, "Source", &pipeline.source);
    html_section_with_pre(&mut out, "Diagnostics", &diagnostics_text(pipeline));
    html_section_with_pre(&mut out, "HIR Summary", &hir_summary_text(pipeline));
    html_section_with_pre(
        &mut out,
        "IR Dump",
        &pipeline.lowering.unit.to_snapshot_text(),
    );
    let output = vm_result
        .map(|result| result.output.to_string_lossy())
        .unwrap_or_else(|| {
            "VM execution skipped because compile-time diagnostics are present.".to_string()
        });
    html_section_with_pre(&mut out, "VM Output", &output);
    html_section_with_pre(
        &mut out,
        "Runtime Diagnostics",
        &runtime_diagnostics_text(vm_result),
    );
    html_section_with_pre(
        &mut out,
        "Known-Gap Status",
        &known_gap_status_text(pipeline, vm_result),
    );
    out.push_str("</body>\n</html>\n");
    out
}

fn push_diagnostics_markdown(out: &mut String, pipeline: &Pipeline) {
    let text = diagnostics_text(pipeline);
    if text == "none" {
        out.push_str("none\n\n");
    } else {
        push_fenced_block(out, "text", &text);
    }
}

fn diagnostics_text(pipeline: &Pipeline) -> String {
    let mut lines = Vec::new();
    for diagnostic in pipeline.frontend.parser_diagnostics() {
        lines.push(format!(
            "{} {}..{} {}",
            diagnostic.id.as_str(),
            diagnostic.span.start().to_usize(),
            diagnostic.span.end().to_usize(),
            diagnostic.message
        ));
    }
    for diagnostic in pipeline.frontend.semantic_diagnostics() {
        lines.push(format!(
            "{} {:?} {}",
            diagnostic.id().as_str(),
            diagnostic.severity(),
            diagnostic.message()
        ));
    }
    for diagnostic in &pipeline.lowering.diagnostics {
        lines.push(format!(
            "{} {}..{} {}",
            diagnostic.id, diagnostic.span.start, diagnostic.span.end, diagnostic.message
        ));
    }
    if let Err(errors) = &pipeline.lowering.verification {
        lines.push(format!("IR verification failed: {} error(s)", errors.len()));
    }
    if lines.is_empty() {
        "none".to_string()
    } else {
        lines.join("\n")
    }
}

fn push_hir_summary_markdown(out: &mut String, pipeline: &Pipeline) {
    out.push_str(&hir_summary_text(pipeline));
    out.push('\n');
}

fn hir_summary_text(pipeline: &Pipeline) -> String {
    let summary = pipeline.frontend.module();
    let mut out = String::new();
    out.push_str(&format!("- Module ID: {}\n", summary.module_id().raw()));
    out.push_str(&format!("- Root kind: `{}`\n", summary.root_kind()));
    out.push_str(&format!("- Source bytes: {}\n", summary.source_bytes()));
    if let Some(module) = pipeline.frontend.database().module(summary.module_id()) {
        out.push_str(&format!("- Namespaces: {}\n", module.namespaces().len()));
        out.push_str(&format!(
            "- Declarations: {}\n",
            module.declarations().len()
        ));
        out.push_str(&format!("- Statements: {}\n", module.statements().len()));
        out.push_str(&format!("- Expressions: {}\n", module.expressions().len()));
        out.push_str(&format!("- Types: {}\n", module.types().len()));
        out.push_str(&format!(
            "- Const expressions: {}\n",
            module.const_exprs().len()
        ));
        out.push_str(&format!("- Signatures: {}\n", module.signatures().len()));
        out.push_str(&format!("- Attributes: {}\n", module.attributes().len()));
        out.push_str(&format!(
            "- Class-like declarations: {}\n",
            module.class_likes().len()
        ));
        out.push_str(&format!("- Methods: {}\n", module.methods().len()));
        out.push_str(&format!("- Properties: {}\n", module.properties().len()));
        out.push_str(&format!(
            "- Class constants: {}",
            module.class_consts().len()
        ));
    } else {
        out.push_str("- Module detail: missing from frontend database");
    }
    out
}

fn push_runtime_diagnostics_markdown(out: &mut String, vm_result: Option<&php_vm::VmResult>) {
    let text = runtime_diagnostics_text(vm_result);
    if text == "none" {
        out.push_str("none\n\n");
    } else {
        push_fenced_block(out, "json", &text);
    }
}

fn runtime_diagnostics_text(vm_result: Option<&php_vm::VmResult>) -> String {
    let Some(result) = vm_result else {
        return "not run".to_string();
    };
    if result.diagnostics.is_empty() {
        return "none".to_string();
    }
    result
        .diagnostics
        .iter()
        .map(php_runtime::RuntimeDiagnostic::to_json)
        .collect::<Vec<_>>()
        .join("\n")
}

fn push_known_gap_status_markdown(
    out: &mut String,
    pipeline: &Pipeline,
    vm_result: Option<&php_vm::VmResult>,
) {
    out.push_str(&known_gap_status_text(pipeline, vm_result));
    out.push_str("\n\n");
}

fn known_gap_status_text(pipeline: &Pipeline, vm_result: Option<&php_vm::VmResult>) -> String {
    let mut ids = Vec::new();
    for diagnostic in &pipeline.lowering.diagnostics {
        if is_known_gap_id(&diagnostic.id) {
            ids.push(diagnostic.id.clone());
        }
    }
    if let Some(result) = vm_result {
        for diagnostic in &result.diagnostics {
            if is_known_gap_id(diagnostic.id()) {
                ids.push(diagnostic.id().to_string());
            }
        }
    }
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        "none detected".to_string()
    } else {
        ids.join("\n")
    }
}

fn is_known_gap_id(id: &str) -> bool {
    id.contains("UNSUPPORTED") || id.contains("KNOWN_GAP") || id.contains("GAP")
}

fn push_fenced_block(out: &mut String, lang: &str, body: &str) {
    out.push_str("```");
    out.push_str(lang);
    out.push('\n');
    out.push_str(body);
    if !body.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("```\n\n");
}

fn html_section_with_pre(out: &mut String, title: &str, body: &str) {
    out.push_str("<section>\n<h2>");
    out.push_str(&escape_html(title));
    out.push_str("</h2>\n<pre>");
    out.push_str(&escape_html(body));
    out.push_str("</pre>\n</section>\n");
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            c => escaped.push(c),
        }
    }
    escaped
}

fn push_range_json(out: &mut String, span: Option<TextRange>) {
    match span {
        Some(span) => {
            out.push_str("{\"start\":");
            out.push_str(&span.start().to_usize().to_string());
            out.push_str(",\"end\":");
            out.push_str(&span.end().to_usize().to_string());
            out.push('}');
        }
        None => out.push_str("null"),
    }
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
            c => escaped.push(c),
        }
    }
    escaped
}

#[allow(dead_code)]
fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

#[cfg(test)]
mod tests {
    use super::{EXIT_COMPILE_ERROR, EXIT_RUNTIME_ERROR, EXIT_SUCCESS, run};
    use std::path::PathBuf;

    #[test]
    fn help_is_available() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(["--help".to_string()], &mut stdout, &mut stderr);

        assert_eq!(code, EXIT_SUCCESS);
        assert!(stderr.is_empty());
        assert!(String::from_utf8(stdout).unwrap().contains("dump-ir"));
    }

    #[test]
    fn compile_json_reports_ir_metadata() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "compile".to_string(),
                fixture("fixtures/runtime/valid/hello.php"),
                "--json".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, EXIT_SUCCESS, "{}", String::from_utf8_lossy(&stderr));
        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.contains("\"ok\":true"));
        assert!(stdout.contains("\"ir\""));
    }

    #[test]
    fn run_executes_hello_fixture() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "run".to_string(),
                fixture("fixtures/runtime/valid/hello.php"),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, EXIT_SUCCESS, "{}", String::from_utf8_lossy(&stderr));
        assert_eq!(stdout, b"hello phase4\n");
    }

    #[test]
    fn args_after_separator_initialize_argc_and_argv() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "run".to_string(),
                fixture("fixtures/runtime/valid/superglobals/argv.php"),
                "--".to_string(),
                "alpha".to_string(),
                "beta".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, EXIT_SUCCESS, "{}", String::from_utf8_lossy(&stderr));
        assert_eq!(stdout, b"3|alpha|beta\n");
    }

    #[test]
    fn args_without_separator_are_rejected() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "run".to_string(),
                fixture("fixtures/runtime/valid/superglobals/argv.php"),
                "alpha".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_ne!(code, EXIT_SUCCESS);
        assert!(stdout.is_empty());
        assert!(
            String::from_utf8(stderr)
                .unwrap()
                .contains("pass script arguments after `--`")
        );
    }

    #[test]
    fn dump_ir_prints_textual_ir() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "dump-ir".to_string(),
                fixture("fixtures/runtime/valid/hello.php"),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, EXIT_SUCCESS, "{}", String::from_utf8_lossy(&stderr));
        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.contains("ir version=1"));
        assert!(stdout.contains("echo r0"));
    }

    #[test]
    fn dump_ir_with_source_prints_source_prelude() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "dump-ir".to_string(),
                fixture("fixtures/runtime/valid/hello.php"),
                "--with-source".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, EXIT_SUCCESS, "{}", String::from_utf8_lossy(&stderr));
        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.contains("source path="));
        assert!(stdout.contains("source 0001: <?php"));
        assert!(stdout.contains("--- ir ---"));
        assert!(stdout.contains("ir version=1"));
    }

    #[test]
    fn report_markdown_contains_debug_sections() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "report".to_string(),
                fixture("fixtures/runtime/valid/hello.php"),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, EXIT_SUCCESS, "{}", String::from_utf8_lossy(&stderr));
        assert!(stderr.is_empty());
        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.contains("# PHP VM Report"));
        assert!(stdout.contains("## Source"));
        assert!(stdout.contains("## Diagnostics"));
        assert!(stdout.contains("## HIR Summary"));
        assert!(stdout.contains("## IR Dump"));
        assert!(stdout.contains("## VM Output"));
        assert!(stdout.contains("## Runtime Diagnostics"));
        assert!(stdout.contains("## Known-Gap Status"));
        assert!(stdout.contains("hello phase4"));
    }

    #[test]
    fn report_html_escapes_source_and_contains_sections() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "report".to_string(),
                fixture("fixtures/runtime/valid/hello.php"),
                "--format=html".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, EXIT_SUCCESS, "{}", String::from_utf8_lossy(&stderr));
        assert!(stderr.is_empty());
        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.contains("<!doctype html>"));
        assert!(stdout.contains("<h1>PHP VM Report</h1>"));
        assert!(stdout.contains("<h2>HIR Summary</h2>"));
        assert!(stdout.contains("&lt;?php"));
    }

    #[test]
    fn report_runtime_error_returns_runtime_error_after_rendering() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "report".to_string(),
                fixture("fixtures/runtime/invalid/errors/undefined-function.php"),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, EXIT_RUNTIME_ERROR);
        assert!(stderr.is_empty());
        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.contains("## Runtime Diagnostics"));
        assert!(stdout.contains("E_PHP_RUNTIME_UNDEFINED_FUNCTION"));
    }

    #[test]
    fn run_trace_writes_stderr_without_changing_stdout() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "run".to_string(),
                "--trace".to_string(),
                fixture("fixtures/runtime/valid/variables/assignment.php"),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, EXIT_SUCCESS, "{}", String::from_utf8_lossy(&stderr));
        assert_eq!(stdout, b"1\n");
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("vm-trace:"), "{stderr}");
        assert!(stderr.contains("function=main(0)"), "{stderr}");
        assert!(stderr.contains("output_len="), "{stderr}");
    }

    #[test]
    fn syntax_error_returns_compile_error_with_path_and_span() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "run".to_string(),
                fixture("fixtures/semantic/invalid/missing-semicolon.php"),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, EXIT_COMPILE_ERROR);
        assert!(stdout.is_empty());
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("missing-semicolon.php"));
        assert!(stderr.contains(".."));
    }

    #[test]
    fn runtime_error_writes_structured_diagnostic() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(
            [
                "run".to_string(),
                fixture("fixtures/runtime/invalid/errors/undefined-function.php"),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, EXIT_RUNTIME_ERROR);
        assert!(stdout.is_empty());
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("runtime-diagnostic:"), "{stderr}");
        assert!(
            stderr.contains("\"id\":\"E_PHP_RUNTIME_UNDEFINED_FUNCTION\""),
            "{stderr}"
        );
        assert!(
            stderr.contains("\"stack\":[{\"function\":\"main\"}]"),
            "{stderr}"
        );
    }

    fn fixture(path: &str) -> String {
        workspace_root().join(path).display().to_string()
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("crate should be under workspace crates directory")
            .to_path_buf()
    }
}

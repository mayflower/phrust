use crate::diagnostics::{
    write_frontend_diagnostics, write_runtime_diagnostics, write_vm_compile_fatal_line,
};
use crate::pipeline::compile_source;
use crate::request::{include_loader_for, runtime_context_for};
use php_diagnostics::{DebugEvent, DiagnosticLayer, DiagnosticOutputFormat, DiagnosticPhase};
use php_runtime::api::ExitStatus;
use php_vm::api::{Vm, VmOptions};
use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) const EXIT_SUCCESS: i32 = 0;
pub(crate) const EXIT_PHP_ERROR: i32 = 255;

/// Compatibility INI options for the legacy CLI-compatible execution entry point.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CliIniOptions {
    pub include_path: Option<Vec<PathBuf>>,
    pub display_errors: Option<bool>,
    pub error_reporting: Option<i64>,
    /// Raw `-d name=value` ini overrides forwarded to the runtime registry.
    pub overrides: Vec<(String, String)>,
}

/// Compatibility input for `execute_php`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineInput {
    pub source: String,
    pub source_path: String,
    pub real_path: Option<PathBuf>,
    pub script_name: String,
    pub script_args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub ini: CliIniOptions,
    pub stdin: Vec<u8>,
    pub debug: bool,
    pub debug_log: Option<PathBuf>,
    pub debug_format: DiagnosticOutputFormat,
}

/// Executes PHP through the legacy CLI-compatible engine path.
pub fn execute_php<W, E>(input: EngineInput, stdout: &mut W, stderr: &mut E) -> Result<i32, String>
where
    W: Write,
    E: Write,
{
    emit_debug_event(
        stderr,
        &input,
        "D_PHRUST_FRONTEND_ANALYZE_START",
        DiagnosticLayer::executor(),
        "frontend",
        "frontend analysis started",
        BTreeMap::from([("path".to_string(), input.source_path.clone())]),
    )?;
    let pipeline = compile_source(&input.source, &input.source_path)?;
    if !pipeline.ok() {
        write_frontend_diagnostics(stderr, &pipeline)?;
        return Ok(EXIT_PHP_ERROR);
    }
    emit_debug_event(
        stderr,
        &input,
        "D_PHRUST_FRONTEND_ANALYZE_END",
        DiagnosticLayer::executor(),
        "frontend",
        "frontend analysis completed",
        BTreeMap::from([
            (
                "parser_diagnostic_count".to_string(),
                pipeline.frontend.parser_diagnostics().len().to_string(),
            ),
            (
                "semantic_diagnostic_count".to_string(),
                pipeline.frontend.semantic_diagnostics().len().to_string(),
            ),
        ]),
    )?;
    emit_debug_event(
        stderr,
        &input,
        "D_PHRUST_IR_LOWER_END",
        DiagnosticLayer::ir(),
        "lower",
        "IR lowering completed",
        BTreeMap::from([
            (
                "lowering_diagnostic_count".to_string(),
                pipeline.lowering.diagnostics.len().to_string(),
            ),
            (
                "function_count".to_string(),
                pipeline.lowering.unit.functions.len().to_string(),
            ),
        ]),
    )?;
    let include_loader = include_loader_for(&input)?;
    let runtime_context = runtime_context_for(&input, include_loader.as_ref());
    let vm = Vm::with_options(VmOptions {
        include_loader,
        runtime_context,
        trace: input.debug,
        trace_runtime: input.debug,
        trace_includes: input.debug,
        ..VmOptions::default()
    });
    emit_debug_event(
        stderr,
        &input,
        "D_PHRUST_VM_EXECUTE_START",
        DiagnosticLayer::vm(),
        "execute",
        "VM execution started",
        BTreeMap::from([("path".to_string(), input.source_path.clone())]),
    )?;
    let result = vm.execute(pipeline.lowering.unit.clone());
    emit_debug_event(
        stderr,
        &input,
        "D_PHRUST_VM_EXECUTE_END",
        DiagnosticLayer::vm(),
        "execute",
        "VM execution completed",
        BTreeMap::from([
            ("status".to_string(), result.status.to_string()),
            (
                "runtime_diagnostic_count".to_string(),
                result.diagnostics.len().to_string(),
            ),
        ]),
    )?;
    for (index, line) in result.trace.iter().enumerate() {
        emit_debug_event(
            stderr,
            &input,
            "D_PHRUST_VM_TRACE",
            DiagnosticLayer::vm(),
            "execute",
            "VM trace event",
            BTreeMap::from([
                ("index".to_string(), index.to_string()),
                ("trace".to_string(), line.clone()),
            ]),
        )?;
    }
    stdout
        .write_all(result.output.as_bytes())
        .map_err(|error| error.to_string())?;
    match result.status.exit_status() {
        ExitStatus::Success => Ok(EXIT_SUCCESS),
        ExitStatus::CompileError => {
            if write_vm_compile_fatal_line(stderr, &pipeline, &result.diagnostics)? {
                return Ok(EXIT_PHP_ERROR);
            }
            write_runtime_diagnostics(stderr, &input.source_path, &result.diagnostics)?;
            writeln!(stderr, "{}: {}", input.source_path, result.status)
                .map_err(|error| error.to_string())?;
            Ok(EXIT_PHP_ERROR)
        }
        ExitStatus::RuntimeError | ExitStatus::Fatal | ExitStatus::Unsupported => {
            write_runtime_diagnostics(stderr, &input.source_path, &result.diagnostics)?;
            writeln!(stderr, "{}: {}", input.source_path, result.status)
                .map_err(|error| error.to_string())?;
            Ok(EXIT_PHP_ERROR)
        }
    }
}

pub fn read_script(path: &Path) -> Result<(String, PathBuf, String), String> {
    let source = fs::read_to_string(path).map_err(|error| {
        let cwd = std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|cwd_error| format!("<unavailable: {cwd_error}>"));
        format!(
            "read source file failed for path `{}` from cwd `{cwd}`: {error}; suggestion: check that the file exists and is readable",
            path.display()
        )
    })?;
    let real_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let source_path = real_path.to_string_lossy().into_owned();
    Ok((source, real_path, source_path))
}

fn emit_debug_event<W: Write>(
    stderr: &mut W,
    input: &EngineInput,
    code: &str,
    layer: DiagnosticLayer,
    phase: &str,
    message: &str,
    context: BTreeMap<String, String>,
) -> Result<(), String> {
    if !input.debug {
        return Ok(());
    }
    let event =
        DebugEvent::new(code, layer, DiagnosticPhase::new(phase), message).with_context(context);
    let rendered = match input.debug_format {
        DiagnosticOutputFormat::Text => {
            let mut line = event.text_line();
            line.push('\n');
            line
        }
        DiagnosticOutputFormat::Json => event.json_line().map_err(|error| error.to_string())?,
    };
    if let Some(path) = input.debug_log.as_ref() {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        file.write_all(rendered.as_bytes())
            .map_err(|error| format!("{}: {error}", path.display()))
    } else {
        stderr
            .write_all(rendered.as_bytes())
            .map_err(|error| error.to_string())
    }
}

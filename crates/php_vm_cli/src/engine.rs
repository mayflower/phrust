use php_ir::{LoweringOptions, lower_frontend_result, verify_unit};
use php_runtime::{ErrorReporting, ExitStatus, FilesystemCapabilities, RuntimeContext};
use php_semantics::{FrontendResult, Severity, analyze_source};
use php_source::TextRange;
use php_vm::{IncludeLoader, Vm, VmOptions};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const EXIT_SUCCESS: i32 = 0;
const EXIT_PHP_ERROR: i32 = 255;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CliIniOptions {
    pub include_path: Option<Vec<PathBuf>>,
    pub display_errors: Option<bool>,
    pub error_reporting: Option<i64>,
}

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
}

pub fn execute_php<W, E>(input: EngineInput, stdout: &mut W, stderr: &mut E) -> Result<i32, String>
where
    W: Write,
    E: Write,
{
    let pipeline = compile_source(&input.source, &input.source_path)?;
    if !pipeline.ok() {
        write_frontend_diagnostics(stderr, &pipeline)?;
        return Ok(EXIT_PHP_ERROR);
    }
    let include_loader = include_loader_for(&input)?;
    let runtime_context = runtime_context_for(&input, include_loader.as_ref());
    let vm = Vm::with_options(VmOptions {
        include_loader,
        runtime_context,
        ..VmOptions::default()
    });
    let result = vm.execute(pipeline.lowering.unit);
    stdout
        .write_all(result.output.as_bytes())
        .map_err(|error| error.to_string())?;
    match result.status.exit_status() {
        ExitStatus::Success => Ok(EXIT_SUCCESS),
        ExitStatus::CompileError
        | ExitStatus::RuntimeError
        | ExitStatus::Fatal
        | ExitStatus::Unsupported => {
            write_runtime_diagnostics(stderr, &input.source_path, &result.diagnostics)?;
            writeln!(stderr, "{}: {}", input.source_path, result.status)
                .map_err(|error| error.to_string())?;
            Ok(EXIT_PHP_ERROR)
        }
    }
}

struct Pipeline {
    path: String,
    frontend: FrontendResult,
    lowering: php_ir::LoweringResult,
}

impl Pipeline {
    fn ok(&self) -> bool {
        !self.frontend.has_errors()
            && self.lowering.diagnostics.is_empty()
            && self.lowering.verification.is_ok()
    }
}

fn compile_source(source: &str, source_path: &str) -> Result<Pipeline, String> {
    let frontend = analyze_source(source);
    let mut lowering = lower_frontend_result(
        &frontend,
        LoweringOptions {
            source_path: source_path.to_string(),
            source_text: Some(source.to_string()),
            ..LoweringOptions::default()
        },
    );
    if !frontend.has_errors() && lowering.verification.is_ok() {
        verify_unit(&lowering.unit).map_err(|errors| {
            format!(
                "{source_path}: IR verification failed: {} error(s)",
                errors.len()
            )
        })?;
        lowering.verification = verify_unit(&lowering.unit);
    }
    Ok(Pipeline {
        path: source_path.to_string(),
        frontend,
        lowering,
    })
}

fn include_loader_for(input: &EngineInput) -> Result<Option<IncludeLoader>, String> {
    let mut roots = Vec::new();
    push_existing_root(&mut roots, &input.cwd);
    if let Some(real_path) = input.real_path.as_ref().and_then(|path| path.parent()) {
        push_existing_root(&mut roots, real_path);
    }
    if let Some(include_path) = &input.ini.include_path {
        for entry in include_path {
            if entry.is_absolute() {
                push_existing_root(&mut roots, entry);
            } else {
                push_existing_root(&mut roots, &input.cwd.join(entry));
                if let Some(real_path) = input.real_path.as_ref().and_then(|path| path.parent()) {
                    push_existing_root(&mut roots, &real_path.join(entry));
                }
            }
        }
    }
    if roots.is_empty() {
        return Ok(None);
    }
    IncludeLoader::new(roots).map(Some)
}

fn push_existing_root(roots: &mut Vec<PathBuf>, path: &Path) {
    if path.exists() {
        roots.push(path.to_path_buf());
    }
}

fn runtime_context_for(
    input: &EngineInput,
    include_loader: Option<&IncludeLoader>,
) -> RuntimeContext {
    let include_path = input
        .ini
        .include_path
        .clone()
        .unwrap_or_else(|| vec![PathBuf::from(".")]);
    let mut context =
        RuntimeContext::controlled_cli(input.script_name.clone(), input.script_args.clone())
            .with_cwd(input.cwd.clone())
            .with_include_path(include_path)
            .with_env(input.env.clone())
            .with_stdin(input.stdin.clone());
    if let Some(mask) = input.ini.error_reporting {
        context.ini.error_reporting = ErrorReporting { mask };
    }
    if let Some(display_errors) = input.ini.display_errors {
        context.ini.display_errors = display_errors;
    }
    let mut capabilities = FilesystemCapabilities::none().with_stdio(true);
    if let Some(loader) = include_loader {
        capabilities = capabilities.with_allowed_roots(loader.allowed_roots().to_vec());
    }
    context.with_filesystem_capabilities(capabilities)
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

pub fn read_script(path: &Path) -> Result<(String, PathBuf, String), String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let real_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let source_path = real_path.to_string_lossy().into_owned();
    Ok((source, real_path, source_path))
}

//! Shared structured diagnostics and debug event primitives.
//!
//! This crate is intentionally below the lexer, parser, semantic frontend, IR,
//! runtime, VM, CLI, and server layers. It provides stable envelopes, source
//! location helpers, redaction, rendering, and a tiny debug sink without taking
//! dependencies on those higher layers.

use php_source::{BytePos, SourceText, TextRange};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Version of the diagnostic/debug JSON schema emitted by this crate.
pub const SCHEMA_VERSION: u8 = 1;

/// Redacted replacement value for secret-bearing diagnostic context keys.
pub const REDACTED_VALUE: &str = "[redacted]";

/// Top-level envelope kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticKind {
    Diagnostic,
    DebugEvent,
}

impl DiagnosticKind {
    /// Returns the stable serialized name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Diagnostic => "diagnostic",
            Self::DebugEvent => "debug_event",
        }
    }
}

/// Diagnostic severity shared by all layers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Notice,
    Deprecation,
    Note,
    RecoverableError,
    FatalError,
    UnsupportedFeature,
    Info,
    Debug,
}

impl DiagnosticSeverity {
    /// Returns the stable serialized name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Notice => "notice",
            Self::Deprecation => "deprecation",
            Self::Note => "note",
            Self::RecoverableError => "recoverable_error",
            Self::FatalError => "fatal_error",
            Self::UnsupportedFeature => "unsupported_feature",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Diagnostic layer name.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiagnosticLayer(String);

impl DiagnosticLayer {
    /// Creates a layer from a stable name.
    #[must_use]
    pub fn new(layer: impl Into<String>) -> Self {
        Self(layer.into())
    }

    /// Returns the layer name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Lexer layer.
    #[must_use]
    pub fn lexer() -> Self {
        Self::new("lexer")
    }

    /// Parser layer.
    #[must_use]
    pub fn parser() -> Self {
        Self::new("parser")
    }

    /// Semantic frontend layer.
    #[must_use]
    pub fn semantic() -> Self {
        Self::new("semantic")
    }

    /// IR layer.
    #[must_use]
    pub fn ir() -> Self {
        Self::new("ir")
    }

    /// Optimizer layer.
    #[must_use]
    pub fn optimizer() -> Self {
        Self::new("optimizer")
    }

    /// Runtime layer.
    #[must_use]
    pub fn runtime() -> Self {
        Self::new("runtime")
    }

    /// Builtin/module layer.
    #[must_use]
    pub fn builtin() -> Self {
        Self::new("builtin")
    }

    /// VM layer.
    #[must_use]
    pub fn vm() -> Self {
        Self::new("vm")
    }

    /// Executor layer.
    #[must_use]
    pub fn executor() -> Self {
        Self::new("executor")
    }

    /// CLI layer.
    #[must_use]
    pub fn cli() -> Self {
        Self::new("cli")
    }

    /// Server layer.
    #[must_use]
    pub fn server() -> Self {
        Self::new("server")
    }

    /// Infrastructure/config/bootstrap layer.
    #[must_use]
    pub fn infrastructure() -> Self {
        Self::new("infrastructure")
    }
}

impl fmt::Display for DiagnosticLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Diagnostic phase name.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiagnosticPhase(String);

impl DiagnosticPhase {
    /// Creates a phase from a stable name.
    #[must_use]
    pub fn new(phase: impl Into<String>) -> Self {
        Self(phase.into())
    }

    /// Returns the phase name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiagnosticPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Half-open byte span in source text.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticSpan {
    pub start: usize,
    pub end: usize,
}

impl DiagnosticSpan {
    /// Creates a half-open byte span.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Creates a span from a source range.
    #[must_use]
    pub const fn from_range(range: TextRange) -> Self {
        Self {
            start: range.start().to_usize(),
            end: range.end().to_usize(),
        }
    }

    /// Converts to a source range, clamping invalid ordering as `TextRange` does.
    #[must_use]
    pub const fn to_range(self) -> TextRange {
        TextRange::new(self.start, self.end)
    }
}

/// Source location with byte span as the authoritative coordinate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<DiagnosticSpan>,
}

impl DiagnosticLocation {
    /// Creates a location without source-line derivation.
    #[must_use]
    pub fn new(
        path: Option<impl Into<String>>,
        source_id: Option<impl Into<String>>,
        span: Option<DiagnosticSpan>,
    ) -> Self {
        Self {
            path: path.map(Into::into),
            source_id: source_id.map(Into::into),
            line: None,
            column: None,
            span,
        }
    }

    /// Creates a location from a `php_source` byte range.
    #[must_use]
    pub fn from_source_range(
        path: Option<impl Into<String>>,
        source_id: Option<impl Into<String>>,
        source: &SourceText,
        range: TextRange,
    ) -> Self {
        let line_col = source.line_col(range.start());
        Self {
            path: path.map(Into::into),
            source_id: source_id.map(Into::into),
            line: Some(line_col.line),
            column: Some(line_col.column),
            span: Some(DiagnosticSpan::from_range(range)),
        }
    }

    /// Creates a location from a single source position.
    #[must_use]
    pub fn from_source_pos(
        path: Option<impl Into<String>>,
        source_id: Option<impl Into<String>>,
        source: &SourceText,
        pos: BytePos,
    ) -> Self {
        Self::from_source_range(path, source_id, source, TextRange::empty(pos.to_usize()))
    }
}

/// Additional source label for a diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticLabel {
    pub message: String,
    pub span: DiagnosticSpan,
}

impl DiagnosticLabel {
    /// Creates a source label.
    #[must_use]
    pub fn new(message: impl Into<String>, span: DiagnosticSpan) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

/// Underlying cause metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticCause {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

impl DiagnosticCause {
    /// Creates cause metadata.
    #[must_use]
    pub fn new(message: impl Into<String>, kind: Option<impl Into<String>>) -> Self {
        Self {
            message: message.into(),
            kind: kind.map(Into::into),
        }
    }
}

/// Optional human-facing suggestion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiagnosticSuggestion(String);

impl DiagnosticSuggestion {
    /// Creates a suggestion.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    /// Returns suggestion text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Structured diagnostic envelope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticEnvelope {
    pub kind: DiagnosticKind,
    pub schema_version: u8,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_id: Option<String>,
    pub layer: DiagnosticLayer,
    pub phase: DiagnosticPhase,
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<DiagnosticLocation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<DiagnosticLabel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<DiagnosticSuggestion>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<DiagnosticCause>,
    pub php_visible: bool,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
}

impl DiagnosticEnvelope {
    /// Creates a diagnostic envelope with stable defaults.
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        layer: DiagnosticLayer,
        phase: DiagnosticPhase,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind: DiagnosticKind::Diagnostic,
            schema_version: SCHEMA_VERSION,
            code: code.into(),
            legacy_id: None,
            layer,
            phase,
            severity,
            message: message.into(),
            location: None,
            labels: Vec::new(),
            notes: Vec::new(),
            suggestion: None,
            context: BTreeMap::new(),
            cause: None,
            php_visible: false,
            request_id: None,
            trace_id: None,
        }
    }

    /// Adds source location metadata.
    #[must_use]
    pub fn with_location(mut self, location: DiagnosticLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Adds redacted context metadata.
    #[must_use]
    pub fn with_context(mut self, context: BTreeMap<String, String>) -> Self {
        self.context = redact_context(&context);
        self
    }

    /// Renders compact JSON.
    pub fn compact_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Renders compact JSON plus a trailing newline.
    pub fn json_line(&self) -> serde_json::Result<String> {
        let mut line = self.compact_json()?;
        line.push('\n');
        Ok(line)
    }

    /// Renders a deterministic one-line text diagnostic.
    #[must_use]
    pub fn text_line(&self) -> String {
        let mut fields = Vec::new();
        fields.push(format!("layer={}", self.layer));
        fields.push(format!("phase={}", self.phase));
        fields.push(format!("severity={}", self.severity));

        if let Some(location) = &self.location {
            push_location_fields(&mut fields, location);
        }
        if let Some(legacy_id) = &self.legacy_id {
            fields.push(format!("legacy_id={}", text_atom(legacy_id)));
        }
        if let Some(request_id) = &self.request_id {
            fields.push(format!("request_id={}", text_atom(request_id)));
        }
        if let Some(trace_id) = &self.trace_id {
            fields.push(format!("trace_id={}", text_atom(trace_id)));
        }
        for (key, value) in &self.context {
            fields.push(format!("{}={}", text_atom(key), text_atom(value)));
        }

        let mut line = format!(
            "{} {}: {}",
            self.code,
            fields.join(" "),
            text_message(&self.message)
        );
        if let Some(cause) = &self.cause {
            line.push_str("; cause=");
            line.push_str(&text_message(&cause.message));
        }
        if let Some(suggestion) = &self.suggestion {
            line.push_str("; suggestion=");
            line.push_str(&text_message(suggestion.as_str()));
        }
        line
    }
}

/// Debug output mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugMode {
    pub enabled: bool,
    pub format: DiagnosticOutputFormat,
    pub path: Option<PathBuf>,
}

impl DebugMode {
    /// Creates disabled debug mode.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            format: DiagnosticOutputFormat::Text,
            path: None,
        }
    }

    /// Creates stderr debug mode.
    #[must_use]
    pub const fn stderr(format: DiagnosticOutputFormat) -> Self {
        Self {
            enabled: true,
            format,
            path: None,
        }
    }

    /// Creates file debug mode.
    #[must_use]
    pub fn file(path: impl Into<PathBuf>, format: DiagnosticOutputFormat) -> Self {
        Self {
            enabled: true,
            format,
            path: Some(path.into()),
        }
    }
}

/// Diagnostic/debug output format.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticOutputFormat {
    Text,
    Json,
}

impl DiagnosticOutputFormat {
    /// Returns the stable serialized name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
        }
    }
}

impl fmt::Display for DiagnosticOutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DiagnosticOutputFormat {
    type Err = DiagnosticFormatParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "text" => Ok(Self::Text),
            "json" | "jsonl" => Ok(Self::Json),
            _ => Err(DiagnosticFormatParseError {
                value: value.to_owned(),
            }),
        }
    }
}

/// Error returned when parsing a diagnostic output format.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFormatParseError {
    value: String,
}

impl fmt::Display for DiagnosticFormatParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown diagnostic output format `{}`", self.value)
    }
}

impl std::error::Error for DiagnosticFormatParseError {}

/// Structured debug event.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DebugEvent {
    pub kind: DiagnosticKind,
    pub schema_version: u8,
    pub code: String,
    pub layer: DiagnosticLayer,
    pub phase: DiagnosticPhase,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<DiagnosticLocation>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, String>,
}

impl DebugEvent {
    /// Creates a debug event.
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        layer: DiagnosticLayer,
        phase: DiagnosticPhase,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind: DiagnosticKind::DebugEvent,
            schema_version: SCHEMA_VERSION,
            code: code.into(),
            layer,
            phase,
            message: message.into(),
            timestamp_ms: None,
            duration_ms: None,
            request_id: None,
            trace_id: None,
            location: None,
            context: BTreeMap::new(),
        }
    }

    /// Adds redacted context metadata.
    #[must_use]
    pub fn with_context(mut self, context: BTreeMap<String, String>) -> Self {
        self.context = redact_context(&context);
        self
    }

    /// Renders compact JSON.
    pub fn compact_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Renders compact JSON plus a trailing newline.
    pub fn json_line(&self) -> serde_json::Result<String> {
        let mut line = self.compact_json()?;
        line.push('\n');
        Ok(line)
    }

    /// Renders a deterministic one-line text event.
    #[must_use]
    pub fn text_line(&self) -> String {
        let mut fields = Vec::new();
        fields.push(format!("layer={}", self.layer));
        fields.push(format!("phase={}", self.phase));

        if let Some(timestamp_ms) = self.timestamp_ms {
            fields.push(format!("timestamp_ms={timestamp_ms}"));
        }
        if let Some(duration_ms) = self.duration_ms {
            fields.push(format!("duration_ms={duration_ms}"));
        }
        if let Some(request_id) = &self.request_id {
            fields.push(format!("request_id={}", text_atom(request_id)));
        }
        if let Some(trace_id) = &self.trace_id {
            fields.push(format!("trace_id={}", text_atom(trace_id)));
        }
        if let Some(location) = &self.location {
            push_location_fields(&mut fields, location);
        }
        for (key, value) in &self.context {
            fields.push(format!("{}={}", text_atom(key), text_atom(value)));
        }

        format!(
            "{} {}: {}",
            self.code,
            fields.join(" "),
            text_message(&self.message)
        )
    }
}

/// Debug sink target and renderer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugSink {
    target: DebugSinkTarget,
    format: DiagnosticOutputFormat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DebugSinkTarget {
    Disabled,
    Stderr,
    File(PathBuf),
}

impl DebugSink {
    /// Creates a disabled sink.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            target: DebugSinkTarget::Disabled,
            format: DiagnosticOutputFormat::Text,
        }
    }

    /// Creates a stderr sink.
    #[must_use]
    pub const fn stderr(format: DiagnosticOutputFormat) -> Self {
        Self {
            target: DebugSinkTarget::Stderr,
            format,
        }
    }

    /// Creates a file sink.
    #[must_use]
    pub fn file(path: impl Into<PathBuf>, format: DiagnosticOutputFormat) -> Self {
        Self {
            target: DebugSinkTarget::File(path.into()),
            format,
        }
    }

    /// Creates a sink from a debug mode.
    #[must_use]
    pub fn from_mode(mode: &DebugMode) -> Self {
        if !mode.enabled {
            return Self::disabled();
        }
        match &mode.path {
            Some(path) => Self::file(path, mode.format),
            None => Self::stderr(mode.format),
        }
    }

    /// Returns whether this sink will write events.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        !matches!(self.target, DebugSinkTarget::Disabled)
    }

    /// Writes one debug event to the configured target.
    pub fn write_event(&self, event: &DebugEvent) -> io::Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        let rendered = match self.format {
            DiagnosticOutputFormat::Text => {
                let mut line = event.text_line();
                line.push('\n');
                line
            }
            DiagnosticOutputFormat::Json => event.json_line().map_err(io::Error::other)?,
        };

        match &self.target {
            DebugSinkTarget::Disabled => Ok(()),
            DebugSinkTarget::Stderr => io::stderr().lock().write_all(rendered.as_bytes()),
            DebugSinkTarget::File(path) => append_to_file(path, rendered.as_bytes()),
        }
    }
}

/// Returns true when a context/header key carries secrets.
#[must_use]
pub fn is_secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    matches!(
        key.as_str(),
        "authorization" | "cookie" | "set-cookie" | "x-phrust-metrics-token"
    ) || key.contains("password")
        || key.contains("token")
        || key.contains("secret")
}

/// Redacts a value when its key is secret-bearing.
#[must_use]
pub fn redact_value_for_key(key: &str, value: &str) -> String {
    if is_secret_key(key) {
        REDACTED_VALUE.to_owned()
    } else {
        value.to_owned()
    }
}

/// Returns a context map with secret-bearing values redacted.
#[must_use]
pub fn redact_context(context: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    context
        .iter()
        .map(|(key, value)| (key.clone(), redact_value_for_key(key, value)))
        .collect()
}

/// Creates a process-boundary panic diagnostic.
#[must_use]
pub fn internal_panic_diagnostic(bin_name: &str, info: &PanicHookInfo<'_>) -> DiagnosticEnvelope {
    let panic_message = if let Some(message) = info.payload().downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = info.payload().downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    };
    let mut context = BTreeMap::from([
        ("bin".to_string(), bin_name.to_string()),
        ("panic_message".to_string(), panic_message.clone()),
        (
            "rust_backtrace".to_string(),
            std::env::var("RUST_BACKTRACE").unwrap_or_else(|_| "unset".to_string()),
        ),
    ]);
    if let Some(thread_name) = std::thread::current().name() {
        context.insert("thread".to_string(), thread_name.to_string());
    }
    if let Some(location) = info.location() {
        context.insert("file".to_string(), location.file().to_string());
        context.insert("line".to_string(), location.line().to_string());
        context.insert("column".to_string(), location.column().to_string());
    }
    let mut envelope = DiagnosticEnvelope::new(
        "E_PHRUST_INTERNAL_PANIC",
        DiagnosticLayer::infrastructure(),
        DiagnosticPhase::new("panic"),
        DiagnosticSeverity::FatalError,
        format!("internal panic in {bin_name}: {panic_message}"),
    )
    .with_context(context);
    envelope.suggestion = Some(DiagnosticSuggestion::new(
        "rerun with RUST_BACKTRACE=1 and preserve stderr/debug logs for triage",
    ));
    envelope
}

/// Installs a panic hook that emits `E_PHRUST_INTERNAL_PANIC` to stderr.
pub fn install_panic_diagnostic_hook(bin_name: &'static str, format: DiagnosticOutputFormat) {
    std::panic::set_hook(Box::new(move |info| {
        let diagnostic = internal_panic_diagnostic(bin_name, info);
        let rendered = match format {
            DiagnosticOutputFormat::Text => {
                let mut line = diagnostic.text_line();
                line.push('\n');
                line
            }
            DiagnosticOutputFormat::Json => diagnostic.json_line().unwrap_or_else(|error| {
                format!(
                    "E_PHRUST_INTERNAL_PANIC layer=infrastructure phase=panic severity=fatal_error: failed to render panic diagnostic; cause={error}\n"
                )
            }),
        };
        let _ = io::stderr().lock().write_all(rendered.as_bytes());
    }));
}

fn append_to_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(bytes)
}

fn push_location_fields(fields: &mut Vec<String>, location: &DiagnosticLocation) {
    if let Some(path) = &location.path {
        fields.push(format!("path={}", text_atom(path)));
    }
    if let Some(source_id) = &location.source_id {
        fields.push(format!("source_id={}", text_atom(source_id)));
    }
    if let Some(line) = location.line {
        fields.push(format!("line={line}"));
    }
    if let Some(column) = location.column {
        fields.push(format!("col={column}"));
    }
    if let Some(span) = location.span {
        fields.push(format!("span={}..{}", span.start, span.end));
    }
}

fn text_atom(value: &str) -> String {
    let value = text_message(value);
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'))
    {
        value
    } else {
        serde_json::to_string(&value).expect("string serialization cannot fail")
    }
}

fn text_message(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::fs;

    #[test]
    fn json_rendering_escapes_strings_and_round_trips() {
        let diagnostic = DiagnosticEnvelope::new(
            "E_PHRUST_ESCAPE",
            DiagnosticLayer::parser(),
            DiagnosticPhase::new("parse"),
            DiagnosticSeverity::Error,
            "quote \" slash \\ newline\n",
        );

        let rendered = diagnostic.compact_json().expect("json renders");
        let decoded: Value = serde_json::from_str(&rendered).expect("valid json");

        assert_eq!(decoded["message"], "quote \" slash \\ newline\n");
        assert_eq!(decoded["kind"], "diagnostic");
        assert_eq!(decoded["schema_version"], SCHEMA_VERSION);
    }

    #[test]
    fn text_rendering_is_one_line_and_includes_stable_fields() {
        let mut diagnostic = DiagnosticEnvelope::new(
            "E_PHRUST_TEXT",
            DiagnosticLayer::lexer(),
            DiagnosticPhase::new("scan"),
            DiagnosticSeverity::FatalError,
            "bad\ninput",
        );
        diagnostic.request_id = Some("req 1".to_owned());

        let rendered = diagnostic.text_line();

        assert!(!rendered.contains('\n'));
        assert!(rendered.contains("E_PHRUST_TEXT"));
        assert!(rendered.contains("layer=lexer"));
        assert!(rendered.contains("phase=scan"));
        assert!(rendered.contains("severity=fatal_error"));
        assert!(rendered.contains("request_id=\"req 1\""));
        assert!(rendered.ends_with(": bad input"));
    }

    #[test]
    fn source_location_derives_one_based_line_and_byte_column() {
        let source = SourceText::new("<?php\nécho();\n");
        let location = DiagnosticLocation::from_source_range(
            Some("example.php"),
            Some("stdin"),
            &source,
            TextRange::new(8, 12),
        );

        assert_eq!(location.path.as_deref(), Some("example.php"));
        assert_eq!(location.source_id.as_deref(), Some("stdin"));
        assert_eq!(location.line, Some(2));
        assert_eq!(location.column, Some(3));
        assert_eq!(location.span, Some(DiagnosticSpan::new(8, 12)));
    }

    #[test]
    fn context_order_is_deterministic_in_json_and_text() {
        let mut context = BTreeMap::new();
        context.insert("zeta".to_owned(), "last".to_owned());
        context.insert("alpha".to_owned(), "first".to_owned());
        let diagnostic = DiagnosticEnvelope::new(
            "E_PHRUST_CONTEXT",
            DiagnosticLayer::runtime(),
            DiagnosticPhase::new("execute"),
            DiagnosticSeverity::Warning,
            "context",
        )
        .with_context(context);

        let json = diagnostic.compact_json().expect("json renders");
        let alpha = json.find("\"alpha\"").expect("alpha present");
        let zeta = json.find("\"zeta\"").expect("zeta present");
        assert!(alpha < zeta);

        let text = diagnostic.text_line();
        let alpha = text.find("alpha=first").expect("alpha present");
        let zeta = text.find("zeta=last").expect("zeta present");
        assert!(alpha < zeta);
    }

    #[test]
    fn redaction_matches_secret_keys_case_insensitively() {
        let mut context = BTreeMap::new();
        context.insert("Authorization".to_owned(), "Bearer abc".to_owned());
        context.insert("Cookie".to_owned(), "session=abc".to_owned());
        context.insert("Set-Cookie".to_owned(), "session=abc".to_owned());
        context.insert("x-phrust-metrics-token".to_owned(), "metric".to_owned());
        context.insert("db_password".to_owned(), "pw".to_owned());
        context.insert("apiToken".to_owned(), "token".to_owned());
        context.insert("client_secret".to_owned(), "secret".to_owned());
        context.insert("path".to_owned(), "/tmp/app.php".to_owned());

        let redacted = redact_context(&context);

        assert_eq!(redacted["Authorization"], REDACTED_VALUE);
        assert_eq!(redacted["Cookie"], REDACTED_VALUE);
        assert_eq!(redacted["Set-Cookie"], REDACTED_VALUE);
        assert_eq!(redacted["x-phrust-metrics-token"], REDACTED_VALUE);
        assert_eq!(redacted["db_password"], REDACTED_VALUE);
        assert_eq!(redacted["apiToken"], REDACTED_VALUE);
        assert_eq!(redacted["client_secret"], REDACTED_VALUE);
        assert_eq!(redacted["path"], "/tmp/app.php");
    }

    #[test]
    fn disabled_debug_sink_is_cheap_noop() {
        let sink = DebugSink::disabled();
        let event = DebugEvent::new(
            "D_PHRUST_DISABLED",
            DiagnosticLayer::server(),
            DiagnosticPhase::new("request"),
            "disabled",
        );

        assert!(!sink.is_enabled());
        sink.write_event(&event).expect("disabled sink is ok");
    }

    #[test]
    fn file_debug_sink_writes_json_lines() {
        let path = std::env::temp_dir().join(format!(
            "phrust-diagnostics-{}-{}.jsonl",
            std::process::id(),
            "debug-sink"
        ));
        let _ = fs::remove_file(&path);
        let sink = DebugSink::file(&path, DiagnosticOutputFormat::Json);
        let event = DebugEvent::new(
            "D_PHRUST_FILE",
            DiagnosticLayer::server(),
            DiagnosticPhase::new("request"),
            "handled",
        );

        sink.write_event(&event).expect("file sink writes");
        let output = fs::read_to_string(&path).expect("debug file exists");
        let _ = fs::remove_file(&path);

        assert!(output.ends_with('\n'));
        let decoded: Value = serde_json::from_str(output.trim_end()).expect("valid json");
        assert_eq!(decoded["kind"], "debug_event");
        assert_eq!(decoded["code"], "D_PHRUST_FILE");
    }
}

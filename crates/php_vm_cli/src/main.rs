//! VM CLI process entry point.
//!
//! Command parsing and debug/report adapters live in `commands`; reusable
//! library entrypoints live in `php_vm_cli`.

mod commands;

use php_diagnostics::{DiagnosticOutputFormat, install_panic_diagnostic_hook};
use std::str::FromStr;

fn main() {
    install_panic_diagnostic_hook("php-vm", env_error_format());
    commands::main_entry();
}

fn env_error_format() -> DiagnosticOutputFormat {
    std::env::var("PHRUST_ERROR_FORMAT")
        .ok()
        .and_then(|value| DiagnosticOutputFormat::from_str(&value).ok())
        .unwrap_or(DiagnosticOutputFormat::Text)
}

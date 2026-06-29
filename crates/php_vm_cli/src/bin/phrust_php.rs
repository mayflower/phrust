use std::env;
use std::io::{self, IsTerminal};
use std::str::FromStr;

use php_diagnostics::{DiagnosticOutputFormat, install_panic_diagnostic_hook};

fn main() {
    install_panic_diagnostic_hook("phrust-php", env_error_format());
    let mut stdin = io::stdin();
    let stdin_is_terminal = stdin.is_terminal();
    let code = php_vm_cli::php_cli::run_with_terminal(
        env::args().skip(1),
        &mut stdin,
        stdin_is_terminal,
        &mut io::stdout(),
        &mut io::stderr(),
    );
    if code != 0 {
        std::process::exit(code);
    }
}

fn env_error_format() -> DiagnosticOutputFormat {
    env::var("PHRUST_ERROR_FORMAT")
        .ok()
        .and_then(|value| DiagnosticOutputFormat::from_str(&value).ok())
        .unwrap_or(DiagnosticOutputFormat::Text)
}

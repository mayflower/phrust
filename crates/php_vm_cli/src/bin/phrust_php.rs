use std::env;
use std::io::{self, IsTerminal};
use std::str::FromStr;
use std::thread;

use php_diagnostics::{DiagnosticOutputFormat, install_panic_diagnostic_hook};

const PHP_CLI_STACK_SIZE: usize = 128 * 1024 * 1024;

fn main() {
    install_panic_diagnostic_hook("phrust-php", env_error_format());
    let args: Vec<String> = env::args().skip(1).collect();
    let handle = thread::Builder::new()
        .name("phrust-php-runtime".to_owned())
        .stack_size(PHP_CLI_STACK_SIZE)
        .spawn(move || {
            let mut stdin = io::stdin();
            let stdin_is_terminal = stdin.is_terminal();
            php_vm_cli::php_cli::run_with_terminal(
                args,
                &mut stdin,
                stdin_is_terminal,
                &mut io::stdout(),
                &mut io::stderr(),
            )
        })
        .expect("failed to spawn phrust-php runtime thread");
    let code = handle
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
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

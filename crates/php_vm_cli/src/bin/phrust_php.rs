use std::env;
use std::io::{self, IsTerminal};

fn main() {
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

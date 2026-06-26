use crate::engine::{CliIniOptions, EngineInput, execute_php, read_script};
use std::env;
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::PathBuf;

const EXIT_SUCCESS: i32 = 0;
const EXIT_PHP_ERROR: i32 = 255;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedCli {
    action: CliAction,
    no_ini: bool,
    defines: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CliAction {
    Help,
    Version,
    RunCode { code: String, args: Vec<String> },
    RunFile { path: PathBuf, args: Vec<String> },
    RunStdin { args: Vec<String> },
}

pub fn run<I, R, W, E>(args: I, stdin: &mut R, stdout: &mut W, stderr: &mut E) -> i32
where
    I: IntoIterator<Item = String>,
    R: Read,
    W: Write,
    E: Write,
{
    run_with_terminal(args, stdin, false, stdout, stderr)
}

pub fn run_with_terminal<I, R, W, E>(
    args: I,
    stdin: &mut R,
    stdin_is_terminal: bool,
    stdout: &mut W,
    stderr: &mut E,
) -> i32
where
    I: IntoIterator<Item = String>,
    R: Read,
    W: Write,
    E: Write,
{
    match run_inner(
        args.into_iter().collect(),
        stdin,
        stdin_is_terminal,
        stdout,
        stderr,
    ) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(stderr, "{error}");
            EXIT_PHP_ERROR
        }
    }
}

fn run_inner<R, W, E>(
    args: Vec<String>,
    stdin: &mut R,
    stdin_is_terminal: bool,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<i32, String>
where
    R: Read,
    W: Write,
    E: Write,
{
    let parsed = ParsedCli::parse(&args)?;
    let _no_ini = parsed.no_ini;
    match parsed.action {
        CliAction::Help => {
            print_usage(stdout)?;
            Ok(EXIT_SUCCESS)
        }
        CliAction::Version => {
            writeln!(
                stdout,
                "PHP {} (phrust-php)",
                php_source::reference_php_version()
            )
            .map_err(|error| error.to_string())?;
            Ok(EXIT_SUCCESS)
        }
        CliAction::RunCode { code, args } => {
            let source = normalize_command_line_code(&code);
            let input = EngineInput {
                source,
                source_path: "Command line code".to_string(),
                real_path: None,
                script_name: "Command line code".to_string(),
                script_args: args,
                cwd: current_dir()?,
                env: collect_env(),
                ini: ini_options(&parsed.defines),
                stdin: read_stdin_if_piped(stdin, stdin_is_terminal)?,
            };
            execute_php(input, stdout, stderr)
        }
        CliAction::RunFile { path, args } => {
            let (source, real_path, source_path) = read_script(&path)?;
            let input = EngineInput {
                source,
                source_path,
                real_path: Some(real_path.clone()),
                script_name: real_path.to_string_lossy().into_owned(),
                script_args: args,
                cwd: current_dir()?,
                env: collect_env(),
                ini: ini_options(&parsed.defines),
                stdin: read_stdin_if_piped(stdin, stdin_is_terminal)?,
            };
            execute_php(input, stdout, stderr)
        }
        CliAction::RunStdin { args } => {
            let mut source = String::new();
            stdin
                .read_to_string(&mut source)
                .map_err(|error| format!("stdin: {error}"))?;
            let input = EngineInput {
                source,
                source_path: "php://stdin".to_string(),
                real_path: None,
                script_name: "Standard input code".to_string(),
                script_args: args,
                cwd: current_dir()?,
                env: collect_env(),
                ini: ini_options(&parsed.defines),
                stdin: Vec::new(),
            };
            execute_php(input, stdout, stderr)
        }
    }
}

impl ParsedCli {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut no_ini = false;
        let mut defines = Vec::new();
        let mut index = 0usize;
        while index < args.len() {
            let arg = &args[index];
            match arg.as_str() {
                "-h" | "--help" => {
                    return Ok(Self {
                        action: CliAction::Help,
                        no_ini,
                        defines,
                    });
                }
                "-v" | "--version" => {
                    return Ok(Self {
                        action: CliAction::Version,
                        no_ini,
                        defines,
                    });
                }
                "-n" | "-q" => {
                    no_ini = true;
                    index += 1;
                }
                "-d" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "-d requires name=value".to_string())?;
                    defines.push(parse_define(value));
                    index += 1;
                }
                _ if arg.starts_with("-d") && arg.len() > 2 => {
                    defines.push(parse_define(&arg[2..]));
                    index += 1;
                }
                "-c" => {
                    if args.get(index + 1).is_none() {
                        return Err("-c requires a path".to_string());
                    }
                    index += 2;
                }
                "--repeat" => {
                    if args.get(index + 1).is_none() {
                        return Err("--repeat requires a count".to_string());
                    }
                    index += 2;
                }
                "-r" => {
                    index += 1;
                    let code = args
                        .get(index)
                        .ok_or_else(|| "-r requires code".to_string())?
                        .clone();
                    index += 1;
                    let rest = parse_script_args(&args[index..])?;
                    return Ok(Self {
                        action: CliAction::RunCode { code, args: rest },
                        no_ini,
                        defines,
                    });
                }
                "-f" => {
                    index += 1;
                    let path = args
                        .get(index)
                        .ok_or_else(|| "-f requires a file path".to_string())?;
                    index += 1;
                    let rest = parse_script_args(&args[index..])?;
                    return Ok(Self {
                        action: CliAction::RunFile {
                            path: PathBuf::from(path),
                            args: rest,
                        },
                        no_ini,
                        defines,
                    });
                }
                "--" => {
                    return Ok(Self {
                        action: CliAction::RunStdin {
                            args: args[index + 1..].to_vec(),
                        },
                        no_ini,
                        defines,
                    });
                }
                _ if arg.starts_with('-') => {
                    return Err(format!("unknown option `{arg}`"));
                }
                _ => {
                    let path = PathBuf::from(arg);
                    let rest = parse_script_args(&args[index + 1..])?;
                    return Ok(Self {
                        action: CliAction::RunFile { path, args: rest },
                        no_ini,
                        defines,
                    });
                }
            }
        }
        Ok(Self {
            action: CliAction::RunStdin { args: Vec::new() },
            no_ini,
            defines,
        })
    }
}

fn parse_script_args(args: &[String]) -> Result<Vec<String>, String> {
    if args.first().is_some_and(|arg| arg == "--") {
        Ok(args[1..].to_vec())
    } else {
        Ok(args.to_vec())
    }
}

fn parse_define(value: &str) -> (String, String) {
    value
        .split_once('=')
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .unwrap_or_else(|| (value.to_string(), "1".to_string()))
}

fn ini_options(defines: &[(String, String)]) -> CliIniOptions {
    let mut options = CliIniOptions::default();
    for (name, value) in defines {
        match name.as_str() {
            "include_path" => {
                options.include_path = Some(split_include_path(value).collect());
            }
            "display_errors" => {
                options.display_errors = Some(parse_bool_ini(value));
            }
            "error_reporting" => {
                if let Ok(mask) = value.parse::<i64>() {
                    options.error_reporting = Some(mask);
                }
            }
            _ if name.starts_with("opcache.") => {}
            _ => {}
        }
    }
    options
}

fn split_include_path(value: &str) -> impl Iterator<Item = PathBuf> + '_ {
    value
        .split(':')
        .filter(|part| !part.is_empty())
        .map(PathBuf::from)
}

fn parse_bool_ini(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "off" | "false" | "no"
    )
}

fn normalize_command_line_code(code: &str) -> String {
    if code.trim_start().starts_with("<?") {
        code.to_string()
    } else {
        format!("<?php {code}")
    }
}

fn read_stdin_if_piped<R>(stdin: &mut R, stdin_is_terminal: bool) -> Result<Vec<u8>, String>
where
    R: Read,
{
    if stdin_is_terminal {
        return Ok(Vec::new());
    }
    let mut bytes = Vec::new();
    stdin
        .read_to_end(&mut bytes)
        .map_err(|error| format!("stdin: {error}"))?;
    Ok(bytes)
}

fn current_dir() -> Result<PathBuf, String> {
    env::current_dir().map_err(|error| format!("current directory: {error}"))
}

fn collect_env() -> Vec<(String, String)> {
    env::vars_os()
        .filter_map(|(name, value)| Some((os_to_string(name)?, os_to_string(value)?)))
        .collect()
}

fn os_to_string(value: OsString) -> Option<String> {
    value.into_string().ok()
}

fn print_usage<W: Write>(stdout: &mut W) -> Result<(), String> {
    writeln!(
        stdout,
        "Usage: phrust-php [options] [-f] <file> [--] [args...]\n       phrust-php [options] -r <code> [--] [args...]"
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_TEMP: AtomicUsize = AtomicUsize::new(0);

    struct TestInput(Cursor<Vec<u8>>);

    impl Read for TestInput {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(buf)
        }
    }

    #[test]
    fn parser_accepts_runner_flags_and_repeated_defines() {
        let parsed = ParsedCli::parse(&[
            "-n".to_string(),
            "-d".to_string(),
            "display_errors=1".to_string(),
            "-dinclude_path=fixtures".to_string(),
            "--repeat".to_string(),
            "2".to_string(),
            "-f".to_string(),
            "test.php".to_string(),
            "--".to_string(),
            "a".to_string(),
            "b".to_string(),
        ])
        .expect("parse");

        assert!(parsed.no_ini);
        assert_eq!(
            parsed.defines,
            vec![
                ("display_errors".to_string(), "1".to_string()),
                ("include_path".to_string(), "fixtures".to_string())
            ]
        );
        assert_eq!(
            parsed.action,
            CliAction::RunFile {
                path: PathBuf::from("test.php"),
                args: vec!["a".to_string(), "b".to_string()]
            }
        );
    }

    #[test]
    fn parser_accepts_run_code_and_bare_file() {
        let run_code =
            ParsedCli::parse(&["-r".to_string(), "echo 1;".to_string()]).expect("parse -r");
        assert_eq!(
            run_code.action,
            CliAction::RunCode {
                code: "echo 1;".to_string(),
                args: Vec::new()
            }
        );

        let file =
            ParsedCli::parse(&["script.php".to_string(), "arg".to_string()]).expect("parse file");
        assert_eq!(
            file.action,
            CliAction::RunFile {
                path: PathBuf::from("script.php"),
                args: vec!["arg".to_string()]
            }
        );
    }

    #[test]
    fn parser_rejects_unknown_options() {
        let error = ParsedCli::parse(&["--not-php".to_string()]).expect_err("error");
        assert!(error.contains("unknown option"));
    }

    #[test]
    fn parser_rejects_missing_option_values() {
        let error = ParsedCli::parse(&["-c".to_string()]).expect_err("error");
        assert!(error.contains("-c requires"));
        let error = ParsedCli::parse(&["--repeat".to_string()]).expect_err("error");
        assert!(error.contains("--repeat requires"));
    }

    #[test]
    fn run_code_prints_php_version() {
        let mut stdin = TestInput(Cursor::new(Vec::new()));
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["-r".to_string(), "echo PHP_VERSION;".to_string()],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(status, 0, "{}", String::from_utf8_lossy(&stderr));
        assert_eq!(
            String::from_utf8(stdout).expect("utf8"),
            php_source::reference_php_version()
        );
    }

    #[test]
    fn run_file_seeds_argv_and_argc() {
        let root = temp_root("argv");
        fs::create_dir_all(&root).expect("mkdir");
        let script = root.join("fixture.php");
        fs::write(
            &script,
            "<?php echo $argc, '|', $argv[1], '|', $_SERVER['argv'][2];",
        )
        .expect("write script");
        let mut stdin = TestInput(Cursor::new(Vec::new()));
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "-n".to_string(),
                "-d".to_string(),
                "display_errors=1".to_string(),
                "-f".to_string(),
                script.to_string_lossy().into_owned(),
                "--".to_string(),
                "a".to_string(),
                "b".to_string(),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(status, 0, "{}", String::from_utf8_lossy(&stderr));
        assert_eq!(String::from_utf8(stdout).expect("utf8"), "3|a|b");
    }

    #[test]
    fn run_code_exposes_stdin_resource() {
        let mut stdin = TestInput(Cursor::new(b"payload".to_vec()));
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "-r".to_string(),
                "echo stream_get_contents(STDIN);".to_string(),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(status, 0, "{}", String::from_utf8_lossy(&stderr));
        assert_eq!(String::from_utf8(stdout).expect("utf8"), "payload");
    }

    #[test]
    fn successful_warning_output_does_not_emit_internal_stderr_diagnostics() {
        let mut stdin = TestInput(Cursor::new(Vec::new()));
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "-r".to_string(),
                "class Test {} $o = new Test; var_dump((int) $o);".to_string(),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(status, 0, "{}", String::from_utf8_lossy(&stderr));
        let stdout = String::from_utf8(stdout).expect("utf8");
        assert!(
            stdout.contains("Warning: Object of class Test could not be converted to int"),
            "{stdout}"
        );
        assert!(stdout.contains("int(1)"), "{stdout}");
        assert_eq!(stderr, b"");
    }

    #[test]
    fn include_path_define_affects_include_resolution() {
        let root = temp_root("include-path");
        let lib = root.join("lib");
        fs::create_dir_all(&lib).expect("mkdir");
        let script = root.join("fixture.php");
        fs::write(lib.join("dep.php"), "<?php echo 'dep';").expect("write dep");
        fs::write(&script, "<?php include 'dep.php';").expect("write script");
        let mut stdin = TestInput(Cursor::new(Vec::new()));
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "-d".to_string(),
                format!("include_path={}", lib.display()),
                script.to_string_lossy().into_owned(),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(status, 0, "{}", String::from_utf8_lossy(&stderr));
        assert_eq!(String::from_utf8(stdout).expect("utf8"), "dep");
    }

    fn temp_root(name: &str) -> PathBuf {
        let index = NEXT_TEMP.fetch_add(1, Ordering::SeqCst);
        let path = env::temp_dir().join(format!(
            "phrust-php-cli-{}-{name}-{index}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }
}

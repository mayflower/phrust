use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let out_dir = PathBuf::from("target/phase4/fuzz-vm-smoke");
    fs::create_dir_all(&out_dir)
        .map_err(|error| format!("failed to create {}: {error}", out_dir.display()))?;
    let vm = std::env::var("PHP_VM_CLI").unwrap_or_else(|_| "target/debug/php-vm".to_string());
    if !Path::new(&vm).is_file() {
        return Err(format!(
            "Rust VM binary is missing: {vm}; run `cargo build -p php_vm_cli` first"
        ));
    }

    let programs = generate_programs();
    let mut passed = 0usize;
    for (index, source) in programs.iter().enumerate() {
        let path = out_dir.join(format!("case-{index:03}.php"));
        fs::write(&path, source)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        if let Err(error) = compile_and_run(&vm, &path) {
            let failure = out_dir.join("failure.php");
            fs::write(&failure, source)
                .map_err(|write_error| format!("{error}; failed to save failure: {write_error}"))?;
            return Err(format!(
                "{error}\nminimized failing input saved to {}",
                failure.display()
            ));
        }
        passed += 1;
    }

    let report = format!(
        "[ok] Phase 4 VM fuzz smoke generated={passed} passed={passed} report_dir={}\n",
        out_dir.display()
    );
    fs::write(out_dir.join("fuzz-vm-smoke.txt"), &report)
        .map_err(|error| format!("failed to write fuzz report: {error}"))?;
    print!("{report}");
    Ok(())
}

fn compile_and_run(vm: &str, path: &Path) -> Result<(), String> {
    let compile = Command::new(vm)
        .arg("compile")
        .arg(path)
        .arg("--json")
        .output()
        .map_err(|error| format!("failed to compile {}: {error}", path.display()))?;
    if !compile.status.success() {
        return Err(format!(
            "compile failed for {}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        ));
    }
    let compile_json = String::from_utf8_lossy(&compile.stdout);
    if !compile_json.contains("\"ok\":true") {
        return Err(format!(
            "compile reported diagnostics for {}\n{}",
            path.display(),
            compile_json
        ));
    }

    let run = Command::new(vm)
        .arg("run")
        .arg(path)
        .output()
        .map_err(|error| format!("failed to run {}: {error}", path.display()))?;
    if !run.status.success() {
        return Err(format!(
            "run failed for {}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        ));
    }
    Ok(())
}

fn generate_programs() -> Vec<String> {
    let mut programs = Vec::new();
    for seed in 0..32 {
        programs.push(format!(
            "<?php\n$a = {seed};\n$b = {rhs};\nif ($a < $b) {{ echo $a + $b, \"\\n\"; }} else {{ echo $a - $b, \"\\n\"; }}\n",
            rhs = seed + 3
        ));
        programs.push(format!(
            "<?php\n$total = 0;\nfor ($i = 0; $i < {limit}; $i++) {{ $total = $total + $i; }}\necho $total, \"\\n\";\n",
            limit = seed % 7 + 1
        ));
        programs.push(format!(
            "<?php\nfunction f_{seed}($x) {{ return $x + {delta}; }}\n$value = f_{seed}({seed});\necho $value, \"\\n\";\n",
            delta = seed % 5
        ));
        programs.push(format!(
            "<?php\n$xs = [{first}, {second}];\n$xs[] = {third};\necho $xs[0], \"|\", $xs[2], \"\\n\";\n",
            first = seed,
            second = seed + 1,
            third = seed + 2
        ));
    }
    programs
}

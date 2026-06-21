use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

struct Case {
    name: &'static str,
    source: &'static str,
}

struct Measurement {
    name: &'static str,
    elapsed_ms: u128,
    output_bytes: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let out_dir = PathBuf::from("target/phase4/bench-vm-smoke");
    fs::create_dir_all(&out_dir)
        .map_err(|error| format!("failed to create {}: {error}", out_dir.display()))?;
    let vm = std::env::var("PHP_VM_CLI").unwrap_or_else(|_| "target/debug/php-vm".to_string());
    if !Path::new(&vm).is_file() {
        return Err(format!(
            "Rust VM binary is missing: {vm}; run `cargo build -p php_vm_cli` first"
        ));
    }

    let cases = cases();
    let mut measurements = Vec::new();
    for case in cases {
        let path = out_dir.join(format!("{}.php", case.name));
        fs::write(&path, case.source)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        measurements.push(measure(&vm, case.name, &path)?);
    }

    let report = render_report(&measurements);
    fs::write(out_dir.join("bench-vm-smoke.txt"), &report)
        .map_err(|error| format!("failed to write bench report: {error}"))?;
    print!("{report}");
    Ok(())
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "echo-loop",
            source: "<?php\nfor ($i = 0; $i < 100; $i++) { echo \"x\"; }\necho \"\\n\";\n",
        },
        Case {
            name: "arithmetic-loop",
            source: "<?php\n$sum = 0;\nfor ($i = 0; $i < 200; $i++) { $sum = $sum + $i; }\necho $sum, \"\\n\";\n",
        },
        Case {
            name: "function-calls",
            source: "<?php\nfunction add($a, $b) { return $a + $b; }\n$total = 0;\nfor ($i = 0; $i < 80; $i++) { $total = add($total, $i); }\necho $total, \"\\n\";\n",
        },
        Case {
            name: "array-append",
            source: "<?php\n$xs = [];\nfor ($i = 0; $i < 80; $i++) { $xs[] = $i; }\necho $xs[0], \"|\", $xs[79], \"\\n\";\n",
        },
        Case {
            name: "method-call",
            source: "<?php\nclass Counter { public $value = 0; public function inc($x) { $this->value = $this->value + $x; return $this->value; } }\n$c = new Counter();\nfor ($i = 0; $i < 40; $i++) { $last = $c->inc($i); }\necho $last, \"\\n\";\n",
        },
    ]
}

fn measure(vm: &str, name: &'static str, path: &Path) -> Result<Measurement, String> {
    let start = Instant::now();
    let output = Command::new(vm)
        .arg("run")
        .arg(path)
        .output()
        .map_err(|error| format!("failed to run {name}: {error}"))?;
    let elapsed_ms = start.elapsed().as_millis();
    if !output.status.success() {
        return Err(format!(
            "bench case {name} failed with status {:?}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(Measurement {
        name,
        elapsed_ms,
        output_bytes: output.stdout.len(),
    })
}

fn render_report(measurements: &[Measurement]) -> String {
    let mut out = String::new();
    out.push_str("# Phase 4 VM Bench Smoke\n\n");
    out.push_str("These numbers are a local smoke baseline, not a performance target.\n\n");
    out.push_str("| Case | Elapsed ms | Output bytes |\n");
    out.push_str("| --- | ---: | ---: |\n");
    for measurement in measurements {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            measurement.name, measurement.elapsed_ms, measurement.output_bytes
        ));
    }
    out
}

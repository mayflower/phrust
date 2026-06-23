<?php
// Phase 6 reference wrapper for builtin-function differential fixtures.
//
// Usage:
//   php scripts/phase6/diff_builtin_function.php path/to/fixture.php
//
// The Python harness normally executes fixtures directly to keep source paths
// aligned with the Rust VM. This wrapper exists for manual reference debugging
// and future fixture modes that need a stable PHP entry point.

if ($argc !== 2) {
    fwrite(STDERR, "usage: diff_builtin_function.php FIXTURE\n");
    exit(5);
}

$fixture = $argv[1];
if (!is_file($fixture)) {
    fwrite(STDERR, "fixture not found: {$fixture}\n");
    exit(5);
}

require $fixture;

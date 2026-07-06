# Standard library Extension PHPT Smoke

`just extension-phpt-smoke` runs a curated upstream extension PHPT smoke set for
the Standard library extension surface. The selector reads
`fixtures/stdlib/phpt_extension_manifest.toml`, resolves each selected path
against `third_party/php-src`, writes a generated allowlist under
`target/stdlib/phpt-extension-smoke`, and delegates runnable files to the
existing `run-phpt-smoke` harness.

The manifest covers `ext/standard`, `ext/spl`, `ext/json`, `ext/pcre`, and
`ext/date`. It records upstream-relative paths, category, disposition, and a
reason for every skip, known gap, or expected failure. The upstream PHPT source
files are not copied into this repository.

If the php-src checkout or an individual selected PHPT is missing, the selector
converts that entry into an explicit skip in the generated allowlist. This keeps
reference checkout availability transparent without treating an absent optional
source tree as a passing run.

Generated files:

- `target/stdlib/phpt-extension-smoke/generated-allowlist.toml`
- `target/stdlib/phpt-extension-smoke/selector-report.json`
- `target/stdlib/phpt-extension-smoke/phpt-smoke-report.json`
- `target/stdlib/phpt-extension-smoke/normalized-report.json`

`normalized-report.json` strips workspace-specific absolute paths from runner
output so the report can be compared across machines. Generated reports remain
under `target/` and must not be committed.

Current Standard library policy is conservative: only PHPTs whose output is already
stable against the Rust VM are marked `run`. Broader extension PHPTs stay in the
manifest as explicit skips or known gaps until the corresponding local
differential fixtures and byte-parity gaps are closed.

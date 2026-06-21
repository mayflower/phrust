# Phase 4 Reference Diff

Phase 4 reference diffing compares Rust VM fixture execution with the pinned PHP
8.5.7 CLI when `REFERENCE_PHP` is available. The diff exists to protect the
implemented subset, not to claim full PHP compatibility.

## Commands

```bash
nix develop -c just runtime-reference-smoke
nix develop -c just runtime-fixtures
nix develop -c just runtime-diff
nix develop -c just phpt-smoke
```

`runtime-fixtures` is the Rust-only executable gate. `runtime-diff` also runs
the PHP reference side when configured and writes generated reports under
`target/phase4/runtime-diff`. `phpt-smoke` runs the selected local PHPT smoke
fixtures through the Rust VM.

## Fixture Metadata

Runtime fixtures are discovered under `fixtures/runtime/**/*.php`. Metadata is
read from initial comments with the `phase4-runtime:` prefix and may define:

- `expect`
- `known_gap`
- `args`
- `normalize`
- `php_ref_required`

The fixture kind is also inferred from `valid`, `invalid`, and `known_gaps`
path segments. Known-gap fixtures must carry a stable gap ID and are validated
by `nix develop -c just runtime-known-gaps`.

## Controlled PHP Reference Environment

The PHP reference runner clears the inherited environment and sets only:

| Variable | Value |
| --- | --- |
| `LC_ALL` | `C` |
| `LANG` | `C` |
| `NO_COLOR` | `1` |
| `PHP_INI_SCAN_DIR` | empty string |

If `REFERENCE_PHP` is missing, reference execution reports `skipped`. If
`REFERENCE_PHP` is explicitly set but unusable, the check fails instead of
silently comparing against the wrong binary.

## Normalization Rules

Runtime stderr normalization is implemented by
`php_testkit::normalize_output::normalize_runtime_stderr`:

| Input | Normalized form |
| --- | --- |
| CRLF or CR line endings | LF |
| Current fixture path | `{file}` |
| PHP binary path | `{php}` |
| `on line <number>` | `on line <line>` |
| Decimal duration-looking values | `<duration>` |

No broad semantic rewriting is performed. Rust diagnostics are expected to stay
structured and deterministic; PHP warning and fatal wording compatibility is
documented as a known gap unless a fixture explicitly requires it.

## Report Shape

Each comparison result records:

- fixture file path;
- reference side status/stdout/stderr/exit information;
- Rust side status/stdout/stderr/exit information;
- aggregate comparison status;
- observed diagnostic IDs;
- known-gap ID when applicable;
- concise mismatch message.

The aggregate report counts pass, fail, skipped, and known-gap outcomes.
Generated files under `target/phase4/` are not committed.

## Typical Deviations

Expected Phase 4 deviations from PHP reference behavior include:

- structured diagnostic IDs instead of PHP engine wording;
- missing or simplified stacktrace and Throwable/Error object formatting;
- deterministic include-root resolution instead of include_path and stream
  wrapper lookup;
- empty controlled request superglobals instead of SAPI-populated state;
- a small builtin registry instead of the full standard library and extensions;
- byte-string behavior without mbstring/intl semantics;
- simple numeric conversion and comparison matrices;
- simple object storage without inheritance, visibility scopes, magic methods,
  property hooks, readonly rules, or autoload;
- simple local references without full reference/COW behavior;
- selected local PHPT smoke sections instead of PHP's complete upstream test
  runner.

Every deviation that is intended to remain visible to users is tracked in
`docs/phase4-known-gaps.md` with an ID, example or fixture, and Phase 5+
assignment.

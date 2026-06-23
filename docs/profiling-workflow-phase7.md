# Phase 7 Optional Profiling Workflow

Prompt 07.A adds maintainer-only profiling recipes. They are not part of
`verify-phase7` and they skip by default so normal development stays fast.

## Recipes

Run the recipe first to see the exact local command and tool availability:

```bash
nix develop -c just profile-phase7-dispatch
nix develop -c just profile-phase7-arrays
nix develop -c just profile-phase7-calls
nix develop -c just profile-phase7-composer
```

To actually collect local profiler output, opt in explicitly:

```bash
nix develop -c env PHRUST_PHASE7_PROFILE_RUN=1 just profile-phase7-dispatch
```

All outputs go under `target/phase7/profiles/` and must not be committed.

## Scenarios

| Recipe | Fixture | Purpose |
| --- | --- | --- |
| `profile-phase7-dispatch` | `tests/fixtures/phase7/perf_smoke/loops.php` | VM dispatch and loop overhead |
| `profile-phase7-arrays` | `tests/fixtures/phase7/perf_smoke/arrays_mixed.php` | array-heavy reads, writes, and count paths |
| `profile-phase7-calls` | `tests/fixtures/phase7/perf_smoke/function_calls.php` | user/internal call dispatch |
| `profile-phase7-composer` | `tests/fixtures/phase6/corpus/container_autoload.php` | local Composer-like container/autoload smoke |

## Supported Tools

The script detects these tools and prints skip messages when they are missing:

- `cargo flamegraph` or `cargo-flamegraph`
- Linux `perf`
- macOS `xctrace` Instruments Time Profiler
- macOS `dtrace` availability for manual privileged probes

The macOS and Linux profilers can require local entitlements, kernel settings,
or elevated permissions. Those failures are local setup issues, not Phase 7
gate failures.

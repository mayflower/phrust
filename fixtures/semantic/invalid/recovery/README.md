# Recovery Fixtures

These fixtures intentionally contain malformed PHP. They verify that Phase 3
can consume recovery CSTs from the parser without panics, infinite loops, or
unbounded diagnostic cascades.

Expected behavior:

- parser diagnostics are preserved in `FrontendResult`
- semantic lowering may produce partial HIR with `missing` nodes
- semantic diagnostics are deduplicated and capped
- parser diagnostics are not re-emitted as duplicate semantic errors

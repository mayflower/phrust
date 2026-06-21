# deferred

Purpose: include, require, and eval HIR nodes with explicit deferred-effect
metadata.

Example rules: literal include paths, dynamic include paths, require, and eval.

Reference classification: accepted by PHP lint; runtime execution is not part
of Phase 3.

Rust diagnostic IDs: `N_PHP_RUNTIME_CHECK_DEFERRED`.

Known gaps: filesystem loading, current-scope execution, symbol definitions,
and eval runtime parsing are deferred.

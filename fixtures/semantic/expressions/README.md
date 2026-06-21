# expressions

Purpose: structural expression HIR.

Example rules: operators, calls, fetches, arrays, list/destructuring, match,
include/require/eval/exit constructs, pipe, clone-with, and first-class
callables.

Reference classification: accepted in the current fixture set.

Rust diagnostic IDs: `N_PHP_RUNTIME_CHECK_DEFERRED` for include/require/eval
notes.

Known gaps: expression evaluation is deferred.

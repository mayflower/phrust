#!/usr/bin/env bash
set -euo pipefail

# Structural P0.1 performance gate. These tests execute the production native
# compile coordinator and fail if one miss publishes foreign PHP bodies,
# dormant declarations emit code, failed compiles publish a cell, or
# concurrent callers compile the same key more than once.
cargo test -p php_jit --lib \
  compile_function_does_not_publish_same_unit_callee_body
cargo test -p php_jit --lib \
  ordinary_instructions_do_not_create_resume_or_clif_entry_blocks
cargo test -p php_jit --lib \
  oversized_php_cfg_compiles_as_bounded_direct_native_fragments
cargo test -p php_jit --lib \
  implicit_method_receiver_survives_native_fragment_boundary
cargo test -p php_jit --lib \
  cross_fragment_backedge_does_not_alias_osr_entry_zero
cargo test -p php_jit --lib \
  fragment_state_keeps_path_dependent_local_separate_from_snapshot_liveness
cargo test -p php_vm --lib \
  loading_declaration_heavy_unit_compiles_only_entry_and_declares_other_cells
cargo test -p php_vm --lib \
  same_unit_call_resolves_on_demand_then_calls_native
cargo test -p php_vm --lib \
  instance_method_resolver_uses_exact_packed_entry_arity
cargo test -p php_vm --lib \
  vm::native_compile_cache::tests::concurrent_same_key_compiles_once
cargo test -p php_vm --lib \
  vm::native_compile_cache::tests::compile_breadth_violation_is_rejected_and_cached
cargo test -p php_vm --lib \
  callable_resolution_dereferences_nested_php_references
cargo test -p php_server --lib \
  worker_pool::tests::serial_requests_reuse_the_warm_worker
cargo test -p php_server --lib \
  worker_pool::tests::concurrent_requests_still_use_distinct_workers

printf '%s\n' '[pass] function-on-demand compile and warm-worker gate passed'

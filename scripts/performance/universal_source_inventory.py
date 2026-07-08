#!/usr/bin/env python3
"""Generate a source-only universal PHP performance baseline inventory."""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
BASELINE_DIR = ROOT / "target/performance/universal-php/baseline"
FOUNDATION_DIR = ROOT / "target/performance/universal-php/foundation"

BASELINE_JSON = BASELINE_DIR / "source_inventory.json"
BASELINE_TEXT = BASELINE_DIR / "source_inventory.txt"
FOUNDATION_JSON = FOUNDATION_DIR / "source-inventory.json"
INVARIANT_LEDGER = FOUNDATION_DIR / "invariant-ledger.md"
BASELINE_SUMMARY = FOUNDATION_DIR / "baseline-summary.json"
BRANCH_CONTRACTS = FOUNDATION_DIR / "branch-contracts.md"


REQUIRED_SOURCE_FILES = (
    "crates/php_server/src/state.rs",
    "crates/php_server/src/php_request.rs",
    "crates/php_server/src/persistent_metadata.rs",
    "crates/php_executor/src/cache.rs",
    "crates/php_executor/src/profile.rs",
    "crates/php_executor/src/input.rs",
    "crates/php_vm/src/vm/options.rs",
    "crates/php_vm_cli/src/commands/args.rs",
    "crates/php_vm/src/copy_patch_bridge.rs",
    "crates/php_jit/src/copy_patch.rs",
    "crates/php_jit/src/helpers.rs",
    "crates/php_jit/src/cranelift_lowering.rs",
    "crates/php_jit/src/backend.rs",
    "crates/php_jit/src/eligibility.rs",
    "crates/php_runtime/src/value.rs",
    "crates/php_runtime/src/array.rs",
    "crates/php_runtime/src/object/class.rs",
    "crates/php_runtime/src/object/member.rs",
    "crates/php_vm/src/include.rs",
    "crates/php_vm/src/vm/calls.rs",
    "crates/php_vm/src/vm/mod.rs",
)


@dataclass(frozen=True)
class SourceRefSpec:
    file: str
    symbol: str
    pattern: str
    regex: bool = False


@dataclass(frozen=True)
class ClaimSpec:
    key: str
    title: str
    status: str
    summary: str
    details: tuple[str, ...]
    refs: tuple[SourceRefSpec, ...]


@dataclass(frozen=True)
class SourceRef:
    file: str
    symbol: str
    line: int
    excerpt: str


SOURCE_REF_SPECS = (
    SourceRefSpec(
        "crates/php_server/src/state.rs",
        "AppState",
        "pub(crate) struct AppState",
    ),
    SourceRefSpec(
        "crates/php_server/src/state.rs",
        "ServerEngineState",
        "pub(crate) struct ServerEngineState",
    ),
    SourceRefSpec(
        "crates/php_server/src/state.rs",
        "ServerEngineState::new",
        "pub(crate) fn new(",
    ),
    SourceRefSpec(
        "crates/php_server/src/state.rs",
        "ServerEngineState::executor_options_for_request",
        "pub(crate) fn executor_options_for_request",
    ),
    SourceRefSpec(
        "crates/php_server/src/state.rs",
        "ServerEngineState::compile_script",
        "pub(crate) fn compile_script(",
    ),
    SourceRefSpec(
        "crates/php_server/src/php_request.rs",
        "execute_php_request",
        "pub(crate) async fn execute_php_request",
    ),
    SourceRefSpec(
        "crates/php_server/src/php_request.rs",
        "http_runtime_context",
        "let mut request_context = http_runtime_context(",
    ),
    SourceRefSpec(
        "crates/php_server/src/php_request.rs",
        "seed_session_state",
        "let session_state = match seed_session_state",
    ),
    SourceRefSpec(
        "crates/php_server/src/php_request.rs",
        "php_runtime_context_for_http",
        "let runtime_context = php_runtime_context_for_http(",
    ),
    SourceRefSpec(
        "crates/php_server/src/php_request.rs",
        "execute_compiled_php_in_blocking_region",
        "execute_compiled_php_in_blocking_region(",
    ),
    SourceRefSpec(
        "crates/php_server/src/persistent_metadata.rs",
        "PersistentMetadataStore",
        "pub(crate) struct PersistentMetadataStore",
    ),
    SourceRefSpec(
        "crates/php_server/src/persistent_metadata.rs",
        "PersistentMetadataStore::quickening_templates",
        "pub(crate) fn quickening_templates",
    ),
    SourceRefSpec(
        "crates/php_server/src/persistent_metadata.rs",
        "PersistentMetadataStore::absorb_quickening_feedback",
        "pub(crate) fn absorb_quickening_feedback",
    ),
    SourceRefSpec(
        "crates/php_executor/src/cache.rs",
        "CompiledScriptCache",
        "pub struct CompiledScriptCache",
    ),
    SourceRefSpec(
        "crates/php_executor/src/cache.rs",
        "CompiledScriptCache::get_or_compile_script",
        "pub fn get_or_compile_script",
    ),
    SourceRefSpec(
        "crates/php_executor/src/cache.rs",
        "CompiledScriptCache::read_script_metadata",
        "fn read_script_metadata",
    ),
    SourceRefSpec(
        "crates/php_executor/src/cache.rs",
        "CompiledScriptCache::read_script_source",
        "fn read_script_source",
    ),
    SourceRefSpec(
        "crates/php_executor/src/cache.rs",
        "CompiledScriptCacheKey",
        "struct CompiledScriptCacheKey",
    ),
    SourceRefSpec(
        "crates/php_executor/src/cache.rs",
        "CompiledScriptCacheStats",
        "pub struct CompiledScriptCacheStats",
    ),
    SourceRefSpec(
        "crates/php_vm/src/include.rs",
        "IncludeCache",
        "pub struct IncludeCache",
    ),
    SourceRefSpec(
        "crates/php_vm/src/include.rs",
        "IncludeCache::resolve_with_include_path",
        "pub fn resolve_with_include_path",
    ),
    SourceRefSpec(
        "crates/php_vm/src/include.rs",
        "IncludeCache::get_or_compile_include",
        "pub fn get_or_compile_include",
    ),
    SourceRefSpec(
        "crates/php_vm/src/include.rs",
        "CompiledIncludeKey",
        "struct CompiledIncludeKey",
    ),
    SourceRefSpec(
        "crates/php_executor/src/profile.rs",
        "EngineProfileName",
        "pub enum EngineProfileName",
    ),
    SourceRefSpec(
        "crates/php_executor/src/profile.rs",
        "PhpExecutorOptions::managed_fast_runtime",
        "pub fn managed_fast_runtime",
    ),
    SourceRefSpec(
        "crates/php_vm/src/vm/options.rs",
        "VmOptions",
        "pub struct VmOptions",
    ),
    SourceRefSpec(
        "crates/php_vm/src/vm/options.rs",
        "VmOptions::default",
        "impl Default for VmOptions",
    ),
    SourceRefSpec(
        "crates/php_vm_cli/src/commands/args.rs",
        "BytecodeCacheMode",
        "pub(super) enum BytecodeCacheMode",
    ),
    SourceRefSpec(
        "crates/php_vm_cli/src/commands/args.rs",
        "default_bytecode_cache_mode",
        "pub(super) fn default_bytecode_cache_mode",
    ),
    SourceRefSpec(
        "crates/php_vm/src/copy_patch_bridge.rs",
        "marshal_local",
        "fn marshal_local",
    ),
    SourceRefSpec(
        "crates/php_vm/src/copy_patch_bridge.rs",
        "unmarshal_result",
        "fn unmarshal_result",
    ),
    SourceRefSpec(
        "crates/php_vm/src/copy_patch_bridge.rs",
        "NativeLeaf::compile",
        "pub fn compile(",
    ),
    SourceRefSpec(
        "crates/php_vm/src/copy_patch_bridge.rs",
        "native_call_permits",
        "fn native_call_permits",
    ),
    SourceRefSpec(
        "crates/php_jit/src/copy_patch.rs",
        "ScalarIntOp",
        "pub enum ScalarIntOp",
    ),
    SourceRefSpec(
        "crates/php_jit/src/copy_patch.rs",
        "ScalarFloatOp",
        "pub enum ScalarFloatOp",
    ),
    SourceRefSpec(
        "crates/php_jit/src/copy_patch.rs",
        "IntBinOp",
        "pub enum IntBinOp",
    ),
    SourceRefSpec(
        "crates/php_jit/src/copy_patch.rs",
        "NativeCallPermits",
        "pub struct NativeCallPermits",
    ),
    SourceRefSpec(
        "crates/php_jit/src/helpers.rs",
        "JIT_HELPER_SYMBOLS",
        "pub const JIT_HELPER_SYMBOLS",
    ),
    SourceRefSpec(
        "crates/php_jit/src/cranelift_lowering.rs",
        "CraneliftNoExecBackend::compile_region",
        "fn compile_region(",
    ),
    SourceRefSpec(
        "crates/php_jit/src/cranelift_lowering.rs",
        "lower_function_to_cranelift",
        "pub fn lower_function_to_cranelift",
    ),
    SourceRefSpec(
        "crates/php_jit/src/backend.rs",
        "JitBackendCompileRequest",
        "pub struct JitBackendCompileRequest",
    ),
    SourceRefSpec(
        "crates/php_jit/src/eligibility.rs",
        "JitCandidateKind",
        "pub enum JitCandidateKind",
    ),
    SourceRefSpec(
        "crates/php_runtime/src/object/class.rs",
        "ClassEntry",
        "pub struct ClassEntry",
    ),
    SourceRefSpec(
        "crates/php_runtime/src/object/member.rs",
        "ClassMethodEntry",
        "pub struct ClassMethodEntry",
    ),
    SourceRefSpec(
        "crates/php_runtime/src/object/member.rs",
        "ClassPropertyEntry",
        "pub struct ClassPropertyEntry",
    ),
    SourceRefSpec(
        "crates/php_runtime/src/array.rs",
        "RecordShape",
        "struct RecordShape",
    ),
    SourceRefSpec(
        "crates/php_runtime/src/array.rs",
        "RECORD_SHAPE_CACHE",
        "static RECORD_SHAPE_CACHE",
    ),
    SourceRefSpec(
        "crates/php_vm/src/vm/calls.rs",
        "Vm::resolve_function_call_target",
        "resolve_function_call_target",
    ),
    SourceRefSpec(
        "crates/php_vm/src/vm/mod.rs",
        "internal_function_dispatch_cacheable",
        "fn internal_function_dispatch_cacheable",
    ),
    SourceRefSpec(
        "crates/php_vm/src/vm/mod.rs",
        "method_call_guard_metadata",
        "fn method_call_guard_metadata",
    ),
)


CLAIMS = (
    ClaimSpec(
        key="request_state_persistence",
        title="Request-visible PHP state persistence",
        status="forbidden",
        summary=(
            "Request-visible PHP state must not persist across requests; the server "
            "holds shared engine/cache state in AppState/ServerEngineState and builds "
            "fresh request, session, and runtime contexts per request."
        ),
        details=(
            "Persisted process state includes AppState, ServerEngineState, metrics, "
            "compiled script cache, include cache, and feedback templates.",
            "Per-request construction covers request context, session seed, runtime "
            "context, execution output, uploads cleanup, and session finalization.",
            "No source-supported path keeps PHP-visible globals, objects, resources, "
            "superglobals, output buffers, or request-local statics alive across requests.",
        ),
        refs=(
            SOURCE_REF_SPECS[0],
            SOURCE_REF_SPECS[1],
            SOURCE_REF_SPECS[5],
            SOURCE_REF_SPECS[6],
            SOURCE_REF_SPECS[7],
            SOURCE_REF_SPECS[8],
            SOURCE_REF_SPECS[9],
        ),
    ),
    ClaimSpec(
        key="immutable_artifact_persistence",
        title="Immutable artifact persistence",
        status="source-verified",
        summary=(
            "Compiled entry scripts and include units are process-local immutable "
            "artifacts keyed by source and runtime fingerprints."
        ),
        details=(
            "ServerEngineState owns Arc<CompiledScriptCache> and Arc<IncludeCache>.",
            "Entry script cache lookups canonicalize the path, stat metadata, and "
            "use CompiledScriptCacheKey for exact source/runtime identity.",
            "Include cache keeps separate resolution and compiled-unit shards and "
            "invalidates stale path/dependency fingerprints.",
        ),
        refs=(
            SOURCE_REF_SPECS[1],
            SOURCE_REF_SPECS[4],
            SOURCE_REF_SPECS[13],
            SOURCE_REF_SPECS[17],
            SOURCE_REF_SPECS[19],
            SOURCE_REF_SPECS[20],
            SOURCE_REF_SPECS[21],
            SOURCE_REF_SPECS[22],
        ),
    ),
    ClaimSpec(
        key="feedback_template_persistence",
        title="Feedback template persistence",
        status="source-verified",
        summary=(
            "The only server-side persistent metadata store carries quickening "
            "feedback templates, not PHP-visible state or full OPcache metadata."
        ),
        details=(
            "PersistentMetadataStore contains quickening_templates behind a Mutex.",
            "executor_options_for_request clones templates into request VM options.",
            "absorb_quickening_feedback deduplicates snapshots by site.",
        ),
        refs=(
            SOURCE_REF_SPECS[3],
            SOURCE_REF_SPECS[10],
            SOURCE_REF_SPECS[11],
            SOURCE_REF_SPECS[12],
        ),
    ),
    ClaimSpec(
        key="compiled_script_cache_key_dimensions",
        title="Compiled script cache key dimensions",
        status="source-verified",
        summary=(
            "Entry-script cache identity includes canonical path, file metadata, "
            "source hash, optimization level, executor version, and debug assertions."
        ),
        details=(
            "Metadata stats run before normal hits unless a fresh-by-path interval "
            "allows avoiding a stat.",
            "Source is read on disabled cache, cache miss, or when metadata/fresh "
            "path/exact lookup cannot prove an existing artifact reusable.",
            "The key stores path, len, modified_nanos, source_hash, "
            "optimization_level, executor_version, and debug_assertions.",
        ),
        refs=(
            SOURCE_REF_SPECS[14],
            SOURCE_REF_SPECS[15],
            SOURCE_REF_SPECS[16],
            SOURCE_REF_SPECS[17],
            SOURCE_REF_SPECS[18],
        ),
    ),
    ClaimSpec(
        key="include_cache_behavior",
        title="Include cache behavior",
        status="source-verified",
        summary=(
            "Include cache persists resolved include paths and compiled include "
            "units while validating path and dependency fingerprints."
        ),
        details=(
            "Resolution keys include loader identity, including file, requested path, "
            "include_path, and cwd.",
            "Compiled include keys include canonical path, resolved fingerprint, "
            "local dependency fingerprints, optimization level, executor version, "
            "debug assertions, and source hash.",
            "Hits avoid source reads; misses and stale dependencies re-read source "
            "and recompile.",
        ),
        refs=(
            SOURCE_REF_SPECS[19],
            SOURCE_REF_SPECS[20],
            SOURCE_REF_SPECS[21],
            SOURCE_REF_SPECS[22],
        ),
    ),
    ClaimSpec(
        key="vm_default_modes",
        title="VM and CLI default modes",
        status="source-verified",
        summary=(
            "Raw VmOptions defaults are conservative, while the product Default "
            "engine profile enables adaptive fast paths and Cranelift selection."
        ),
        details=(
            "VmOptions::default uses IR execution, dense includes off, quickening "
            "off, inline caches off, JIT off, and O0 include optimization.",
            "EngineProfileName::Default sets execution auto, dense includes auto, "
            "superinstructions on, quickening on, inline caches on, JIT Cranelift, "
            "tiering enabled, last-use moves on, class-context frame reuse on, "
            "O2 entry compile, and O0 include compile.",
            "CLI bytecode cache defaults to read-write unless PHRUST_BYTECODE_CACHE "
            "or flags override it.",
        ),
        refs=(
            SOURCE_REF_SPECS[23],
            SOURCE_REF_SPECS[24],
            SOURCE_REF_SPECS[25],
            SOURCE_REF_SPECS[26],
            SOURCE_REF_SPECS[27],
            SOURCE_REF_SPECS[28],
        ),
    ),
    ClaimSpec(
        key="copy_patch_native_supported_values",
        title="Copy-patch native supported values",
        status="source-verified",
        summary=(
            "The VM bridge marshals Int, Bool, and Float values into native slots; "
            "other values become Uninitialized and force fallback."
        ),
        details=(
            "Committed native results unmarshal only Int, Bool, and FloatBits tags.",
            "Unsupported strings, arrays, objects, references, null, and "
            "uninitialized values cannot cross as committed native values.",
            "Non-aarch64/non-Unix hosts always return None from the scalar region "
            "bridge.",
        ),
        refs=(
            SOURCE_REF_SPECS[29],
            SOURCE_REF_SPECS[30],
        ),
    ),
    ClaimSpec(
        key="copy_patch_supported_op_families",
        title="Copy-patch supported operation families",
        status="source-verified",
        summary=(
            "Copy-patch lowering covers guarded scalar int/float operations, value "
            "copies, builtin abs(), and a guarded native-to-userland tail-call shape."
        ),
        details=(
            "Integer ops include const, copy, compare, binary/binary-const add, "
            "sub, mul, mod, bitwise and/or/xor, and shifts with guards.",
            "Float ops include const, copy, and guarded binary add/sub/mul/div.",
            "NativeCallPermits gates builtin abs and userland tail-call lowering; "
            "VM-owned name resolution supplies the permissions.",
        ),
        refs=(
            SOURCE_REF_SPECS[31],
            SOURCE_REF_SPECS[32],
            SOURCE_REF_SPECS[33],
            SOURCE_REF_SPECS[34],
            SOURCE_REF_SPECS[35],
            SOURCE_REF_SPECS[36],
        ),
    ),
    ClaimSpec(
        key="cranelift_supported_candidate_families",
        title="Cranelift supported candidate families",
        status="source-verified",
        summary=(
            "Cranelift native compilation is constrained to explicitly recognized "
            "candidate families and otherwise reports verified CLIF-only rejection."
        ),
        details=(
            "Candidate families include int leaf/inline arithmetic, constant "
            "return, packed-array fetch, packed foreach int sum, known strlen/count "
            "calls, string concat, record-array lookup, and property load.",
            "Backend compile requests carry unit/function context, native execution "
            "permission, and VM-owned helper addresses.",
            "Generic lower_function_to_cranelift handles a small integer single-block "
            "subset and rejects unsupported opcodes/control flow.",
        ),
        refs=(
            SOURCE_REF_SPECS[41],
            SOURCE_REF_SPECS[38],
            SOURCE_REF_SPECS[39],
            SOURCE_REF_SPECS[40],
        ),
    ),
    ClaimSpec(
        key="current_call_binding_fast_path_shapes",
        title="Current call binding fast-path shapes",
        status="source-verified",
        summary=(
            "Call fast paths are generic PHP call shapes: direct function lookup, "
            "internal builtin dispatch caching, method-call guard metadata, and "
            "JIT candidate known-call dispatch."
        ),
        details=(
            "The VM resolves function and callable targets through compiled-unit "
            "lookup and runtime state, not framework-specific names.",
            "Internal function dispatch cacheability is guarded by function name and "
            "argument metadata.",
            "Method-call guards track class layout/method table epochs and declaring "
            "class/method metadata.",
        ),
        refs=(
            SOURCE_REF_SPECS[47],
            SOURCE_REF_SPECS[48],
            SOURCE_REF_SPECS[49],
            SOURCE_REF_SPECS[41],
        ),
    ),
    ClaimSpec(
        key="current_object_class_metadata_sharing",
        title="Current object/class metadata sharing",
        status="source-verified",
        summary=(
            "Class, method, and property metadata are runtime table entries; array "
            "record shapes are thread-local interned immutable layouts."
        ),
        details=(
            "ClassEntry stores normalized name, parent/interfaces, methods, "
            "properties, constants, enum cases, attributes, constructor id, and flags.",
            "ClassMethodEntry and ClassPropertyEntry carry method/property flags, "
            "runtime types, hooks, attributes, and function IDs.",
            "RecordShape interns string-key layout and slot maps per thread; record "
            "arrays share shapes and keep values separately.",
        ),
        refs=(
            SOURCE_REF_SPECS[42],
            SOURCE_REF_SPECS[43],
            SOURCE_REF_SPECS[44],
            SOURCE_REF_SPECS[45],
            SOURCE_REF_SPECS[46],
        ),
    ),
    ClaimSpec(
        key="available_performance_counters",
        title="Available performance counters",
        status="source-verified",
        summary=(
            "Baseline-visible counters cover script cache, include cache, persistent "
            "metadata, VM counters, JIT helper registry, and request phases."
        ),
        details=(
            "CompiledScriptCacheStats exposes lookups, hits, misses, source_reads, "
            "metadata_stats, stale_invalidations, compile_errors, evictions, "
            "compile_in_progress, compiles_avoided, entries, and entries_by_shard.",
            "Server request traces record entry_script_cache_hits/misses/source_reads "
            "and include resolution/compile/source read counters.",
            "JIT helper symbols and statuses provide stable native helper metadata "
            "for later branch reports.",
        ),
        refs=(
            SOURCE_REF_SPECS[18],
            SOURCE_REF_SPECS[19],
            SOURCE_REF_SPECS[37],
            SOURCE_REF_SPECS[5],
            SOURCE_REF_SPECS[9],
        ),
    ),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = build_report()
    if args.self_test:
        self_test(report)
        print("[pass] universal source inventory self-test")
        return 0
    write_outputs(report)
    print(f"[pass] wrote {relative(BASELINE_JSON)}")
    print(f"[pass] wrote {relative(BASELINE_TEXT)}")
    print(f"[pass] wrote {relative(FOUNDATION_JSON)}")
    print(f"[pass] wrote {relative(INVARIANT_LEDGER)}")
    print(f"[pass] wrote {relative(BASELINE_SUMMARY)}")
    print(f"[pass] wrote {relative(BRANCH_CONTRACTS)}")
    return 0


def build_report() -> dict[str, Any]:
    files = load_required_files()
    source_refs = {
        ref_key(spec): resolve_ref(files, spec) for spec in SOURCE_REF_SPECS
    }
    claims = [build_claim(spec, source_refs) for spec in CLAIMS]
    return {
        "schema": "phrust.universal_php.source_inventory.v1",
        "source_truth": "rust_and_python_source_only",
        "generated_artifacts": {
            "baseline_json": relative(BASELINE_JSON),
            "baseline_text": relative(BASELINE_TEXT),
            "foundation_source_inventory_json": relative(FOUNDATION_JSON),
            "foundation_invariant_ledger": relative(INVARIANT_LEDGER),
            "foundation_baseline_summary": relative(BASELINE_SUMMARY),
            "foundation_branch_contracts": relative(BRANCH_CONTRACTS),
        },
        "non_overlap": {
            "implements_optimizations": False,
            "modifies_application_code": False,
            "uses_warm_runner": False,
        },
        "source_files": [
            {
                "path": path,
                "sha256": file_sha256(ROOT / path),
                "bytes": (ROOT / path).stat().st_size,
            }
            for path in REQUIRED_SOURCE_FILES
        ],
        "claims": {claim["key"]: claim for claim in claims},
        "derived": {
            "engine_profiles": extract_engine_profiles(files),
            "vm_option_defaults": extract_vm_defaults(files),
            "compiled_script_cache_key_fields": extract_struct_fields(
                files["crates/php_executor/src/cache.rs"],
                "CompiledScriptCacheKey",
            ),
            "compiled_script_cache_counters": extract_struct_fields(
                files["crates/php_executor/src/cache.rs"],
                "CompiledScriptCacheStats",
            ),
            "jit_candidate_kinds": extract_enum_variants(
                files["crates/php_jit/src/eligibility.rs"],
                "JitCandidateKind",
            ),
            "jit_helper_symbols": extract_jit_helper_symbols(
                files["crates/php_jit/src/helpers.rs"]
            ),
            "copy_patch_int_ops": extract_enum_variants(
                files["crates/php_jit/src/copy_patch.rs"],
                "IntBinOp",
            ),
            "copy_patch_scalar_int_ops": extract_enum_variants(
                files["crates/php_jit/src/copy_patch.rs"],
                "ScalarIntOp",
            ),
            "copy_patch_scalar_float_ops": extract_enum_variants(
                files["crates/php_jit/src/copy_patch.rs"],
                "ScalarFloatOp",
            ),
        },
    }


def load_required_files() -> dict[str, str]:
    loaded = {}
    missing = []
    for path in REQUIRED_SOURCE_FILES:
        full_path = ROOT / path
        if not full_path.is_file():
            missing.append(path)
            continue
        loaded[path] = full_path.read_text(encoding="utf-8")
    if missing:
        raise SystemExit(f"missing required source files: {', '.join(missing)}")
    return loaded


def build_claim(
    spec: ClaimSpec, source_refs: dict[str, SourceRef]
) -> dict[str, Any]:
    return {
        "key": spec.key,
        "title": spec.title,
        "status": spec.status,
        "summary": spec.summary,
        "details": list(spec.details),
        "source_refs": [
            source_ref_to_json(source_refs[ref_key(ref)]) for ref in spec.refs
        ],
    }


def resolve_ref(files: dict[str, str], spec: SourceRefSpec) -> SourceRef:
    text = files[spec.file]
    match: re.Match[str] | None
    if spec.regex:
        match = re.search(spec.pattern, text, flags=re.MULTILINE)
    else:
        match = re.search(re.escape(spec.pattern), text, flags=re.MULTILINE)
    if match is None:
        raise SystemExit(
            f"{spec.file}: required symbol `{spec.symbol}` pattern not found: "
            f"{spec.pattern}"
        )
    line = text.count("\n", 0, match.start()) + 1
    excerpt = text.splitlines()[line - 1].strip()
    return SourceRef(spec.file, spec.symbol, line, excerpt)


def source_ref_to_json(ref: SourceRef) -> dict[str, Any]:
    return {
        "file": ref.file,
        "symbol": ref.symbol,
        "line": ref.line,
        "excerpt": ref.excerpt,
    }


def ref_key(spec: SourceRefSpec) -> str:
    return f"{spec.file}:{spec.symbol}:{spec.pattern}"


def extract_struct_fields(text: str, struct_name: str) -> list[str]:
    block = extract_named_block(text, "struct", struct_name)
    fields = []
    for line in block.splitlines():
        stripped = line.strip()
        if stripped.startswith("pub ") or stripped.startswith("pub(crate) "):
            field = stripped.split(":", 1)[0]
            field = field.replace("pub(crate)", "").replace("pub", "").strip()
            fields.append(field)
        elif re.match(r"^[a-zA-Z_][a-zA-Z0-9_]*:", stripped):
            fields.append(stripped.split(":", 1)[0].strip())
    if not fields:
        raise SystemExit(f"{struct_name}: no fields extracted")
    return fields


def extract_enum_variants(text: str, enum_name: str) -> list[str]:
    block = extract_named_block(text, "enum", enum_name)
    variants = []
    for line in block.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith(("///", "#[")):
            continue
        match = re.match(r"^([A-Z][A-Za-z0-9_]*)\b", stripped)
        if match:
            variants.append(match.group(1))
    if not variants:
        raise SystemExit(f"{enum_name}: no variants extracted")
    return variants


def extract_named_block(text: str, kind: str, name: str) -> str:
    match = re.search(rf"\b{kind}\s+{name}\b[^{{]*{{", text)
    if match is None:
        raise SystemExit(f"{kind} {name}: block not found")
    start = match.end()
    depth = 1
    index = start
    while index < len(text) and depth > 0:
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
        index += 1
    if depth != 0:
        raise SystemExit(f"{kind} {name}: unterminated block")
    return text[start : index - 1]


def extract_engine_profiles(files: dict[str, str]) -> dict[str, Any]:
    text = files["crates/php_executor/src/profile.rs"]
    default_block = extract_match_block(text, "EngineProfileName::Default =>")
    baseline_block = extract_match_block(text, "EngineProfileName::Baseline =>")
    experimental_block = extract_match_block(text, "EngineProfileName::ExperimentalJit =>")
    return {
        "baseline": summarize_profile_block(baseline_block),
        "default": summarize_profile_block(default_block),
        "experimental_jit": summarize_profile_block(experimental_block),
    }


def extract_match_block(text: str, marker: str) -> str:
    start = text.find(marker)
    if start == -1:
        raise SystemExit(f"profile marker not found: {marker}")
    brace = text.find("{", start)
    if brace == -1:
        raise SystemExit(f"profile marker has no block: {marker}")
    depth = 1
    index = brace + 1
    while index < len(text) and depth > 0:
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
        index += 1
    if depth != 0:
        raise SystemExit(f"profile marker block unterminated: {marker}")
    return text[brace + 1 : index - 1]


def summarize_profile_block(block: str) -> dict[str, str]:
    fields = (
        "execution_format",
        "dense_include_execution",
        "superinstructions",
        "dense_jump_threading",
        "bytecode_layout",
        "quickening",
        "inline_caches",
        "jit",
        "jit_blacklist",
        "last_use_moves",
        "reuse_class_context_frames",
        "include_optimization_level",
    )
    values = {}
    for field in fields:
        match = re.search(rf"vm_options\.{field}\s*=\s*([^;]+);", block)
        if match:
            values[field] = compact(match.group(1))
    opt_match = re.search(r"OptimizationLevel::(O\d)", block)
    if opt_match:
        values["optimization_level"] = opt_match.group(1)
    return values


def extract_vm_defaults(files: dict[str, str]) -> dict[str, str]:
    text = files["crates/php_vm/src/vm/options.rs"]
    block = extract_match_block(text, "fn default() -> Self")
    fields = (
        "execution_format",
        "dense_include_execution",
        "superinstructions",
        "dense_jump_threading",
        "bytecode_layout",
        "quickening",
        "inline_caches",
        "jit",
        "last_use_moves",
        "reuse_class_context_frames",
        "typecheck_fast_paths",
        "internal_function_dispatch_cache",
    )
    defaults = {}
    for field in fields:
        match = re.search(rf"{field}:\s*([^,]+),", block)
        if match:
            defaults[field] = compact(match.group(1))
    return defaults


def extract_jit_helper_symbols(text: str) -> list[dict[str, Any]]:
    helpers = []
    for block_match in re.finditer(r"JitHelperSymbol\s*{(?P<body>.*?)\n\s*}", text, re.DOTALL):
        body = block_match.group("body")
        name = extract_string_field(body, "name")
        description = extract_string_field(body, "description")
        if name:
            helpers.append(
                {
                    "name": name,
                    "returns": extract_enum_expr_field(body, "returns"),
                    "can_throw": extract_bool_field(body, "can_throw"),
                    "has_side_effects": extract_bool_field(body, "has_side_effects"),
                    "description": description,
                }
            )
    if not helpers:
        raise SystemExit("JIT_HELPER_SYMBOLS: no helper symbols extracted")
    return helpers


def extract_string_field(text: str, field: str) -> str | None:
    match = re.search(rf'{field}:\s*"([^"]+)"', text)
    return match.group(1) if match else None


def extract_enum_expr_field(text: str, field: str) -> str | None:
    match = re.search(rf"{field}:\s*([^,]+),", text)
    return compact(match.group(1)) if match else None


def extract_bool_field(text: str, field: str) -> bool | None:
    match = re.search(rf"{field}:\s*(true|false),", text)
    if not match:
        return None
    return match.group(1) == "true"


def compact(value: str) -> str:
    return " ".join(value.strip().split())


def write_outputs(report: dict[str, Any]) -> None:
    write_json(BASELINE_JSON, report)
    write_text(BASELINE_TEXT, render_text_report(report))
    write_json(FOUNDATION_JSON, report)
    write_json(BASELINE_SUMMARY, baseline_summary(report))
    write_text(INVARIANT_LEDGER, render_invariant_ledger(report))
    write_text(BRANCH_CONTRACTS, render_branch_contracts(report))


def baseline_summary(report: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema": "phrust.universal_php.foundation_baseline_summary.v1",
        "status": "pass",
        "source_truth": report["source_truth"],
        "implements_optimizations": False,
        "modifies_application_code": False,
        "uses_warm_runner": False,
        "claim_count": len(report["claims"]),
        "claims": sorted(report["claims"].keys()),
        "artifacts": report["generated_artifacts"],
    }


def render_text_report(report: dict[str, Any]) -> str:
    lines = [
        "Phrust universal PHP source inventory",
        "=====================================",
        "",
        f"source_truth: {report['source_truth']}",
        "request_visible_php_state_persistence: forbidden",
        "implements_optimizations: false",
        "modifies_application_code: false",
        "uses_warm_runner: false",
        "",
    ]
    for claim in report["claims"].values():
        lines.extend(
            [
                claim["title"],
                "-" * len(claim["title"]),
                f"key: {claim['key']}",
                f"status: {claim['status']}",
                f"summary: {claim['summary']}",
                "details:",
            ]
        )
        lines.extend(f"- {detail}" for detail in claim["details"])
        lines.append("source_refs:")
        lines.extend(
            "- {file}:{line} {symbol} :: {excerpt}".format(**ref)
            for ref in claim["source_refs"]
        )
        lines.append("")
    lines.append("Derived values")
    lines.append("--------------")
    lines.append(json.dumps(report["derived"], indent=2, sort_keys=True))
    lines.append("")
    return "\n".join(lines)


def render_invariant_ledger(report: dict[str, Any]) -> str:
    lines = [
        "# Universal PHP Foundation Invariant Ledger",
        "",
        "Generated from Rust/Python source only. Markdown documents are not "
        "implementation authority for this ledger.",
        "",
        "## Non-Negotiable Invariants",
        "",
        "- Request-visible PHP state persistence is forbidden. Each request must "
        "receive fresh runtime context, superglobals, globals, output buffers, "
        "objects/resources, and request-local state.",
        "- Process lifetime may retain immutable engine artifacts, compiled "
        "script/include caches, helper registries, and non-PHP-visible feedback "
        "templates.",
        "- Application PHP code must not be modified, generated, wrapped, or "
        "special-cased.",
        "- Warm-runner behavior is out of scope: no userland bootstrap may stay "
        "resident as PHP-visible state.",
        "- Later performance branches must update this generated inventory when "
        "they add a shared contract or change a persisted artifact surface.",
        "",
        "## Source-Backed Claims",
        "",
    ]
    for claim in report["claims"].values():
        refs = ", ".join(
            f"`{ref['file']}:{ref['line']}`" for ref in claim["source_refs"][:3]
        )
        lines.append(f"- `{claim['key']}`: {claim['summary']} Sources: {refs}.")
    lines.append("")
    return "\n".join(lines)


def render_branch_contracts(report: dict[str, Any]) -> str:
    derived = report["derived"]
    cache_fields = ", ".join(derived["compiled_script_cache_key_fields"])
    counters = ", ".join(derived["compiled_script_cache_counters"])
    candidates = ", ".join(derived["jit_candidate_kinds"])
    helpers = ", ".join(helper["name"] for helper in derived["jit_helper_symbols"])
    lines = [
        "# Universal PHP Branch Contracts",
        "",
        "This file names shared contracts that Branch A, Branch B, and Branch C "
        "may depend on. It does not implement deployment images, direct calls, "
        "object templates, native stencils, Cranelift preload, callback "
        "specialization, or builtin stubs.",
        "",
        "## Shared Contracts",
        "",
        "- Request isolation contract: PHP-visible state must be fresh per request; "
        "only immutable engine artifacts and non-visible feedback metadata may "
        "persist across process lifetime.",
        f"- Entry script cache key contract: `{cache_fields}`.",
        "- Include cache contract: resolution and compiled include caches validate "
        "path, runtime, source, and dependency fingerprints before reuse.",
        "- VM option naming contract: later branches may use the existing "
        "`VmOptions`, `EngineProfileName`, dense bytecode, quickening, inline "
        "cache, JIT, last-use, and class-context reuse option names.",
        f"- JIT candidate family contract: `{candidates}`.",
        f"- JIT helper registry contract: `{helpers}`.",
        f"- Cache counter contract: `{counters}`.",
        "- Source inventory contract: any branch changing these surfaces must "
        "regenerate the inventory and ledger from source.",
        "",
        "## Branch Non-Overlap",
        "",
        "- Branch A/B/C may depend on the contracts above but must not assume "
        "WordPress-specific function names, class names, hooks, plugins, or path "
        "conventions as semantics.",
        "- WordPress may be used only as a representative benchmark/regression "
        "workload.",
        "- No branch may persist userland objects, globals, resources, output "
        "buffers, or superglobals across requests.",
        "",
    ]
    return "\n".join(lines)


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_text(path: Path, value: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value, encoding="utf-8")


def relative(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def file_sha256(path: Path) -> str:
    import hashlib

    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def self_test(report: dict[str, Any]) -> None:
    required_claims = {
        "request_state_persistence",
        "immutable_artifact_persistence",
        "feedback_template_persistence",
        "compiled_script_cache_key_dimensions",
        "include_cache_behavior",
        "vm_default_modes",
        "copy_patch_native_supported_values",
        "copy_patch_supported_op_families",
        "cranelift_supported_candidate_families",
        "current_call_binding_fast_path_shapes",
        "current_object_class_metadata_sharing",
        "available_performance_counters",
    }
    claim_keys = set(report["claims"].keys())
    if claim_keys != required_claims:
        raise SystemExit(f"claim key mismatch: {sorted(claim_keys)}")
    if report["claims"]["request_state_persistence"]["status"] != "forbidden":
        raise SystemExit("request_state_persistence must be forbidden")
    if report["non_overlap"]["implements_optimizations"]:
        raise SystemExit("source inventory must not implement optimizations")
    if "path" not in report["derived"]["compiled_script_cache_key_fields"]:
        raise SystemExit("compiled script cache key fields did not include path")
    if not report["derived"]["jit_helper_symbols"]:
        raise SystemExit("expected at least one JIT helper symbol")
    expected_ref_files = {
        "current_call_binding_fast_path_shapes": {
            "crates/php_vm/src/vm/calls.rs",
            "crates/php_vm/src/vm/mod.rs",
            "crates/php_jit/src/eligibility.rs",
        },
        "current_object_class_metadata_sharing": {
            "crates/php_runtime/src/object/class.rs",
            "crates/php_runtime/src/object/member.rs",
            "crates/php_runtime/src/array.rs",
        },
        "cranelift_supported_candidate_families": {
            "crates/php_jit/src/eligibility.rs",
            "crates/php_jit/src/cranelift_lowering.rs",
            "crates/php_jit/src/backend.rs",
        },
        "available_performance_counters": {
            "crates/php_executor/src/cache.rs",
            "crates/php_vm/src/include.rs",
            "crates/php_jit/src/helpers.rs",
            "crates/php_server/src/php_request.rs",
        },
    }
    for key, expected_files in expected_ref_files.items():
        actual_files = {ref["file"] for ref in report["claims"][key]["source_refs"]}
        missing = expected_files - actual_files
        if missing:
            raise SystemExit(
                f"{key}: missing expected source files: {', '.join(sorted(missing))}"
            )


if __name__ == "__main__":
    raise SystemExit(main())

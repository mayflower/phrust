#!/usr/bin/env python3
"""Structural ratchet for stable native linkage and unit-bundle footprint."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def require(failures: list[str], condition: bool, message: str) -> None:
    if not condition:
        failures.append(message)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--counters",
        type=Path,
        help="optional linkage-smoke counters JSON for runtime ratchets",
    )
    return parser.parse_args()


def counter(document: dict[str, object], name: str) -> int:
    value = document.get(name, 0)
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"counter {name!r} is not numeric")
    return int(value)


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    lowering = text("crates/php_jit/src/cranelift_lowering/executable_region.rs")
    layout = text("crates/php_jit/src/cranelift_lowering/module_layout.rs")
    linkage = text("crates/php_jit/src/cranelift_lowering/native_linkage.rs")
    manager = text("crates/php_jit/src/code_manager.rs")
    cache = text("crates/php_jit/src/native_cache.rs")
    vm_cache = text("crates/php_vm/src/vm/native_compile_cache.rs")
    worker = text("crates/php_server/src/worker_pool.rs")
    vm_tests = text("crates/php_vm/src/vm/mod.rs")
    dispatch = text("crates/php_vm/src/vm/jit_abi/call_dispatch.rs")
    compiled_unit = text("crates/php_vm/src/compiled_unit.rs")
    arena = text("crates/php_vm/src/vm/jit_abi/frame_arena.rs")

    require(
        failures,
        "whole_unit_function_order" in lowering and "whole_unit_function_order" in layout,
        "unit compilation no longer uses the whole-unit body order",
    )
    require(
        failures,
        not re.search(r"MAX_(?:SMALL|LARGE|LOCAL)_CALL_GRAPH", lowering),
        "transitive call-graph body limit returned",
    )
    require(
        failures,
        "NativeIndirectionCell" in linkage and "AtomicUsize" in linkage,
        "atomic native function indirection cells are missing",
    )
    require(
        failures,
        "duplicate_function_publications" in manager
        and "function_body_compile_count" in manager,
        "duplicate function publication accounting is missing",
    )
    require(
        failures,
        'PNA_MAGIC: [u8; 4] = *b"PNA2"' in cache,
        "native cache writer is not the PNA2 unit-bundle schema",
    )
    require(
        failures,
        'format: "PRM4".to_owned()' in cache
        and "cached_metadata_graph_indices" in cache
        and "encode_functions_v2" in cache,
        "PNA2 repeats graph metadata or per-function ABI identity",
    )
    require(
        failures,
        not re.search(r"put_u(?:32|64)\([^\n]*\.address", cache),
        "persistent artifacts appear to serialize a process address",
    )
    require(
        failures,
        "LoadedNativeUnitRegistry" in vm_cache and "Arc<BTreeMap<FunctionId" in vm_cache,
        "process-shared loaded artifact/entry registry is missing",
    )
    require(
        failures,
        "pub(super) fn get_or_load(" in vm_cache,
        "native artifacts may be mapped outside the process registry",
    )
    stack_match = re.search(
        r"DEFAULT_(?:PINNED_)?PHP_WORKER_STACK_BYTES:\s*usize\s*=\s*(\d+)\s*\*\s*1024\s*\*\s*1024",
        worker,
    )
    require(
        failures,
        stack_match is not None and int(stack_match.group(1)) <= 16,
        "default pinned PHP worker stack exceeds 16 MiB",
    )
    require(
        failures,
        "direct_call_counter_is_nonzero_without_dynamic_dispatch" in vm_tests,
        "linkage smoke does not ratchet non-zero direct-call telemetry",
    )
    require(
        failures,
        "worker_registry_reuses_loaded_artifact_without_remapping_file" in vm_tests,
        "per-request artifact mapping regression test is missing",
    )
    require(
        failures,
        "instruction_for_source" not in dispatch
        and "prepared_callsite_instruction" not in dispatch
        and "instruction_for_continuation" not in dispatch
        and "prepared_native_callsite" in dispatch
        and "NativeCallSiteDescriptor" in compiled_unit,
        "dynamic dispatch returned to source-instruction recovery instead of typed descriptors",
    )
    require(
        failures,
        "native_call_encoded_scratch" in dispatch and ".collect::<Vec<i64>>" not in dispatch,
        "dynamic common-call argument vectors are allocated per call",
    )
    require(
        failures,
        "lookup_native_method_pic" in dispatch
        and "PersistentNativeMethodPic" in compiled_unit
        and "OnceLock<PersistentNativeMethodPicEntry>" in compiled_unit
        and "NATIVE_METHOD_PIC_LIMIT: usize = 4" in text("crates/php_vm/src/vm/jit_abi.rs")
        and "warmed_method_pic_reclassifies_stable_call_as_direct" in vm_tests,
        "process-persistent monomorphic/polymorphic method linkage is not ratcheted",
    )
    require(
        failures,
        "bounded_inline_constant_return" in lowering
        and "bounded_constant_wrapper_inlining_removes_native_call" in vm_tests,
        "bounded native inlining is not ratcheted",
    )
    require(
        failures,
        "bounded_tail_forward_target" in lowering
        and ".return_call(" in lowering
        and "counters.native_tail_calls, 1" in vm_tests,
        "bounded native tail-call lowering is not ratcheted",
    )
    require(
        failures,
        "MAX_NATIVE_SPILL_FRAME_BYTES" in lowering
        and "native_stack_bytes" in lowering,
        "per-function native stack growth is not bounded",
    )
    require(
        failures,
        "NativeFrameArena" in arena
        and "FRAME_ARENA_MAX_BYTES" in arena
        and "libc::mprotect" in arena
        and "guarded_chunk_faults_on_first_byte_past_the_usable_range" in arena
        and "non-LIFO frame release" in arena,
        "bounded guard-page-backed request-local native frame arena is missing",
    )
    require(
        failures,
        "deep_direct_recursion_hits_php_frame_limit_without_stack_abort" in vm_tests,
        "deep direct recursion has no process-abort regression test",
    )

    if args.counters is not None:
        try:
            document = json.loads(args.counters.read_text(encoding="utf-8"))
            if not isinstance(document, dict):
                raise ValueError("top-level JSON value must be an object")
            require(
                failures,
                counter(document, "native_call_direct") > 0,
                "linkage smoke recorded zero direct calls",
            )
            require(
                failures,
                counter(document, "native_duplicate_function_body_count") == 0,
                "linkage smoke published duplicate function bodies",
            )
        except (OSError, json.JSONDecodeError, ValueError) as error:
            failures.append(f"cannot validate counters: {error}")

    if failures:
        for failure in failures:
            print(f"native-linkage-ratchet: {failure}")
        return 1
    print("native-linkage-ratchet: linkage and footprint structure passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

# Cranelift ObjectModule / AOT Research For Performance

Date: 2026-06-24.

Reference target: PHP 8.5.7 (`php-8.5.7`).

This document covers Optional 07.CL.A. It evaluates Cranelift object output for
a future persistent native code cache or preload path. It does not enable a
production path, does not add `cranelift-object` to the workspace, and does not
change runtime behavior.

## Recommendation

Do not enable persistent native code caching in Performance. Keep Cranelift native
execution process-local, feature-gated, and default-off. Revisit
`ObjectModule` only after future runtime has a production lifecycle model for
request reset, worker recycling, preload ownership, dependency invalidation,
W^X policy, crash containment, and cache eviction.

An object-output prototype is useful later, but it should be introduced with a
separate ADR and an optional dependency on `cranelift-object`. The current
addendum already has enough native-execution risk through `JITModule`; adding a
persistent object cache now would expand the safety and invalidation surface
without a matching runtime owner.

## ObjectModule Versus JITModule

| Area | `JITModule` in Performance | `ObjectModule` future path |
| --- | --- | --- |
| Output | Process-local executable memory and function pointers. | Relocatable object files or object bytes that can be persisted. |
| Lifetime | Tied to the running process; Performance leaks finalized modules for handle safety. | Tied to cache files, loader state, relocation records, and possibly worker lifetimes. |
| Symbol resolution | Runtime helpers are resolved through the JIT builder and current process symbols. | Helper references must be serialized as relocations or linked against a stable loader contract. |
| Invalidation | Process-local keys include IR fingerprint, ABI hash, JIT config, ISA, and runtime epoch. | Persistent keys also need source/dependency metadata, helper registry version, object format, relocation model, OS/CPU features, and loader policy. |
| Safety boundary | Function pointers never leave the process and are disabled by default. | Object files can outlive the process that produced them, so stale ABI or helper metadata becomes a disk-cache safety problem. |
| Debuggability | CLIF and in-process counters are enough for current smoke gates. | Object dumps could improve disassembly and symbol-level diagnosis. |

`JITModule` is the right Performance implementation vehicle because it lets the VM
prove narrow native paths without defining a persistent code format. It keeps
all compiled handles inside the process and lets `jit-cranelift` remain an
explicit experiment.

`ObjectModule` becomes attractive only if later layers need a native-code cache
similar in lifecycle to OPcache/preload. That design must be owned by the
runtime/SAPI/cache layer, not by the parser, semantic frontend, or benchmark
harness.

## Cache-Key Requirements

A persistent native object cache key would need at least:

- PHP source fingerprint and normalized source path policy;
- complete include/require/autoload dependency fingerprints;
- lowered IR fingerprint for every compiled function;
- optimizer configuration and lowering version;
- `JIT_RUNTIME_ABI_HASH`;
- `JIT_HELPER_REGISTRY_ABI_HASH`;
- Cranelift version and backend settings;
- target triple, CPU feature set, pointer width, endianness, and object format;
- JIT mode, tiering configuration, guard policy, and blacklist policy;
- runtime class/layout epoch or a persistent equivalent;
- helper symbol version and loader/linker contract;
- repository engine version and PHP reference target version;
- cache format version and schema migration policy.

Any missing dimension must force a miss. Reusing stale object code is not a
permissible fallback because it can bypass VM guards and execute arbitrary
machine code under an outdated ABI.

## ABI-Hash Requirements

Persistent object code must embed or be indexed by both stable hashes already
used by the addendum:

- `JIT_RUNTIME_ABI_HASH` for frame/value/exit layout and native entry
  convention;
- `JIT_HELPER_REGISTRY_ABI_HASH` for helper ids, names, signatures, and return
  status meanings.

The loader must reject a cached object before mapping or linking it when either
hash differs. It is not enough to check after function lookup because stale
relocations could already reference incompatible helper symbols.

## Security And Invalidation Risks

| Risk | Why Performance does not accept it |
| --- | --- |
| Stale executable code | Performance has no complete include/autoload/dependency model for persistent native code. |
| ABI drift | Helper and frame layout hashes can change as Cranelift paths expand. |
| Cross-host reuse | Object files are target-, CPU-, OS-, and relocation-model-specific. |
| Disk cache tampering | Native object files need stronger integrity and trust boundaries than bytecode cache payloads. |
| W^X ownership | Loading object code requires an explicit executable-memory owner and protection-transition policy. |
| Crash containment | Bad native code can crash the process instead of returning a PHP diagnostic. |
| Preload semantics | Persistent native functions can outlive request-local assumptions and destructors. |
| Debug symbol leakage | Object files may expose implementation details or source-derived names. |

The safe Performance answer is to keep persistent native cache disabled and make
unsupported object-cache attempts fail closed.

## Prototype Status

No object file is generated by this work item. The current dependency set includes
`cranelift-codegen`, `cranelift-frontend`, `cranelift-jit`,
`cranelift-module`, and `cranelift-native`; it does not include
`cranelift-object`. Adding that dependency only to produce a discarded local
object under `target/` would expand the build graph without proving a runtime
cache owner.

A future prototype can write to:

```text
target/performance/cranelift/objectmodule/trivial_add.o
```

That future prototype must be optional, excluded from default CI, and validated
only as a diagnostic artifact. It must not be loaded by the runtime until the
persistent native cache ADR exists.

## Follow-up

future runtime should revisit ObjectModule only if all of these are true:

- persistent bytecode/include/autoload invalidation has a production owner;
- executable-memory policy is documented and tested on supported platforms;
- native object cache files have integrity checks and versioned metadata;
- object loading is isolated from default interpreter execution;
- guard/deopt state is serializable or explicitly reconstructed per request;
- the Big-Win matrix shows enough stable hot-path benefit to justify the
  operational complexity.

Until then, `ObjectModule` remains a research item and no product path depends
on it.

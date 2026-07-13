# Native telemetry families

The CLI and server expose one stable telemetry vocabulary for the mandatory
native engine. Counters are opt-in and serialized with schema version 4.

| Family | Meaning |
| --- | --- |
| `native_compile` | Compilation attempts, successes, time, code size, and descriptors. |
| `native_cache` | Persistent artifact loads, stores, misses, rejections, and rebuilds. |
| `native_execution` | Native entries, exits, and executed work. |
| `native_region` | Region compilation, entry, OSR, and region-level exits. |
| `native_call` | Direct compiled calls and native dispatch-trampoline activity. |
| `native_version` | Published baseline and specialized code generations. |
| `native_transition` | Guard exits and precise native-to-native continuation transfers. |
| `runtime_helper` | Calls through the typed runtime-helper ABI. |
| `GC_safepoint` | Published-root and safepoint activity. |

New product counters must belong to one of these families. Detailed diagnostic
profiles may contain nested labels, but they must not recreate retired executor
or backend identities.

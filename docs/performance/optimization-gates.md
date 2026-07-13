# Native optimization gates

All optimization work targets the mandatory Cranelift pipeline. A change is
admissible only when baseline and default preserve PHP-visible output,
diagnostics, exit status, side-effect order, native continuation state, helper
ABI compatibility, and cache identity.

| Class | Policy | Required evidence |
| --- | --- | --- |
| Baseline lowering | Required | Exhaustive IR manifest, helper ABI audit, and native-entry tests. |
| Speculative specialization | Allowed in `default` | Guards, precise state reconstruction, native transitions, and focused reference fixtures. |
| OSR and compiled calls | Allowed | Published native entries, generation ownership, safepoints, and no effect replay. |
| Persistent native cache | Allowed | Identity validation, corruption rebuild, W^X, and fresh-process hit proof. |
| PHP-visible behavior change | Forbidden | Redesign outside the performance layer. |

Use `just cranelift-only-ratchet` for the architectural contract and the
narrowest owning performance fixture before broader gates.

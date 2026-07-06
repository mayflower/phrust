# Known-Gap Manifests

This directory contains machine-readable mirrors for known-gap policy that was
previously checked only through prose and shell greps.

- `runtime.jsonl` mirrors `docs/runtime/known-gaps.md`.
- `performance.jsonl` mirrors `docs/performance/known-gaps.md`.
- `phpt-runner-smoke.jsonl` records current PHPT runner-smoke closure status.

Each JSONL row uses the same required fields: `id`, `feature`, `status`,
`layer`, `fixtures`, `reference_behavior`, `current_behavior`, and
`owner_area`. Concrete `fixtures` must exist in the repository. Wildcards and
future examples belong in `fixture_patterns` or `examples`; rows with neither
must set `fixture_planned=true`. Implemented rows must point at concrete proof
fixtures.

Use the manifest status as the active backlog:

- `known_gap` means the row still blocks a focused validation gate or a documented
  fixture expectation.
- `implemented` means the listed fixtures are positive proof and the row remains
  only as closure evidence.
- Accepted PHPT runner-smoke non-green outcomes must also appear in
  `tests/phpt/manifests/runner-smoke-known-non-green.jsonl`. Keep that manifest
  empty when the runner-smoke gate is green apart from intentional skips and
  XFAILs.

Run `just known-gaps` to validate duplicate IDs, expired entries, fixture paths,
documentation symmetry, actionable metadata, and accepted PHPT non-green
mappings.

# Validate A Change

Use the narrowest relevant gate while iterating, then run the owning aggregate
gate before handing off a change.

## Fast Baseline

```bash
nix develop -c just fmt
nix develop -c just quality-fast
```

For Rust code changes, also run clippy and the relevant tests:

```bash
nix develop -c cargo clippy --workspace --all-targets -- -D warnings
nix develop -c cargo test --workspace
```

## Choose The Owning Gate

| Change area | Focused checks | Aggregate gate |
| --- | --- | --- |
| Lexer | `nix develop -c just lexer-fixtures` | `nix develop -c just verify-frontend` |
| Parser/CST | `nix develop -c just parser-fixtures` | `nix develop -c just verify-frontend` |
| Semantic frontend | `nix develop -c just semantic-fixtures` | `nix develop -c just verify-frontend` |
| Runtime or VM | `nix develop -c just runtime-fixtures` | `nix develop -c just verify-runtime` |
| Runtime semantics | `nix develop -c just runtime-semantics-fixtures` | `nix develop -c just verify-runtime` |
| Standard library | Relevant stdlib unit or PHPT module gate | `nix develop -c just verify-stdlib` |
| Integrated server | `nix develop -c just server-smoke` | `nix develop -c just verify-server` |
| Performance | `nix develop -c just perf-report` or a focused smoke target | `nix develop -c just verify-performance` |
| PHPT tooling or baselines | `nix develop -c just phpt-runner-smoke` | `nix develop -c just verify-phpt` |
| Documentation only | `nix develop -c just quality-docs` | `nix develop -c just quality-docs` |

Use `nix develop -c just help` when a more specific target may exist.

## Known Gaps

If a change documents or preserves unsupported behavior, update the relevant
known-gap document and any owning JSONL manifest:

- [Known-gap manifests](../known_gaps/README.md)
- [Runtime known gaps](../runtime/known-gaps.md)
- [Runtime semantics known gaps](../runtime/semantics-known-gaps.md)
- [Standard library known gaps](../stdlib/known-gaps.md)
- [Performance known gaps](../performance/known-gaps.md)
- [PHPT known gaps](../phpt/known-gaps.md)

Validate manifest consistency with:

```bash
nix develop -c just known-gaps
```

## Reference-Dependent Checks

When a gate compares against PHP itself, prefer an explicit reference binary:

```bash
REFERENCE_PHP=/path/to/php nix develop -c just verify-phpt
```

Do not commit reference checkouts, raw generated reports, or anything under
`target/`.

# Phase 9: PHPT-driven Runtime Completion

Phase 9 turns the php-src PHPT corpus into the primary implementation loop for
runtime completion. The pinned php-src checkout is read-only input containing:

- the PHPT corpus: all discovered `.phpt` files;
- php-src C/Zend implementation source used for source lookup and behavior
  notes;
- Reference PHP build inputs and `run-tests.php` for parity checks.

The Rust engine remains the Target PHP. Phase 9 does not copy php-src source
into the Rust implementation and does not implement behavior by editing the
reference checkout.

Supporting docs:

- [Source integrity](source-integrity.md)
- [Source lookup](source-lookup.md)
- [Binary discovery](binary-discovery.md)
- [Official run-tests.php cross-check](official-runner.md)
- [Generated PHPTs](generated-tests.md)
- [Full PHPT gate](full-phpt-gate.md)

## Terms

Original PHPT: a `.phpt` file under the pinned php-src checkout. It is never
modified.

Derived PHPT: a generated test under `tests/phpt/generated/` with provenance
back to an Original PHPT or Reference PHP observation.

Minimized PHPT: a smaller regression case reduced from an Original PHPT while
preserving the targeted behavior.

Module batch: a curated group of Original PHPT, Derived PHPT, and Minimized PHPT
cases for one runtime area.

Full PHPT gate: a complete corpus run compared against the accepted known
failure baseline after each module batch.

## Required Layout

```text
third_party/php-src-8.5.7/      # preferred pinned php-src checkout
third_party/php-src/            # current local checkout name, accepted by tools
target/phpt-work/               # generated run artifacts only
tests/phpt/generated/           # derived and minimized PHPTs
tests/phpt/manifests/           # indexes, module manifests, baselines
docs/phase9/modules/            # module plans and notes
docs/phase9/php-src-behavior/   # behavior notes from source lookup
docs/phase9/reports/            # committed summary reports
```

Generated run artifacts belong under `target/phpt-work/` and must not be
committed.

## Gate Meanings

Module green means the selected module batch passes for runnable tests and
Derived PHPT or Minimized PHPT cases for that module pass.

Full-run no-regression means the complete PHPT corpus was executed and compared
with the accepted known-failure baseline. Existing known failures may remain,
but new unexpected failures, BORKs, crashes, timeouts, or changed fingerprints
outside the current module reject the change.

Final strict green means the full PHPT corpus passes under the final strict
policy. Any remaining skip or xfail must come from legitimate PHPT metadata,
platform conditions, or a documented intentionally unsupported external
extension.

## Current Foundation Status

The current Phase 9 foundation includes source integrity checks, source lookup,
PHPT corpus indexing, generated PHPT support, Reference PHP smoke checks,
official `run-tests.php` smoke checks, target smoke reporting, and full-corpus
known-failure baselining. Later Phase 9 module prompts use these gates to close
runtime gaps without modifying Original PHPTs.

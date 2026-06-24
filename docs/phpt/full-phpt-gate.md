# PHPT Full PHPT Gate

The Full PHPT gate is mandatory after every module batch. It executes the
complete discovered PHPT corpus and compares the result with the accepted
known-failure baseline.

## Why It Exists

A module batch can be green while the engine regresses unrelated behavior. The
full gate catches new failures, new BORKs, crashes, timeouts, warning
escalations, and changed failure fingerprints outside the active module.

## Outcomes

Module green:

- selected runnable Original PHPT cases for the module pass;
- Derived PHPT and Minimized PHPT cases for the module pass.

Full-run no-regression:

- the complete PHPT corpus runs;
- existing known failures may remain;
- no new unexpected failure or changed fingerprint appears outside the current
  module;
- source integrity still passes.

Final strict green:

- the complete PHPT corpus satisfies the final strict policy;
- no known must-fix runtime failures remain;
- every skip or xfail is documented and justified.

## Artifact Policy

Full-run machine artifacts are written under `target/phpt-work/full-runs/`.
Committed files are limited to manifests, stable generated PHPTs, concise docs,
and summary reports under `docs/phpt/reports/`.

## PHPT Command

```bash
nix develop -c just phpt-full-regression
```

The command runs the complete discovered PHPT corpus from
`tests/phpt/manifests/phpt-corpus.jsonl`, writes machine results to
`target/phpt-work/full-runs/<timestamp>/results.jsonl`, and updates
`tests/phpt/manifests/full-known-failures.jsonl` plus
`docs/phpt/reports/full-baseline.md`.

The default target is `target/debug/php-vm` in `php-vm` mode. This uses
`php-vm run <file>` rather than a PHP CLI-compatible shim. Set `TARGET_PHP` and
`PHPT_TARGET_MODE=php-cli` for a PHP CLI-compatible executable.

When an accepted known-failure manifest already exists, the command compares the
new full run with that baseline and rejects new or changed failure fingerprints.
Set `PHPT_ACCEPT_BASELINE=1` only when intentionally accepting a new baseline.

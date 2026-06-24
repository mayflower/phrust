# PHPT Generated PHPTs

PHPT generated tests are derived from Original PHPTs in the pinned php-src
checkout. The generator never edits `third_party/php-src`; it reads Original
PHPTs and writes derived cases under `tests/phpt/generated/<module>/`.

## Commands

Generate and reference-check a module batch:

```bash
nix develop -c just phpt-generate-module MODULE=zend.basic
```

Run the generated module batch against Reference PHP and Target PHP:

```bash
nix develop -c just phpt-module MODULE=zend.basic
```

Artifacts:

- Original module manifest:
  `tests/phpt/manifests/<module>-originals.jsonl`
- Generated manifest:
  `tests/phpt/manifests/<module>-generated.jsonl`
- Generated PHPTs:
  `tests/phpt/generated/<module>/`
- Run reports:
  `target/phpt-work/module-runs/<module>/`

`target/phpt-work/` artifacts are local run output and are not committed.

## Provenance

Every generated PHPT includes a `DESCRIPTION` section with:

- original php-src path
- original source hash
- generated timestamp
- generator version
- reason for generation

The generated manifest repeats the same provenance as machine-readable JSONL.

## Reducer Policy

Regression cases use a conservative FILE-body reducer. It starts with the whole
Original PHPT FILE body and attempts bounded line removals. A removal is kept
only when Reference PHP produces the same exit status, stdout, and stderr under
the same INI, ENV, ARGS, and STDIN context. If equivalence is not proven, the
line stays.

SKIPIF, INI, ENV, ARGS, and STDIN dependencies are not removed by assumption.
The current generator only emits simple runnable cases where Reference PHP exits
successfully and the generated `EXPECT` output is captured directly from
Reference PHP.

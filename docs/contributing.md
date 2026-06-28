# Contributor Guide

Use this guide when changing Phrust code, tests, documentation, PHPT fixtures,
or compatibility data.

## Development Shell

Run repository commands through Nix:

```bash
nix develop
just help
```

For one-off commands:

```bash
nix develop -c just quality-fast
```

## Validation

Use the narrowest relevant check while iterating. Before handing off a change,
run the aggregate gate for the affected layer.

- General fast gate: `nix develop -c just quality-fast`
- Documentation: `nix develop -c just quality-docs`
- Frontend: `nix develop -c just verify-frontend`
- Runtime and VM: `nix develop -c just verify-runtime`
- Standard library: `nix develop -c just verify-stdlib`
- Integrated server: `nix develop -c just verify-server`
- Performance: `nix develop -c just verify-performance`
- PHPT: `nix develop -c just verify-phpt`

For a detailed gate map, see [Validate a change](how-to/validate-a-change.md).

## Reference PHP

Bootstrap the local reference checkout when a gate needs it:

```bash
nix develop -c just bootstrap-ref
nix develop -c just ref-php-version
```

Keep reference checkouts under `third_party/`. Do not commit php-src copies.

## PHPT Work

Original php-src PHPT files are read-only. Generated or minimized fixtures live
under `tests/phpt/generated/`; manifests and baselines live under
`tests/phpt/manifests/`.

Use [Work with PHPT](how-to/work-with-phpt.md) for commands and the
[PHPT reference](phpt/README.md) for runner details.

## Generated Artifacts

Run artifacts belong under `target/` and must not be committed. Committed
reports should be concise summaries with current status, evidence, and
remaining gaps.

# PHP Reference Metadata

This directory stores the lockfiles and metadata for the pinned PHP reference
oracle. The target is:

- PHP series: `8.5`
- PHP version: `8.5.7`
- Git tag: `php-8.5.7`
- Repository: `https://github.com/php/php-src.git`

The `php-src` source tree is not committed to this repository. Local reference
checkouts belong under:

```text
third_party/php-src
```

Current files:

- `php-src.lock.example.toml`: example lockfile shape for the pinned target.
- `php-src.lock.toml`: local lockfile for the checked-out reference target.
- `php-src.metadata.json`: deterministic metadata extracted from the reference
  checkout, including paths, hashes, sizes, line counts, directory summaries,
  and Git state. It does not copy PHP source code into `references/`.

## Bootstrap

Create the local reference checkout and lockfile with:

```bash
nix develop -c just bootstrap-ref
```

Verify an existing checkout against the lockfile with:

```bash
nix develop -c just verify-ref
```

Reference-dependent checks use `REFERENCE_PHP` when set, then the local
`third_party/php-src/sapi/cli/php` binary when present, then an eligible `php`
from `PATH` where the specific check allows it. Checks skip with an explicit
reason when no suitable reference is available and fail strictly when
`REFERENCE_PHP` is explicitly set but unusable.

The lockfile records:

- PHP series, version, tag, repository, and resolved commit.
- Local checkout path.
- Critical scanner, parser, VM, AST, compiler, and type files.

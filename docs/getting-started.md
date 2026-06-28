# Getting Started

Use this guide to run a PHP file with Phrust from a clean checkout. It assumes
Nix with Flake support is installed.

## Open The Development Shell

```bash
nix develop
just help
```

Phrust commands should run through the Nix shell. For one-off commands, use
`nix develop -c ...`.

## Check The Build

```bash
nix develop -c just quality-fast
```

This is the default fast confidence check for source integrity, dependency
policy, compile coverage, rustdoc, and doctests.

## Run A PHP File

Create or choose a PHP file, then run it through the developer VM CLI:

```bash
nix develop -c cargo run -p php_vm_cli --bin php-vm -- run path/to/file.php
```

The local debug executable used by tests is:

```text
target/debug/php-vm
```

## Optional Reference PHP

Some compatibility checks need a local PHP reference checkout and binary.

```bash
nix develop -c just bootstrap-ref
nix develop -c just ref-php-version
```

Reference-dependent checks skip clearly when no usable reference binary is
available. If `REFERENCE_PHP` is set explicitly, the gate is strict and fails
when that binary is missing or unusable.

## Where To Go Next

- Run HTTP requests through Phrust: [Web server](web-server.md).
- See the CLI surface: [CLI usage](cli.md).
- Check current compatibility: [Compatibility](compatibility.md).
- Contribute to the engine: [Contributor guide](contributing.md).

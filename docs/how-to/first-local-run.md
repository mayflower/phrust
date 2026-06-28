# First Local Run

Use this guide to get from a clean checkout to a working Phrust command. It
assumes Nix with Flake support is installed.

## Open The Development Shell

```bash
nix develop
just help
```

All repository validation commands should run through the Nix shell. For one-off
commands, use `nix develop -c ...`.

## Run The Fast Local Gate

```bash
nix develop -c just quality-fast
```

This is the default first confidence check for source integrity, dependency
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

## Optional Reference PHP Setup

Some compatibility gates need a local PHP reference checkout and binary.

```bash
nix develop -c just bootstrap-ref
nix develop -c just ref-php-version
```

Reference-dependent checks skip clearly when no usable reference binary is
available. If `REFERENCE_PHP` is set explicitly, the gate is strict and fails
when that binary is missing or unusable.

## Where To Go Next

- Validate a change: [Validate a change](validate-a-change.md).
- Run HTTP requests through Phrust: [Run the web server](run-the-web-server.md).
- Work on PHPT compatibility: [Work with PHPT](work-with-phpt.md).
- Understand repository structure: [Documentation index](../README.md).

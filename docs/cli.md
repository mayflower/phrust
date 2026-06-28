# CLI Usage

Use `php-vm` to run PHP code through Phrust's frontend, runtime, and VM.

## Run A PHP Script

```bash
nix develop -c cargo run -p php_vm_cli --bin php-vm -- run path/to/file.php
```

During local development, tests use this debug executable:

```text
target/debug/php-vm
```

## What The CLI Is For

The CLI is intended for local execution, compatibility testing, runtime
debugging, and PHPT target runs. It is not a system PHP replacement and does not
provide Zend extension loading, FPM, Opcache, or a production SAPI.

## Related Docs

- [Getting started](getting-started.md)
- [Compatibility](compatibility.md)
- [Contributor guide](contributing.md)

# ftp

- Strategy: default-disabled FTP surface
- Selected manifest: `tests/phpt/manifests/modules/ftp.selected.jsonl`
- Selected fixture: `tests/phpt/generated/ftp/basic.phpt`

## Implemented Surface

`ftp_connect` and `ftp_ssl_connect` are registered and return `false`
deterministically by default. No host network connection is opened.

## Gaps

FTP login, upload, download, directory listing, SSL/TLS negotiation, and real
network protocol support remain out of scope without an explicit capability
design.

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c just phpt-dev-module MODULE=ftp`

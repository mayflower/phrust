# ftp

- Strategy: loopback-gated FTP control-channel subset
- Selected manifest: `tests/phpt/manifests/modules/ftp.selected.jsonl`
- Selected fixture: `tests/phpt/generated/ftp/basic.phpt`

## Implemented Surface

`ftp_connect` remains disabled by default unless `PHRUST_NET_TESTS=1` is set
for the request. When enabled, it only opens loopback plain FTP control
connections (`127.0.0.1`, `localhost`, or `::1`) and returns an `FTP\Connection`
object after a `220` greeting.

The implemented control-channel and passive-listing functions are:

- `ftp_alloc`
- `ftp_append`
- `ftp_cdup`
- `ftp_chdir`
- `ftp_chmod`
- `ftp_close`
- `ftp_connect`
- `ftp_delete`
- `ftp_exec`
- `ftp_fget` as a deterministic stream-resource gap result
- `ftp_fput` as a deterministic stream-resource gap result
- `ftp_get`
- `ftp_get_option`
- `ftp_login`
- `ftp_mdtm`
- `ftp_mkdir`
- `ftp_mlsd`
- `ftp_nb_continue` as an immediate finished status for completed sync transfers
- `ftp_nb_fget` as a deterministic stream-resource gap result
- `ftp_nb_fput` as a deterministic stream-resource gap result
- `ftp_nb_get`
- `ftp_nb_put`
- `ftp_nlist`
- `ftp_pasv`
- `ftp_put`
- `ftp_pwd`
- `ftp_quit`
- `ftp_raw`
- `ftp_rawlist`
- `ftp_rename`
- `ftp_rmdir`
- `ftp_set_option`
- `ftp_site`
- `ftp_size`
- `ftp_ssl_connect` as a deterministic `false` result for unsupported TLS
- `ftp_systype`

The PHP-src FTP constants for transfer modes, options, and non-blocking status
codes are registered.

## Gaps

Active mode, local stream resource transfers (`ftp_fget`, `ftp_fput`,
`ftp_nb_fget`, `ftp_nb_fput`), true asynchronous transfer progress, and SSL/TLS
negotiation remain gaps. Passive data-channel support is limited to loopback
`PASV` endpoints. External network connections remain disabled even when
`PHRUST_NET_TESTS=1`; only loopback hosts are accepted.

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c just phpt-dev-module MODULE=ftp`

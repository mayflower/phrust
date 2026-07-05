# LDAP PHPT scope

Prompt: `LDAP-1`

Implemented scope:

- Registers the LDAP extension, `LDAP\Connection`, `LDAP\Result`, and `LDAP\ResultEntry`.
- Registers common LDAP functions used by enterprise applications, including connect, bind, read, list, search, get entries, add, modify, delete, option, error, TLS, and extended-operation entry points.
- Provides request-local connection handles without opening network sockets.
- Provides deterministic option storage for common OpenLDAP options such as protocol version, referrals, deref, limits, and TLS require-cert policy.
- Provides deterministic error state through `ldap_errno`, `ldap_error`, and `ldap_err2str`.
- Implements `ldap_escape`, `ldap_explode_dn`, and `ldap_dn2ufn` locally.
- Returns deterministic empty `LDAP\Result` handles from read/list/search so result traversal paths can execute without a live LDAP service.
- Fails bind, write, compare, extended-operation, and TLS operations explicitly with LDAP backend errors.

Security posture:

- No LDAP network connection is opened by default.
- `ldap_start_tls` returns `false` and records an explicit TLS/backend error. It never silently downgrades to plaintext success.
- Certificate/TLS options are stored for app compatibility, but no certificate validation is claimed until a real LDAP backend is wired.

Known gaps:

- No OpenLDAP/libldap or LDAP crate backend is connected yet.
- No DSN-gated live integration PHPTs are promoted yet.
- No SASL, controls parsing, paged results, referrals, or TLS handshake behavior is implemented.
- Directory mutations and bind operations return deterministic failure until a backend exists.

Validation:

- `cargo test -p php_runtime ldap --no-fail-fast`
- `cargo test -p php_std ldap --no-fail-fast`
- `cargo test -p php_vm ldap --no-fail-fast`
- `just phpt-dev-module MODULE=ldap`

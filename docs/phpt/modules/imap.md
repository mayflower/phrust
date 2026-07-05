# IMAP PHPT scope

Prompt: `IMAP-1`

Implemented scope:

- Registers the IMAP extension, `IMAP\Connection`, common mailbox/status/fetch constants, and high-use IMAP functions.
- Provides request-local `IMAP\Connection` handles without opening network sockets or using credentials.
- Provides deterministic empty mailbox behavior for headers, overview, search, fetch body/header/structure, status, check, counts, and mailbox info.
- Tracks request-local delete and expunge state enough for app control flow.
- Implements `imap_last_error`, `imap_errors`, and `imap_alerts` with deterministic queue behavior.
- Fails append and copy/move operations explicitly with an IMAP backend error.

Security posture:

- No IMAP or IMAPS network connection is opened by default.
- Usernames, passwords, and mailbox strings are validated for argument shape but are not logged.
- TLS and certificate behavior is not claimed until a real backend is wired.

Known gaps:

- No c-client compatible binding or IMAP crate backend is connected yet.
- The configured PHP 8.5.7 php-src oracle checkout does not include `ext/imap`, so upstream IMAP PHPT promotion is unavailable from this source tree.
- No live IMAP/IMAPS fixture server or opt-in DSN-gated tests are promoted yet.
- MIME parsing, server search, authentication, TLS handshakes, mailbox mutations, and message persistence are not implemented.

Validation:

- `cargo test -p php_runtime imap --no-fail-fast`
- `cargo test -p php_std imap --no-fail-fast`
- `cargo test -p php_vm imap --no-fail-fast`
- `just phpt-dev-module MODULE=imap`

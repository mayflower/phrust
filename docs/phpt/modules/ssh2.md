# SSH2 PHPT scope

Prompt: `SSH2-1`

Implemented scope:

- Registers the SSH2 extension, common constants, and high-use SSH2 functions for connect, auth, exec, shell, SCP, SFTP, fingerprint, methods, forwarding, and tunnel entry points.
- Provides request-local `SSH2\Session` handles without opening network sockets.
- Provides request-local `SSH2\Sftp` handles attached to open sessions.
- Returns deterministic method and fingerprint shapes for application control flow.
- Fails authentication, exec, shell, tunnel, SCP, forwarding, public-key, and backend-required SFTP operations explicitly until a backend is configured.

Security posture:

- No SSH network connection is opened by default.
- Usernames, passwords, private-key paths, and passphrases are validated for argument shape but are not logged or included in diagnostics.
- No host key, fingerprint, credential, command, or file-transfer success is claimed without a real libssh2 backend.

Known gaps:

- No `ssh2` crate/libssh2 backend is connected yet.
- The configured PHP 8.5.7 php-src oracle checkout does not include an SSH2 extension; SSH2 is normally PECL, so upstream php-src PHPT promotion is unavailable from this source tree.
- No live SSH fixture or opt-in DSN-gated integration tests are promoted yet.
- Real authentication, host-key verification, channel streams, PTY behavior, SCP, SFTP mutation, forwarding, and tunnels are not implemented.

Validation:

- `cargo test -p php_runtime ssh2 --no-fail-fast`
- `cargo test -p php_std ssh2 --no-fail-fast`
- `cargo test -p php_vm ssh2 --no-fail-fast`
- `just phpt-dev-module MODULE=ssh2`

# Run The Web Server

Phrust includes an integrated HTTP server that executes PHP through the
workspace frontend, runtime, and VM. It does not use FPM, FastCGI, CGI, Apache,
`mod_php`, or an external PHP fallback.

## Start The Basic Fixture App

```bash
nix develop -c cargo run -p php_server --bin phrust-server -- --docroot fixtures/server/apps/basic/public --listen 127.0.0.1:8080
```

In another shell:

```bash
curl -i http://127.0.0.1:8080/
```

## Use A Config File

The server supports CLI flags and a simple TOML-style config file:

```bash
nix develop -c cargo run -p php_server --bin phrust-server -- --config path/to/server.toml
```

See [server functionality](../server-functionality.md) for config keys,
timeouts, access-log settings, metrics-token handling, cache options, and TLS
options.

## Run Server Checks

```bash
nix develop -c just server-smoke
nix develop -c just server-compat-smoke all
nix develop -c just server-tls-smoke
nix develop -c just server-benchmark-smoke
nix develop -c just verify-server
```

## Related Docs

- [Server functionality](../server-functionality.md)
- [Server architecture](../server-architecture.md)
- [Server known gaps](../server-known-gaps.md)
- [Cache architecture](../cache-architecture.md)
- [API facades](../api-facades.md)

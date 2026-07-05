# gettext

- Strategy: request-local gettext fallback and binding state
- Selected manifest: `tests/phpt/manifests/modules/gettext.selected.jsonl`
- Selected fixtures:
  - `tests/phpt/generated/gettext/fallback-state.phpt`

## Implemented Surface

The runtime exposes the ext/gettext procedural surface: `gettext`, `_`,
`dgettext`, `dcgettext`, `ngettext`, `dngettext`, `dcngettext`,
`textdomain`, `bindtextdomain`, and `bind_textdomain_codeset`.

This slice implements php-src-compatible untranslated fallback behavior when no
message catalog is loaded, plural fallback selection, request-local textdomain
state, request-local domain directory bindings, request-local codeset bindings,
and php-src length/category value guards.

## Gaps

MO catalog parsing, locale lookup, plural-form expression evaluation, and native
libintl integration remain open. The implemented fallback is intentionally
deterministic for WordPress and other applications that call gettext before any
translation catalog has been installed.

The pinned php-src CLI used by this workspace does not load ext/gettext, so the
selected PHPT row may skip on the reference side while still exercising the
phrust target.

## Target Gates

- `nix develop -c cargo test -p php_runtime gettext`
- `nix develop -c cargo test -p php_std gettext`
- `nix develop -c just phpt-dev-module MODULE=gettext`

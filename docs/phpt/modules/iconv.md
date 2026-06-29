# iconv

- Strategy: bounded encoding conversion MVP
- Selected manifest: `tests/phpt/manifests/modules/iconv.selected.jsonl`
- Selected fixture: `tests/phpt/generated/iconv/basic.phpt`

## Implemented Surface

The runtime exposes `iconv`, `iconv_strlen`, `iconv_substr`, `iconv_strpos`,
`iconv_get_encoding`, and `iconv_set_encoding`.

Supported encodings are `UTF-8`, `ASCII`, and `ISO-8859-1` aliases. Encoding
state is request-local and defaults to `UTF-8`.

## Gaps

The full iconv encoding database, transliteration, ignore-mode parity, and
legacy multibyte encodings remain out of scope.

## Target Gates

- `nix develop -c cargo test -p php_runtime iconv`
- `nix develop -c just phpt-dev-module MODULE=iconv`

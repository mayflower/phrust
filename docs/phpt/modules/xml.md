# xml

- Strategy: bounded parser MVP
- Classification: optional, enabled for the local WordPress XML slice
- Selected manifest: `tests/phpt/manifests/modules/xml.selected.jsonl`
- Selected gate: 6 PASS covering platform visibility, strict parse/reject
  behavior, parser error helpers, current-position helpers, and selected parser
  options.

## Runtime Contract

- `extension_loaded("xml")` returns `true`.
- `xml_parser_create()` returns a bounded `XMLParser` object.
- `xml_parser_create_ns()` returns the same bounded parser resource for the
  selected namespace-aware construction probes.
- `xml_parse(XMLParser $parser, string $data, bool $is_final = false)` returns
  `1` for a strict single-root XML document and `0` for malformed XML.
- `xml_get_error_code()` and `xml_error_string()` expose deterministic parser
  error state for the selected malformed-input slice.
- `xml_get_current_byte_index()`, `xml_get_current_line_number()`, and
  `xml_get_current_column_number()` expose deterministic positions after
  `xml_parse()`.
- `xml_parser_get_option()` and `xml_parser_set_option()` retain selected
  parser options: case folding, target encoding, skip-tag-start, and skip-white.
- Built-in XML entities are decoded. Unresolved entities, DTDs, processing
  instructions beyond the XML declaration, and trailing content are rejected.
- The PHP SAX parser API remains unsupported.

## Required PHPTs

- `tests/phpt/generated/xml/platform-checks.phpt`
- `tests/phpt/generated/xml/parser-basic.phpt`
- `tests/phpt/generated/xml/parser-error-state.phpt`
- `tests/phpt/generated/xml/parser-current-position.phpt`

## Unsupported Area

| Stable ID | Reference behavior summary | Current phrust behavior | Fixture path | Next owner layer |
| --- | --- | --- | --- | --- |
| `XML-DOM-INTL-XML-SAX-CALLBACKS` | PHP `ext/xml` exposes parser callbacks, parser options, and position constants. | `XMLParser`, `xml_parser_create`, `xml_parser_create_ns`, strict `xml_parse`, selected parser options, selected position helpers, and selected error helpers are implemented; SAX callbacks are absent. | `tests/phpt/generated/xml/platform-checks.phpt` | future XML parser resource layer |
| `XML-DOM-INTL-LIBXML-ERROR-STATE` | libxml reports structured parse diagnostics and global error state. | Parse failures expose a deterministic selected error code/string, but no full libxml error buffer is modeled. | `tests/phpt/generated/xml/parser-error-state.phpt` | future libxml compatibility layer |

## Target Gates

- `nix develop -c just phpt-module-target MODULE=xml`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=xml`
- `nix develop -c just verify-phpt`

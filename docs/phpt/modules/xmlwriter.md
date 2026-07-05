# xmlwriter

- Strategy: bounded in-memory XMLWriter MVP
- Selected manifest: `tests/phpt/manifests/modules/xmlwriter.selected.jsonl`
- Selected gate: 2 generated PHPTs covering memory output, element, attribute,
  text, `writeElement()`, static memory construction, procedural aliases, and
  document close behavior

## Runtime Contract

- `extension_loaded("xmlwriter")` returns `true`.
- `XMLWriter::openMemory()`, `startDocument()`, `startElement()`,
  `writeAttribute()`, `text()`, `writeElement()`, `endElement()`,
  `endDocument()`, and `outputMemory()` are implemented for deterministic XML
  output.
- `XMLWriter::toMemory()` returns an initialized in-memory writer.
- Procedural aliases are implemented for the supported in-memory writer path:
  `xmlwriter_open_memory`, `xmlwriter_start_document`,
  `xmlwriter_start_element`, `xmlwriter_write_attribute`, `xmlwriter_text`,
  `xmlwriter_write_element`, `xmlwriter_end_element`,
  `xmlwriter_end_document`, and `xmlwriter_output_memory`.

## Unsupported Area

| Stable ID | Reference behavior summary | Current phrust behavior | Fixture path | Next owner layer |
| --- | --- | --- | --- | --- |
| `XML-DOM-INTL-XMLWRITER-FULL-SURFACE` | PHP XMLWriter supports file/URI/stream output, namespaces, indentation, comments, DTDs, PIs, and libxml-backed error behavior. | Only in-memory elements, attributes, text, `writeElement`, static memory construction, and matching procedural aliases are implemented. | `tests/phpt/generated/xmlwriter/basic.phpt`; `tests/phpt/generated/xmlwriter/procedural-memory.phpt` | future XMLWriter state/libxml layer |

## Target Gates

- `nix develop -c just phpt-module-target MODULE=xmlwriter`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=xmlwriter`
- `nix develop -c just verify-phpt`

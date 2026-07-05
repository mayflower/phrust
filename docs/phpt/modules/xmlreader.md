# xmlreader

- Strategy: bounded XMLReader attribute navigation MVP over the shared XML tree
- Selected manifest: `tests/phpt/manifests/modules/xmlreader.selected.jsonl`
- Selected gate: 4 generated PHPTs covering `open()`, `XML()`, `read()`,
  `next()`, node fields, namespace lookup, attribute cursor movement, XML
  string readers, and `close()`

## Runtime Contract

- `extension_loaded("xmlreader")` returns `true`.
- `XMLReader::open()` reads a local XML file path and initializes the reader.
- `XMLReader::XML()` parses an in-memory strict XML string.
- `XMLReader::read()` advances through element, text, and end-element events.
- `XMLReader::next()` advances to the next element sibling in the bounded event
  stream, optionally matching the element name.
- `XMLReader::lookupNamespace()` resolves namespace declarations visible to the
  current node.
- `XMLReader::moveToAttribute()`, `moveToAttributeNo()`,
  `moveToFirstAttribute()`, `moveToNextAttribute()`, and `moveToElement()`
  expose bounded attribute cursor movement.
- `nodeType`, `name`, `localName`, `prefix`, `depth`, `value`,
  `namespaceURI`, `attributeCount`, `hasAttributes`, `hasValue`,
  `getAttribute()`, `getAttributeNo()`, `readString()`, `readInnerXml()`,
  `readOuterXml()`, and `close()` are available.

## Unsupported Area

| Stable ID | Reference behavior summary | Current phrust behavior | Fixture path | Next owner layer |
| --- | --- | --- | --- | --- |
| `XML-DOM-INTL-XMLREADER-FULL-STREAM` | PHP XMLReader supports URI/stream wrappers, validation flags, DOM expansion, full libxml options/errors, and true streaming behavior. | Local-file and in-memory strict XML traversal, bounded namespace lookup, bounded attribute cursor movement, and XML string readers are implemented. URI/stream wrappers remain unsupported. | `tests/phpt/generated/xmlreader/basic.phpt`, `tests/phpt/generated/xmlreader/navigation-readxml.phpt`, `tests/phpt/generated/xmlreader/open-local-file.phpt`, `tests/phpt/generated/xmlreader/attributes-namespaces.phpt` | future XMLReader stream/libxml/DOM layer |

## Target Gates

- `nix develop -c just phpt-module-target MODULE=xmlreader`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=xmlreader`
- `nix develop -c just verify-phpt`

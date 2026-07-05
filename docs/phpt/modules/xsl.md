# xsl

- Strategy: bounded platform facade
- Classification: optional partial
- Selected manifest: `tests/phpt/manifests/modules/xsl.selected.jsonl`
- Selected gate: 2 generated PHPTs covering XSL platform visibility, stable
  constants, class reflection metadata, method visibility, and the missing
  libxslt backend gate
- Corpus snapshot: 72 `xsl`-owned candidates in
  `tests/phpt/manifests/phpt-corpus.jsonl`; committed known outcomes are
  65 FAIL, 7 BORK, and 72 known non-green outcomes.

## Decision

Expose a bounded XSL platform facade in this branch.

Full XSL requires DOM inputs, libxslt/libexslt integration, stylesheet
import/include handling, PHP callback registration, security preferences,
filesystem/network policy, and output serialization. Those behaviors remain
out of scope, but the extension now exposes platform metadata needed by
framework and capability probes. `XSLTProcessor` construction is owned by the
VM and now fails closed with `E_PHP_VM_UNSUPPORTED_XSL` until a libxslt backend
and DOM handle integration exist.

## Runtime Contract

- `extension_loaded("xsl")` returns `true`.
- `class_exists("XSLTProcessor", false)` returns `true`.
- `method_exists("XSLTProcessor", "hasExsltSupport")` returns `true` from
  generated arginfo metadata.
- `new XSLTProcessor()` emits `E_PHP_VM_UNSUPPORTED_XSL` instead of an
  unknown-class diagnostic while libxslt support is absent.
- Stable clone constants are defined:
  `XSL_CLONE_AUTO`, `XSL_CLONE_NEVER`, and `XSL_CLONE_ALWAYS`.
- Stable security-preference constants are defined:
  `XSL_SECPREF_NONE`, `XSL_SECPREF_READ_FILE`,
  `XSL_SECPREF_WRITE_FILE`, `XSL_SECPREF_CREATE_DIRECTORY`,
  `XSL_SECPREF_READ_NETWORK`, `XSL_SECPREF_WRITE_NETWORK`, and
  `XSL_SECPREF_DEFAULT`.

## Required PHPTs

Required for this strategy:

- `tests/phpt/generated/xsl/platform-checks.phpt`
- `tests/phpt/generated/xsl/backend-gate.phpt`

## Unsupported Area

- Stable ID: `XML-FAMILY-XSL-REAL-IMPLEMENTATION`
- Reference behavior summary: PHP with `ext/xsl` enabled exposes
  `XSLTProcessor` and XSL/libxslt constants declared in
  `ext/xsl/php_xsl.stub.php`.
- Current phrust behavior: XSL is registered as a metadata facade with
  `XSLTProcessor`, stable clone constants, stable security-preference
  constants, and an explicit unsupported-libxslt constructor gate, but no
  libxslt transform engine.
- Fixture paths: `tests/phpt/generated/xsl/platform-checks.phpt` and
  `tests/phpt/generated/xsl/backend-gate.phpt`
- Next owner layer: future DOM/XML implementation plus a dedicated XSL owner
  layer if libxslt integration is approved.

## Out-of-Scope PHPTs

Out of scope for this branch:

- Upstream `ext/xsl/tests/**`
- Successful `XSLTProcessor` construction and method execution
- Stylesheet parsing, transforms, file/network includes, callback registration,
  version constants, and libxslt security preferences

## Target Gates

- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=xsl`
- `nix develop -c just verify-phpt`

## Next Step

Add libxslt/libexslt integration and DOMDocument handle interop before
promoting transform PHPTs.

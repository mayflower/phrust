# soap

- Strategy: bounded platform facade
- Classification: partial
- Selected manifest: `tests/phpt/manifests/modules/soap.selected.jsonl`
- Selected gate: 1 generated PHPT covering SOAP platform visibility and
  global helper state
- Corpus snapshot: 589 `soap`-owned candidates in
  `tests/phpt/manifests/phpt-corpus.jsonl`; committed baseline counts are
  0 PASS, 16 SKIP, 567 FAIL, 6 BORK, and 577 known non-green outcomes.

## Decision

Expose a bounded SOAP facade in this branch.

SOAP's full behavior requires WSDL parsing, XML schema handling, DOM/libxml
behavior, HTTP and stream integration, encoding rules, persistence modes, and
security-sensitive request/response processing. Those areas stay out of scope,
but the extension now has deterministic platform visibility for introspection
and the two global helper functions declared by `ext/soap/soap.stub.php`.

## Runtime Contract

- `extension_loaded("soap")` returns `true`.
- `function_exists("is_soap_fault")` and
  `function_exists("use_soap_error_handler")` return `true`.
- Legacy SOAP classes such as `SoapClient`, `SoapServer`, `SoapFault`,
  `SoapHeader`, `SoapParam`, and `SoapVar` are visible to `class_exists`.
- Namespaced generated metadata classes such as `Soap\SoapClient` and
  `Soap\SoapFault` are visible to `class_exists`.
- `is_soap_fault()` recognizes SOAP fault object classes.
- `use_soap_error_handler()` stores request-local facade state and returns the
  previous enabled flag.

## Required PHPTs

Required for this strategy:

- `tests/phpt/generated/soap/platform-checks.phpt`

## Unsupported Area

- Stable ID: `XML-FAMILY-SOAP-BOUNDED-FACADE`
- Reference behavior summary: PHP with `ext/soap` enabled exposes SOAP client,
  server, fault, parameter, header, and WSDL/encoding behavior declared in
  `ext/soap/soap.stub.php`.
- Current phrust behavior: SOAP is registered as a platform facade with global
  helper functions and class metadata, but no WSDL, transport, XML schema, or
  serialization implementation.
- Fixture path: `tests/phpt/generated/soap/platform-checks.phpt`
- Next owner layer: a future SOAP runtime layer would need DOM/XML, streams,
  HTTP, and schema support first.

## Out-of-Scope PHPTs

Out of scope for this branch:

- Upstream `ext/soap/tests/**`
- `SoapClient` and `SoapServer` construction/calls
- WSDL, HTTP transport, XML schema, encoding, persistence, and security
  regression behavior

## Target Gates

- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=soap`
- `nix develop -c just verify-phpt`

## Next Step

Grow the facade only behind local WSDL and serializer fixtures, then promote
targeted upstream SOAP PHPTs.

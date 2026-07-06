# Prompt Pack 7B Object-Language Closeout

## Scope

This pass focused on selected object-language PHPT and runtime-semantics gaps for
objects, reflection metadata, magic debug output, and object type display names.
It did not attempt broad standard-library extension coverage.

## Starting Inventory

- `runtime-gap-report`: total 97, open 80, implemented 17.
- `runtime-semantics-fixtures`: object-semantics total 84, skip 74, known_gap 10.
- `objects.classes`: reference 246 pass; target 242 pass, 4 fail.
- `objects.core`: reference 16 pass; target 16 pass.
- `reflection`: reference 22 pass; target 22 pass.
- `spl.interfaces`: reference 1 pass; target 1 pass.

The `objects.classes` failures were one cluster: PHP-visible class type names
in method signature diagnostics were lowercased instead of preserving source
spelling. The affected selected PHPT rows were:

- `tests/classes/type_hinting_005c.phpt`
- `tests/classes/type_hinting_005a.phpt`
- `tests/classes/type_hinting_002.phpt`
- `tests/classes/autoload_009.phpt`

## Changes

- Preserved source-spelled class type display names through HIR-to-IR return
  type lowering and VM runtime type diagnostics.
- Added `__debugInfo()` dispatch for `var_dump` on public instance methods.
  The returned array is formatted through the existing runtime debug formatter
  as the original object handle, preserving string and integer debug labels.
- Promoted runtime-semantics fixtures that now match the PHP reference:
  - `fixtures/runtime_semantics/types/static-property.php`
  - `fixtures/runtime_semantics/enums/reflection-name.php`
  - `fixtures/runtime_semantics/reflection/attribute-newinstance.php`
  - `fixtures/runtime_semantics/objects/anonymous-class.php`
  - `fixtures/runtime_semantics/magic/debug-info.php`
- Updated known-gap documentation and JSONL entries to mark
  `E_PHP_RUNTIME_UNSUPPORTED_DEBUGINFO` implemented and keep wider edge cases
  under the broader magic-method gap.

## Gap IDs Closed or Narrowed

- `E_PHP_RUNTIME_UNSUPPORTED_DEBUGINFO`: implemented for fixture-covered
  `var_dump` dispatch with public instance `__debugInfo()`.
- `E_PHP_RUNTIME_TYPEERROR_TEXT_COMPAT`: narrowed by preserving source-spelled
  class names in selected method signature diagnostics.
- `E_PHP_IR_UNSUPPORTED_CLASSLIKE_OBJECT`: narrowed by the promoted anonymous
  class construction fixture.
- `E_PHP_IR_UNSUPPORTED_STATIC_PROPERTY`: narrowed by the promoted typed static
  property read fixture.
- `E_PHP_IR_UNSUPPORTED_REFLECTION`: narrowed by the promoted
  `ReflectionEnum::getName()` fixture.
- `E_PHP_RUNTIME_UNSUPPORTED_ATTRIBUTE_NEWINSTANCE`: narrowed by the promoted
  bounded userland attribute `newInstance()` fixture.

`runtime-gap-report` now reports total 97, open 79, implemented 18.

## After PHPT Counts

Using:

```bash
REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php \
PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src \
PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 \
nix develop -c just phpt-dev-module MODULE=<module>
```

- `objects.classes`: reference 246 pass; target 246 pass.
- `objects.core`: reference 16 pass; target 16 pass.
- `reflection`: reference 22 pass; target 22 pass.
- `spl.interfaces`: reference 1 pass; target 1 pass.

## Remaining Object-Language Gaps

- Trait properties, trait constants, nested trait uses, and broader exact
  trait/interface consistency diagnostics.
- Full property-hook grammar, inheritance/override compatibility, readonly
  interactions, by-reference lvalues, and parser coverage for defaults before
  hook lists.
- Clone-with private/protected/readonly/static/full-property-hook parity.
- By-reference `__get` lvalues and exact debug-output recursion/diagnostics.
- Enum serialization and the wider `ReflectionEnum`/Reflection API matrix.
- Full `ReflectionAttribute::newInstance()` parity for named arguments, target
  and repeatability validation, internal attributes, exact diagnostics, and
  autoload-sensitive lookup.

## Validation

- PASS: `nix develop -c cargo fmt --check`
- PASS: `nix develop -c cargo test -p php_runtime object`
- PASS: `nix develop -c cargo test -p php_vm objects`
- PASS: `nix develop -c cargo test -p php_vm methods`
- PASS: `nix develop -c cargo test -p php_vm reflection`
- PASS: `nix develop -c cargo test -p php_vm var_dump_uses_debug_info_with_original_object_handle -- --nocapture`
- PASS: `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c scripts/runtime_semantics_diff.py --out target/runtime-semantics/7b-promoted-pass ...`
  - total 5, pass 5, fail 0, skip 0, known_gap 0.
- PASS: `nix develop -c just runtime-semantics-fixtures`
- PASS: `nix develop -c just runtime-gap-report`
- PASS: `nix develop -c just runtime-known-gaps`
- PASS: selected PHPT module gates listed above.
- PASS: `nix develop -c just verify-runtime`
- PASS: `PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just verify-phpt`
  - Source integrity checked 24475 php-src manifest entries, skipped 0.

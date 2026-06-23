# Phase 6 Symbol Introspection

Reference target: PHP 8.5.7 (`php-8.5.7`).

Prompt 06.21 adds VM-backed symbol introspection for framework and Composer
checks. The non-VM builtin registry advertises the functions, but execution is
routed through the VM because these APIs need user function tables, class
tables, constants, object metadata, and SPL autoload state.

Prompt 06.43 extends the same VM-owned surface with object/class inspection,
callable invocation helpers, and current-call argument access.

Implemented functions:

- `defined`
- `constant`
- `function_exists`
- `class_exists`
- `interface_exists`
- `enum_exists`
- `method_exists`
- `property_exists`
- `is_subclass_of`
- `get_class`
- `get_parent_class`
- `get_declared_classes`
- `get_declared_interfaces`
- `get_object_vars`
- `get_mangled_object_vars`
- `get_class_methods`
- `get_class_vars`
- `call_user_func`
- `call_user_func_array`
- `forward_static_call`
- `func_get_args`
- `func_num_args`
- `func_get_arg`

The optional autoload argument for `class_exists`, `interface_exists`, and
`enum_exists` is respected. `false` performs a symbol-table-only lookup and does
not invoke registered autoload callbacks; `true` or an omitted argument invokes
autoload when the symbol is missing.

Known gap:

- `PHASE6-GAP-SYMBOL-TRAIT-INTROSPECTION`: user trait declarations are still not
  preserved in the frontend/IR symbol tables, so `trait_exists` and
  `get_declared_traits` are registered but cannot match PHP for user traits yet.
- `PHASE6-GAP-FORWARD-STATIC-CALL-EDGE-PARITY`: ordinary static callables are
  routed through the VM callable path, but complex late-static-binding edge
  cases and byte-perfect diagnostics are deferred.
- `PHASE6-GAP-CALL-USER-FUNC-ARRAY-BYREF`: array elements passed to
  `call_user_func_array` are not yet promoted into by-reference callback slots.

Validation fixtures:

- `tests/fixtures/phase6/_harness/stdlib/symbol_introspection.php`
- `tests/fixtures/phase6/_harness/stdlib/symbol_introspection_autoload.php`
- `tests/fixtures/phase6/_harness/stdlib/symbol_introspection_traits.php`

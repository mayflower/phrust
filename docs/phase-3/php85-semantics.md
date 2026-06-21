# PHP 8.5 Semantic Frontend Notes

Phase 3 models PHP 8.5 syntax structurally. It records HIR shape and
compile-time diagnostics, but does not execute callables, clone operations,
casts, object creation, or constant-expression values that require runtime
semantics.

## Pipe Operator

`|>` lowers to `HirExprKind::Pipe` with `input` and `callable` expression IDs.
The right side is preserved in callable position, including first-class
callable syntax, but Phase 3 does not check whether it is invokable and does
not call it.

Fixtures:

- `fixtures/semantic/php85/pipe.php`
- `fixtures/semantic/php85/pipe-basic.php`
- `fixtures/semantic/php85/pipe-precedence.php`

## Void Cast

`(void)` lowers to `HirExprKind::Cast { kind: "void", ... }` so later phases can
see the parsed shape. The pinned PHP reference rejects this source form with a
parse error, so Phase 3 emits `E_PHP_INVALID_VOID_CAST` and Rust acceptance must
match the reference. The frontend also emits `N_PHP_RUNTIME_CHECK_DEFERRED` as a
note because value-discard behavior would be runtime behavior if a future target
accepted the syntax.

Fixtures:

- `fixtures/semantic/php85/void-cast.php`
- `fixtures/semantic/php85/void-cast-const-expr.php`

## Clone-With

Clone-with syntax lowers to `HirExprKind::CloneWith`. Phase 3 records the
target expression and replacement expressions structurally. It does not inspect
object identity, property visibility, property existence, or copy-on-write
behavior.

Fixtures:

- `fixtures/semantic/php85/clone-with.php`
- `fixtures/semantic/php85/clone-with-basic.php`

## Constant Expressions

Closures, first-class callables, casts, and `new` expressions in constant
expression positions are represented symbolically:

- closures: `ConstExprKind::Closure`
- first-class callables: `ConstExprKind::FirstClassCallable`
- casts: `ConstExprKind::Cast`
- object creation: `ConstExprKind::New`

Phase 3 intentionally avoids producing closure objects, callable targets, zvals,
or constructed objects.

Fixtures:

- `fixtures/semantic/php85/closure-const-expr.php`
- `fixtures/semantic/php85/first-class-callable-const-expr.php`
- `fixtures/semantic/php85/cast-const-expr.php`
- `fixtures/semantic/php85/const-expr-closure.php`
- `fixtures/semantic/php85/const-expr-new.php`
- `fixtures/semantic/php85/first-class-callable.php`

## Reference Status

`nix develop -c scripts/compare_semantic_acceptance.py --fixture-root
fixtures/semantic/php85` is the reference acceptance gate for this focused
matrix. Any unexpected Rust/PHP acceptance difference must be either fixed or
entered into `fixtures/semantic/known_gaps.toml` with a narrow reason.

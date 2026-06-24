# Syntax Sources

Foundation records the authoritative PHP `8.5.7` syntax sources. It does not
extract a grammar, implement a scanner, or implement a parser.

## Normative Syntax Sources

The Foundation syntax contract uses these sources:

- `Zend/zend_language_scanner.l`
- `Zend/zend_language_parser.y`
- `Zend/zend_ast.h`
- `Zend/zend_compile.h`
- `Zend/zend_vm_def.h`
- PHP manual token list
- `.phpt` tests from `Zend/tests` and `tests`

## Why Parser File Alone Is Not Enough

PHP syntax is not fully described by a single EBNF grammar. Compatibility
depends on scanner states, PHP/HTML mode transitions, heredoc and nowdoc
handling, string interpolation, variable variables, parser actions, and
compile-time semantic checks outside the parser grammar.

The pinned `php-src` checkout is therefore the primary source, supported by the
manual token list and `.phpt` tests.

## Critical Files

| Source | Purpose |
| --- | --- |
| `Zend/zend_language_scanner.l` | Tokenization rules, scanner states, PHP/HTML mode handling, heredoc/nowdoc, interpolation. |
| `Zend/zend_language_parser.y` | Parser grammar, parser actions, syntactic accept/reject behavior. |
| `Zend/zend_vm_def.h` | VM opcode definitions and execution behavior reference for later layers. |
| `Zend/zend_ast.h` | AST node kinds used by the Zend compiler. |
| `Zend/zend_compile.h` | Compiler interfaces and semantic checks coupled to parsing. |
| `Zend/zend_types.h` | Core Zend value and type declarations for later runtime mapping. |
| `Zend/zend_exceptions.h` | Exception structures and behavior relevant to parse/compile/runtime errors. |
| `Zend/zend_interfaces.c` | Built-in interface behavior relevant to later runtime compatibility. |
| `Zend/zend_builtin_functions.c` | Built-in functions that shape observable runtime behavior. |
| `Zend/tests` | Zend-specific `.phpt` behavior tests. |
| `tests` | Broader PHP source-tree tests. |
| PHP manual token list | Human-readable token catalog for cross-checking generated token artifacts. |

## PHP 8.5 Syntax Feature Candidates

These features are candidates for explicit verification in later layers:

| Feature | Reference source | Tests present? | Layer | Status |
| --- | --- | --- | --- | --- |
| Pipe Operator `|>` | Scanner, parser, `.phpt` tests | To verify | Lexer and Parser | Planned |
| Clone With | Parser, compiler, `.phpt` tests | To verify | Parser and Frontend | Planned |
| `#[\NoDiscard]` | Attributes, compiler, `.phpt` tests | To verify | Parser and Frontend | Planned |
| `(void)` cast | Scanner, parser, compiler tests | To verify | Lexer and Parser | Planned |
| Closures in constant expressions | Parser, compiler tests | To verify | Parser | Planned |
| Casts in constant expressions | Parser, compiler tests | To verify | Parser | Planned |
| First-class callables in constant expressions | Parser, compiler tests | To verify | Parser | Planned |
| `match` | Parser, compiler, `.phpt` tests | To verify | Parser and Frontend | Planned |
| Enums | Parser, compiler, runtime tests | To verify | Parser and Frontend | Planned |
| Attributes | Scanner, parser, compiler tests | To verify | Lexer and Parser | Planned |
| `readonly` | Parser, compiler, object tests | To verify | Parser and Frontend | Planned |
| Property hooks | Parser, compiler, object tests | To verify | Parser and Frontend | Planned |
| Asymmetric visibility | Parser, compiler, object tests | To verify | Parser and Frontend | Planned |
| Nullsafe operator | Scanner, parser, runtime tests | To verify | Lexer, Parser, and Frontend | Planned |
| First-class callables | Scanner, parser, runtime tests | To verify | Lexer, Parser, and Frontend | Planned |
| Constructor promotion | Parser, compiler, object tests | To verify | Parser and Frontend | Planned |

## Later Derived Artifacts

Later layers may derive structured artifacts from the pinned sources:

- `references/derived/tokens.json`
- `references/derived/parser-rules.json`
- `references/derived/scanner-states.json`
- `references/derived/syntax-features.json`

These artifacts are not produced by Foundation unless a later work item explicitly
adds extraction scripts. Foundation only records the source contract and metadata
foundation.

Future generated syntax fixtures will live under:

- `tests/fixtures/lexer`
- `tests/fixtures/parser`

## Planned Coverage Areas

| Area | Reference source | Later gate |
| --- | --- | --- |
| Tokens | `zend_language_scanner.l`, manual token list, `token_get_all()` | Token differential fixtures. |
| Parser acceptance | `zend_language_parser.y`, `php -l` | Accept/reject fixture parity. |
| Parse diagnostics | `php -l`, `.phpt` tests | Error class and position approximation. |
| Compile-time checks | `zend_compile.h`, compiler sources, `.phpt` tests | Compile-error fixture parity. |
| Runtime behavior | `zend_vm_def.h`, Zend tests, broader `.phpt` tests | Runtime differential tests. |

## Coverage Status

| Feature | Referenzquelle | Tests vorhanden? | Layer | Status |
| --- | --- | --- | --- | --- |
| Reference pin | `references/php-src.lock.toml` | Yes | Foundation | Done |
| Scanner source catalog | `Zend/zend_language_scanner.l` | Metadata only | Foundation | Done |
| Parser source catalog | `Zend/zend_language_parser.y` | Metadata only | Foundation | Done |
| Token catalog | `token_get_all()` and manual token list | No generated fixtures yet | Lexer | Planned |
| Parser rule index | `Zend/zend_language_parser.y` | No generated index yet | Parser | Planned |
| Scanner state index | `Zend/zend_language_scanner.l` | No generated index yet | Lexer | Planned |
| Syntax feature catalog | PHP 8.5 source and tests | No generated catalog yet | Lexer and Parser | Planned |

## Foundation Boundary

Foundation does not copy or reimplement PHP grammar logic. It only pins PHP
`8.5.7`, records the critical files, and prepares metadata extraction for later
work.

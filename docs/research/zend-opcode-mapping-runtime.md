# Runtime IR and Zend Opcode Mapping Notes

This document is a compatibility research aid. It does not define the Runtime
execution contract and it does not claim Zend bytecode compatibility. The local
contract remains the register-based IR in `php_ir` plus the deterministic VM
behavior tested by Runtime fixtures.

## Reference Sources

- `third_party/php-src/Zend/zend_compile.c`
- `third_party/php-src/Zend/zend_vm_def.h`
- `third_party/php-src/Zend/zend_execute.c`

PHP's compiler and VM encode behavior through Zend opcodes, operands, zvals,
CV slots, temporary variables, object handlers, HashTables, exception state,
and executor globals. Runtime keeps a smaller, source-mapped IR that is easier
to verify while runtime compatibility is still incomplete.

## Mapping Sketch

| Runtime IR | Closest Zend concept | Notes |
| --- | --- | --- |
| `LoadConst` | constant/literal op arrays, `ZEND_QM_ASSIGN`-like temp materialization | Runtime loads interned IR constants into registers. Zend may encode literals directly in opcode operands or use temp variables. |
| `LoadLocal` / `StoreLocal` | CV reads and writes such as `ZEND_ASSIGN`, CV operands | Runtime locals are named slots in the compiled unit. Zend CV storage is coupled to zval reference/COW semantics, which Runtime only models for a local-alias MVP. |
| `AssignDim` | array dimension assignment opcodes such as `ZEND_ASSIGN_DIM` | Runtime performs ordered-map mutation through `PhpArray`. Zend routes this through HashTable/zval helpers, reference separation, and COW. |
| `FetchDim` | dimension fetch opcodes such as `ZEND_FETCH_DIM_R` | Runtime supports the selected MVP read cases and emits structured diagnostics for missing keys. Zend has mode-specific fetch opcodes and warning behavior. |
| `Binary` / `Unary` / `Compare` | arithmetic, cast, concat, comparison opcodes | Runtime groups families into higher-level instruction variants. Zend has many specialized opcodes and helper paths for type juggling. |
| `Call` | `ZEND_INIT_FCALL`, `ZEND_SEND_*`, `ZEND_DO_FCALL` families | Runtime uses a single call instruction with resolved user/builtin/closure targets where possible. Zend separates call setup, argument send, and call execution. |
| `Return` | `ZEND_RETURN` | Runtime returns an optional operand from a frame. Zend return behavior includes reference returns, generator returns, type verification, and VM stack cleanup. |
| `Jump` / `Branch` / `Switch` / `Match` | jump and switch opcode families | Runtime keeps structured targets in basic blocks. Zend op arrays encode opline jumps and specialized dispatch. |
| `Throw` | `ZEND_THROW` plus executor exception state | Runtime throws internal exception values through VM control flow. Zend integrates `Throwable`, stack trace construction, and executor exception handling. |
| `Include` | `ZEND_INCLUDE_OR_EVAL` variants | Runtime has a local include loader with constrained roots. Zend handles include path, stream wrappers, op array compilation, symbol side effects, and error wording. |
| `NewObject` / property and method ops | object creation, fetch, assign, and call opcodes | Runtime supports concrete public-class MVP paths. Zend object handlers cover visibility, inheritance, interfaces, traits, property hooks, magic methods, and autoload. |

## Intentional Level Differences

- Runtime IR is more source-oriented than Zend opcodes. It preserves spans and
  a small number of high-level instructions for auditing and diagnostics.
- Runtime call lowering is coarser than Zend's call setup/send/do-call sequence.
  That avoids encoding Zend VM stack details before the runtime model is ready.
- Runtime arrays and objects are higher-level runtime values. Zend's behavior is
  driven by zval, HashTable, object handlers, refcounting, and COW machinery.
- Runtime exceptions are explicit VM control flow around a small exception
  value. Zend's exception model is tied to `Throwable` objects, executor state,
  and engine stack traces.
- Runtime includes are intentionally narrower. They are file-loader operations,
  not stream-wrapper/opcache/include-path compatible compilation events.

## Runtime semantics and Performance Research Questions

1. Which IR instructions need split read/write/reference modes once references
   and Copy-on-Write become real zval-like storage?
2. Should array and object operations stay high-level with runtime helper calls,
   or should Runtime semantics introduce lower-level helper instructions for COW,
   visibility checks, and property hooks?
3. How much of Zend's call setup sequence is needed to reproduce argument
   evaluation, by-reference parameters, variadics, named arguments, and stack
   traces without copying Zend opcode layout?
4. Can source-mapped IR keep stable diagnostic quality while adding runtime
   specialization or quickening in later layers?
5. Which `.phpt` groups should become the first compatibility oracle for each
   Zend opcode family?
6. If Performance adds optimization/JIT work, should it target this local IR, a
   lowered helper-call form, or a separate optimized representation?

## Compatibility Boundary

The Runtime VM does not consume, emit, or execute Zend op arrays. Mapping to
Zend concepts is useful for finding missing behavior and choosing PHPT coverage,
but the executable contract remains local and must be validated through the
Runtime gates.

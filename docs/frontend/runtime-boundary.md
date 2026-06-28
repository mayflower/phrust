# Semantic Frontend to Runtime Boundary

Runtime should start only after Semantic frontend provides a stable semantic frontend
contract.

## Expected Inputs

- `FrontendResult`
- `SemanticModule`
- `HirModule`
- declaration table
- import table
- symbol table
- name-resolution output
- scope arena
- type arena
- constant-expression table
- attribute table
- diagnostic list
- HIR-to-source source maps

## Runtime Consumers

Bytecode or IR lowering should consume HIR statements, expressions,
function-like bodies, class-like metadata, constants, attributes, type
metadata, and deferred include/eval nodes. It should not inspect parser events
or mutate CST construction.

Concrete entry points:

- Use `php_semantics::query::frontend::analyze_file` as the high-level API.
- Read the module ID through `FrontendResult::module().module_id()`.
- Read HIR arenas through `FrontendResult::database().module(module_id)`.
- Use source maps from `FrontendDatabase::source_map()` for byte-span
  attribution.
- Treat `semantic_diagnostics` as a pre-bytecode gate. Error diagnostics mean
  Runtime should not lower executable code for that file.

## Risks Carried Forward

- PHP references and copy-on-write behavior are not modeled in Semantic frontend.
- Include/eval effects are deferred metadata only. Runtime must decide how to
  model dynamic include paths, current-scope execution, symbols defined by
  include/require, and eval runtime parsing/security rules.
- Autoloading and cross-file symbol resolution are absent.
- Exact PHP error text compatibility is incomplete.
- Runtime semantics for property hooks, asymmetric visibility, clone-with, and
  PHP 8.5 features remain later work.
- CFG-level checks such as full goto boundary validation remain later work.
- Cross-file duplicate class/interface/trait/enum behavior must be reconciled
  with autoload and file loading before bytecode linking.

## Required Pre-Bytecode Checks Still Missing

- full inheritance validation and member compatibility
- cross-file symbol linking
- autoload-aware class resolution
- exact callable validation
- include/require/eval execution model
- runtime constant lookup and fallback behavior
- exact PHP error-message compatibility where user-visible wording matters

# Reflection Current Status

Generated: `2026-06-28`

The Reflection PHPT gate is split into eight selected submodules. The aggregate
`reflection` selected manifest runs one focused generated fixture from each
submodule while the full upstream `ext/reflection/tests` backlog remains tracked
by the committed full PHPT baseline and known-gap catalog.

| Submodule | Selected PHPTs | Current selected result | Main covered metadata | Known gaps |
| --- | ---: | --- | --- | --- |
| `reflection.functions` | 1 | PASS | internal/userland function names, counts, return type, extension | callable invocation, doc comments |
| `reflection.parameters` | 1 | PASS | generated arginfo names, optionality, variadic, by-ref, simple types | default constants, complex ReflectionType parity |
| `reflection.classes` | 1 | PASS | names, namespace, flags, parent, interfaces, member counts | full internal class parity, autoload-sensitive construction |
| `reflection.methods` | 1 | PASS | declaring class, visibility, static/final, parameters, return type | invocation, complete modifier parity |
| `reflection.properties` | 1 | PASS | declaring class, visibility, static, readonly, type | private value mutation, property-hook object parity |
| `reflection.attributes` | 1 | PASS | names, arguments, repeat metadata, class/method/property/parameter targets | `newInstance`, full target validation |
| `reflection.enums` | 1 | PASS | backed enum type, cases, backed case values | serialization and exact exception parity |
| `reflection.extensions` | 1 | PASS | extension name, functions, classes, owner metadata | dependencies, INI matrix, module globals, Zend ABI |

## Required Gates

- `nix develop -c just phpt-dev-module MODULE=reflection`
- `nix develop -c just diff-spl-reflection`
- `nix develop -c just verify-stdlib`
- `nix develop -c just verify-phpt`

The selected-result column records the expected gate state of the committed
fixtures; validation output from the current checkout is reported separately by
the running task.

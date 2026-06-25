# PHPT Module Plan

This directory contains the functional module plan for PHPT-driven runtime completion. The order is based on core language dependencies, failure volume, and expected leverage across later modules.

| Priority | Module | Corpus | PASS | SKIP | FAIL | BORK | Next step |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 1 | [phpt.foundation](phpt.foundation.md) | 0 | 0 | 0 | 0 | 0 | Keep committed baseline, corpus, and source-integrity manifests consistent. |
| 2 | [phpt.runner](phpt.runner.md) | 0 | 0 | 0 | 0 | 437 | Reduce runner-owned BORKs before attributing failures to the engine. |
| 3 | [phpt.cli](phpt.cli.md) | 350 | 2 | 0 | 274 | 0 | Keep target invocation deterministic for upstream PHPT execution. |
| 4 | [zend.basic](zend.basic.md) | 3509 | 274 | 1 | 3226 | 0 | Close basic execution and scalar-output failures first. |
| 5 | [operators.conversions](operators.conversions.md) | 129 | 11 | 0 | 118 | 0 | Stabilize scalar conversion and invalid-operand diagnostics. |
| 6 | [diagnostics.output](diagnostics.output.md) | 0 | 0 | 0 | 0 | 0 | Centralize runtime diagnostic rendering and continuation semantics. |
| 7 | [strings.literals](strings.literals.md) | 9 | 0 | 0 | 9 | 0 | Separate frontend literal gaps from runtime string builtin gaps. |
| 8 | [arrays.references](arrays.references.md) | 273 | 13 | 0 | 260 | 0 | Close array data-model and reference/COW gaps before array builtins. |
| 9 | [functions.callables](functions.callables.md) | 887 | 46 | 2 | 817 | 0 | Use generated arginfo for builtin arity and parameter metadata. |
| 10 | [objects.classes](objects.classes.md) | 2136 | 136 | 0 | 1999 | 0 | Stabilize constructor/property/method basics before magic behavior. |
| 11 | [filesystem.streams](filesystem.streams.md) | 1194 | 28 | 4 | 1100 | 0 | Keep filesystem policy root-constrained and deterministic. |
| 12 | [standard.arrays](standard.arrays.md) | 821 | 85 | 0 | 735 | 0 | Implement array builtins after array data model gaps are closed. |
| 13 | [standard.strings](standard.strings.md) | 727 | 81 | 0 | 621 | 0 | Close common binary-safe string functions against Reference PHP. |
| 14 | [standard.math](standard.math.md) | 171 | 8 | 0 | 163 | 0 | Use php-src arginfo and Reference PHP for edge-case numeric behavior. |
| 15 | [standard.variables](standard.variables.md) | 446 | 8 | 2 | 435 | 0 | Stabilize var_dump/print_r/serialization-adjacent value rendering. |
| 16 | [standard.serialization](standard.serialization.md) | 126 | 10 | 0 | 115 | 0 | Implement serialization after arrays/objects are stable. |
| 17 | [json](json.md) | 88 | 9 | 0 | 79 | 0 | Close request-local JSON error state and common flags. |
| 18 | [pcre](pcre.md) | 165 | 36 | 1 | 126 | 0 | Use PCRE2 while documenting unsupported modifier/callout gaps. |
| 19 | [date](date.md) | 687 | 11 | 1 | 675 | 0 | Stabilize timezone persistence and common formatting/parsing. |
| 20 | [spl](spl.md) | 520 | 26 | 1 | 493 | 0 | Build on stable object, array, iterator, and filesystem layers. |
| 21 | [reflection](reflection.md) | 304 | 9 | 0 | 295 | 0 | Expose generated arginfo and semantic metadata through Reflection APIs. |
| 22 | [extension.policy](extension.policy.md) | 9006 | 263 | 52 | 8434 | 0 | Classify extension failures without hiding them from full regression. |

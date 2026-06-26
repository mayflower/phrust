
## Prompt 1 — PHPT-Bookkeeping stabilisieren

```text
Du arbeitest im Repository `mayflower/phrust` auf dem aktuellen `main`.

Aktueller Stand:
Der letzte Full-Regression-Run ist:
`target/phpt-work/full-runs/20260624T125543Z`
Ergebnis:
- 21.548 PHPTs run
- 1.056 PASS
- 64 SKIP
- 20.428 known non-green
- keine rejected new regression fingerprints

Ziel:
Stabilisiere zuerst das PHPT-Bookkeeping, bevor Runtime-Features implementiert werden.

Aufgaben:
1. Prüfe die aktuellen PHPT-Dateien:
   - `docs/phpt/reports/full-baseline.md`
   - `tests/phpt/manifests/full-known-failures.jsonl`
   - `tests/phpt/manifests/phpt-corpus.jsonl`
   - `scripts/phpt/full_regression.sh`
   - `crates/php_phpt_tools/src/main.rs`
2. Stelle sicher, dass die maschinenlesbare bekannte Failure-Baseline vollständig und konsistent mit dem Markdown-Report ist.
3. Falls `full-known-failures.jsonl` leer oder unvollständig ist, baue ein robustes Baseline-Format:
   - entweder eine vollständige JSONL-Datei
   - oder Shards nach Modul unter `tests/phpt/manifests/full-known-failures/`
4. Ergänze eine Validierung:
   - Der Markdown-Report darf keine non-green Zahl ausweisen, wenn die maschinenlesbare Failure-Baseline leer ist.
   - `just verify-phpt` muss diese Konsistenz prüfen.
5. Keine Runtime-Features implementieren.
6. Keine `target/`-Artefakte committen.
7. Keine Originaldateien aus `php-src` verändern.

Akzeptanzkriterien:
- `nix develop -c cargo test -p php_phpt_tools`
- `nix develop -c just verify-phpt`
- PHPT-Baseline ist versioniert, nachvollziehbar und für spätere Regression-Vergleiche verwendbar.

Am Ende:
Gib einen kurzen Bericht:
- Welche Baseline-Dateien existieren jetzt?
- Wie viele known non-green Fingerprints sind maschinenlesbar erfasst?
- Welche invariants prüft `verify-phpt`?
```

---

## Prompt 2 — PHPT-Triage-Report erzeugen

```text
Du arbeitest auf dem Ergebnis von Prompt 1 weiter.

Ziel:
Erzeuge eine stabile PHPT-Triage-Schicht, die aus der Full-Baseline automatisch ableitet, welche Module in welcher Reihenfolge bearbeitet werden sollen.

Aufgaben:
1. Ergänze `php_phpt_tools` um einen Befehl, z.B.:
   `php-phpt-tools triage`
2. Der Befehl soll aus der aktuellen Full-Baseline erzeugen:
   - Top failing modules
   - Top failure clusters
   - Top unsupported feature guesses
   - BORK-Unterklassen
   - Anzahl PASS/SKIP/FAIL/BORK pro Modul
   - Kandidatenliste der nächsten Module
3. Schreibe die Ergebnisse nach:
   - `docs/phpt/reports/triage.md`
   - `tests/phpt/manifests/module-priority.json`
4. Ergänze ein `just`-Target:
   `just phpt-triage`
5. Keine Engine-Features implementieren.

Akzeptanzkriterien:
- `nix develop -c cargo test -p php_phpt_tools`
- `nix develop -c just phpt-triage`
- `nix develop -c just verify-phpt`

Am Ende:
Gib die Top 20 Module aus, sortiert nach:
1. Core-first Relevanz
2. Anzahl Failures
3. erwarteter Hebelwirkung auf andere Module
```

---

## Prompt 3 — Modulplan für die gesamte PHPT-Arbeit erzeugen

```text
Du arbeitest auf dem Ergebnis von Prompt 2 weiter.

Ziel:
Erzeuge einen linearen, gepflegten Modulplan für die vollständige PHPT-Abarbeitung.

Aufgaben:
1. Erzeuge `docs/phpt/modules/README.md`.
2. Erzeuge für jedes priorisierte Modul eine Datei:
   `docs/phpt/modules/<module>.md`
3. Jedes Modul-Dokument enthält:
   - Modulname
   - Priorität
   - Scope
   - Nicht-Scope
   - relevante PHPT-Pfade
   - relevante php-src Source-Orte
   - aktuelle PASS/SKIP/FAIL/BORK-Zahlen
   - erwartete Ziel-Gates
   - bekannte Gaps
   - nächster konkreter Schritt
4. Erzeuge oder aktualisiere:
   `tests/phpt/manifests/modules/<module>.json`
5. Der erste lineare Plan soll diese Reihenfolge verwenden:
   1. phpt.foundation
   2. phpt.runner
   3. phpt.cli
   4. zend.basic
   5. operators.conversions
   6. diagnostics.output
   7. strings.literals
   8. arrays.references
   9. functions.callables
   10. objects.classes
   11. filesystem.streams
   12. standard.arrays
   13. standard.strings
   14. standard.math
   15. standard.variables
   16. standard.serialization
   17. json
   18. pcre
   19. date
   20. spl
   21. reflection
   22. extension.policy
6. Keine Runtime-Features implementieren.

Akzeptanzkriterien:
- `nix develop -c just phpt-triage`
- `nix develop -c just verify-phpt`

Am Ende:
Gib den finalen linearen Modulplan aus.
```

---

## Prompt 4 — PHPT-Runner BORKs reduzieren

```text
Du arbeitest auf dem Ergebnis von Prompt 3 weiter.

Ziel:
Reduziere PHPT-BORKs, bevor Engine-Features implementiert werden.

Aufgaben:
1. Analysiere alle aktuellen BORKs aus der Baseline.
2. Klassifiziere BORKs sauber:
   - unsupported section
   - malformed PHPT
   - unsupported FILE_EXTERNAL
   - unsupported EXPECT variant
   - unsupported ENV/INI/ARGS/STDIN/CLEAN behavior
   - missing target CLI capability
   - extension policy issue
3. Implementiere im PHPT-Runner Unterstützung für generische PHPT-Features:
   - SKIPIF
   - CLEAN
   - EXPECT
   - EXPECTF
   - EXPECTREGEX
   - XFAIL
   - INI
   - ENV
   - ARGS
   - STDIN
   - FILEEOF
   - FILE_EXTERNAL, soweit sicher möglich
4. Erzeuge Runner-Fixtures für jede unterstützte Sektion.
5. Aktualisiere:
   - `docs/phpt/README.md`
   - `docs/phpt/full-phpt-gate.md`
   - `docs/phpt/reports/triage.md`
6. Keine VM-/Runtime-Features implementieren, außer minimale Runner-Kompatibilität es zwingend erfordert.

Akzeptanzkriterien:
- `nix develop -c cargo test -p php_phpt_tools`
- `nix develop -c just phpt-runner-smoke`
- `nix develop -c just verify-phpt`
- Optional, falls lokal machbar: `nix develop -c just phpt-full-regression`

Am Ende:
Gib BORK vorher/nachher aus.
```

---

## Prompt 5 — PHP-kompatible Target-CLI herstellen

```text
Du arbeitest auf dem Ergebnis von Prompt 4 weiter.

Ziel:
Baue eine PHP-CLI-kompatible Target-Binary für PHPT, ohne die bestehende Developer-CLI `php-vm` zu beschädigen.

Aufgaben:
1. Füge eine Binary `phrust-php` hinzu.
2. Unterstütze mindestens:
   - `phrust-php -v`
   - `phrust-php -r 'code'`
   - `phrust-php -n`
   - `phrust-php -d key=value`
   - `phrust-php file.php arg1 arg2`
   - STDIN
   - `$argc`
   - `$argv`
   - `$_SERVER['argc']`
   - `$_SERVER['argv']`
3. Mappe Exit-Codes PHP-kompatibler.
4. Integriere INI-Optionen aus PHPT in RuntimeContext.
5. Passe PHPT-Binary-Discovery an, sodass `phrust-php` bevorzugt wird.
6. `php-vm` bleibt Developer-CLI und wird nicht entfernt.

Akzeptanzkriterien:
- `nix develop -c cargo test -p php_vm_cli`
- `nix develop -c cargo build -p php_vm_cli --bin phrust-php`
- `TARGET_PHP=target/debug/phrust-php PHPT_TARGET_MODE=php-cli nix develop -c just phpt-target-smoke`
- `TARGET_PHP=target/debug/phrust-php PHPT_TARGET_MODE=php-cli nix develop -c just phpt-runner-smoke`

Am Ende:
Dokumentiere in `docs/phpt/binary-discovery.md`, welche CLI-Flächen unterstützt werden und welche noch fehlen.
```

---

## Prompt 6 — Runtime-Builtin-Struktur aufräumen

```text
Du arbeitest auf dem Ergebnis von Prompt 5 weiter.

Ziel:
Räume die Runtime-/Builtin-Struktur auf, bevor Standard-Library-Funktionen massiv erweitert werden.

Aufgaben:
1. Refactore `crates/php_runtime/src/builtins.rs` in ein sauberes Modul-Layout.
2. Zielstruktur:
   - `crates/php_runtime/src/builtins/mod.rs`
   - `crates/php_runtime/src/builtins/context.rs`
   - `crates/php_runtime/src/builtins/error.rs`
   - `crates/php_runtime/src/builtins/registry.rs`
   - `crates/php_runtime/src/builtins/signatures.rs`
   - `crates/php_runtime/src/builtins/modules/core.rs`
   - `crates/php_runtime/src/builtins/modules/arrays.rs`
   - `crates/php_runtime/src/builtins/modules/strings.rs`
   - `crates/php_runtime/src/builtins/modules/math.rs`
   - `crates/php_runtime/src/builtins/modules/filesystem.rs`
   - `crates/php_runtime/src/builtins/modules/streams.rs`
   - `crates/php_runtime/src/builtins/modules/json.rs`
   - `crates/php_runtime/src/builtins/modules/pcre.rs`
   - `crates/php_runtime/src/builtins/modules/date.rs`
   - `crates/php_runtime/src/builtins/modules/spl.rs`
   - `crates/php_runtime/src/builtins/modules/reflection.rs`
3. Keine Verhaltensänderung.
4. Registry soll aus Modul-Slices zusammengesetzt werden.
5. Schreibe eine kurze Architektur-Doku:
   `docs/runtime-builtin-modules.md`

Akzeptanzkriterien:
- `nix develop -c cargo fmt --all --check`
- `nix develop -c cargo clippy --workspace --all-targets -- -D warnings`
- `nix develop -c cargo test -p php_runtime`
- `nix develop -c just verify-runtime`
- `nix develop -c just verify-stdlib`

Am Ende:
Gib eine Mapping-Tabelle aus:
alte Builtin-Bereiche → neue Moduldateien.
```

---

## Prompt 7 — php-src Arginfo und Signaturen importieren

```text
Du arbeitest auf dem Ergebnis von Prompt 6 weiter.

Ziel:
Nutze php-src systematisch als Oracle für Funktionssignaturen, Arity, Parameter-Namen, By-Ref, Variadics und Reflection-Metadaten.

Aufgaben:
1. Prüfe bestehende Generatoren:
   - `scripts/stdlib/generate_arginfo.py`
   - `just generate-arginfo`
   - `just phpt-source-index`
   - `just phpt-source-lookup`
2. Erzeuge oder verbessere ein generiertes Rust-Metadatenformat:
   - Funktionen
   - Klassen
   - Methoden
   - Konstanten
   - Parameter
   - Defaults
   - Nullable/Union/Intersection, soweit extrahierbar
   - By-ref
   - Variadic
   - Extension-/Modul-Owner
3. Stelle sicher:
   - keine php-src C-Implementierung kopieren
   - nur Signaturen, Namen, Metadaten und Source-Verweise extrahieren
4. BuiltinRegistry und Reflection sollen diese Metadaten nutzen.
5. Overrides müssen in einer separaten Datei liegen und begründet sein.

Akzeptanzkriterien:
- `nix develop -c just phpt-source-index`
- `nix develop -c just generate-arginfo`
- `nix develop -c cargo test -p php_runtime`
- `nix develop -c just diff-spl-reflection`
- `nix develop -c just verify-stdlib`

Am Ende:
Gib aus:
- Anzahl importierter Funktionen
- Anzahl importierter Klassen
- Anzahl importierter Methoden
- Anzahl Overrides
- bekannte Extractor-Gaps
```

---

## Prompt 8 — `zend.basic` grün machen

```text
Du arbeitest auf dem Ergebnis von Prompt 7 weiter.

Ziel:
Mache das Modul `zend.basic` grün.

Scope:
- Top-level execution
- scalar literals
- numeric literal separators
- echo
- print
- statement sequencing
- top-level return
- top-level exit
- basic var_dump output

Aufgaben:
1. Prüfe:
   - `docs/phpt/modules/zend.basic.md`
   - `tests/phpt/manifests/modules/zend.basic.json`
   - `tests/phpt/manifests/modules/zend.basic.selected.jsonl`
   - `tests/phpt/generated/zend.basic/`
2. Führe aus:
   `nix develop -c just phpt-module MODULE=zend.basic`
3. Nutze Reference PHP und Original-PHPTs als Oracle.
4. Implementiere nur fehlende Basisfunktionalität für dieses Modul.
5. Wenn ein Original-PHPT zu groß ist, erzeuge ein minimiertes Generated PHPT mit Provenance.
6. Aktualisiere Modul-Doku und Report.

Akzeptanzkriterien:
- `nix develop -c just phpt-module MODULE=zend.basic`
- `nix develop -c just verify-runtime`
- `nix develop -c just verify-phpt`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib aus:
- PASS/SKIP/FAIL/BORK für `zend.basic`
- reduzierte Full-Baseline-Fingerprints
- neue Fingerprints, falls vorhanden
```

---

## Prompt 9 — `operators.conversions` grün machen

```text
Du arbeitest auf dem Ergebnis von Prompt 8 weiter.

Ziel:
Mache das Modul `operators.conversions` grün.

Scope:
- arithmetic
- bitwise
- comparison
- boolean conversion
- numeric string behavior
- concat
- assignment operators
- increment/decrement
- invalid operand warnings

Aufgaben:
1. Prüfe:
   - `docs/phpt/modules/operators.conversions.md`
   - `tests/phpt/manifests/modules/operators.conversions.json`
   - `tests/phpt/manifests/modules/operators.conversions.selected.jsonl`
   - `tests/phpt/generated/operators.conversions/`
2. Führe aus:
   `nix develop -c just phpt-module MODULE=operators.conversions`
3. Fixe Verhalten in:
   - numeric string parsing
   - scalar conversion
   - comparison
   - concat
   - inc/dec
   - invalid operand diagnostics
4. Jede Änderung an Konvertierungslogik braucht eine PHPT- oder Runtime-Semantics-Fixture.
5. Keine unrelated Refactors.

Akzeptanzkriterien:
- `nix develop -c just phpt-module MODULE=operators.conversions`
- `nix develop -c just verify-runtime`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib aus:
- Modulstatus
- welche Konvertierungsfälle neu unterstützt werden
- welche bekannten Gaps bleiben
```

---

## Prompt 10 — Warning-, Error- und Output-Kanal PHP-kompatibler machen

```text
Du arbeitest auf dem Ergebnis von Prompt 9 weiter.

Ziel:
Reduziere die Failure-Cluster `runtime-error-or-diagnostic` und `runtime-output-mismatch`.

Aufgaben:
1. Analysiere aktuelle Failure-Fingerprints mit:
   - runtime-error-or-diagnostic
   - runtime-output-mismatch
2. Baue eine zentrale Error-/Warning-Ausgabeschicht.
3. Unterstütze:
   - Warning formatting
   - Notice/Warning/Fatal channel
   - display_errors
   - error_reporting
   - Datei-/Zeilen-Ausgabe
   - Fortsetzung nach Warning
   - Fatal exit behavior
4. Entferne ad-hoc Warning-Strings aus Runtime und VM, soweit möglich.
5. Erzeuge PHPTs für:
   - undefined variable
   - invalid operand
   - array to string
   - builtin arity
   - builtin type error
6. Aktualisiere bekannte Gaps für exakte PHP-Wording-Parität.

Akzeptanzkriterien:
- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just verify-runtime`
- `nix develop -c just phpt-module MODULE=zend.basic`
- `nix develop -c just phpt-module MODULE=operators.conversions`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib aus:
- Cluster vorher/nachher
- betroffene Failure-Fingerprints
- verbleibende Wording-Gaps
```

---

## Prompt 11 — String Literals und `standard.strings` vorbereiten

```text
Du arbeitest auf dem Ergebnis von Prompt 10 weiter.

Ziel:
Behebe String-Literal- und String-Runtime-Gaps, bevor `standard.strings` vollständig bearbeitet wird.

Scope:
- single quoted strings
- double quoted strings
- escape decoding
- NUL escapes
- binary-safe strings
- heredoc/nowdoc execution
- string interpolation, soweit für PHPTs notwendig
- common string output behavior

Aufgaben:
1. Führe das Modul aus:
   `nix develop -c just phpt-module MODULE=standard.strings`
2. Identifiziere, welche Failures durch Lexer/Parser/Semantik entstehen und welche durch Runtime/Builtins.
3. Fixe zuerst Source-Literal-Decoding.
4. Danach common string functions, aber nur soweit sie im Modulbatch benötigt werden.
5. Nutze php-src und Reference PHP als Oracle.
6. Erzeuge minimierte Generated PHPTs für neue Edge Cases.

Akzeptanzkriterien:
- `nix develop -c just verify-frontend`
- `nix develop -c just verify-runtime`
- `nix develop -c just phpt-module MODULE=standard.strings`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib aus:
- Parser/Semantik-Fixes
- Runtime/String-Fixes
- verbleibende String-Gaps
```

---

## Prompt 12 — Arrays, References, COW und Foreach

```text
Du arbeitest auf dem Ergebnis von Prompt 11 weiter.

Ziel:
Mache Array- und Reference-Grundlagen PHPT-tauglich.

Scope:
- ordered PHP arrays
- int/string keys
- key conversion
- append
- unset
- isset/empty
- array copy-on-write
- array element references
- foreach by value
- foreach by reference MVP
- common array debug output

Aufgaben:
1. Führe aus:
   `nix develop -c just phpt-module MODULE=standard.arrays`
2. Analysiere Failures nach Untergruppen:
   - key conversion
   - mutation order
   - COW
   - references
   - foreach
   - builtin behavior
3. Fixe zuerst Runtime-Datenmodell und VM-Operationen.
4. Danach Builtins.
5. Jede COW-/Reference-Änderung braucht kleine Tests.
6. Aktualisiere Runtime Known Gaps.

Akzeptanzkriterien:
- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just verify-runtime`
- `nix develop -c just phpt-module MODULE=standard.arrays`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib aus:
- array behavior coverage
- reference/COW coverage
- verbleibende Gaps
```

---

## Prompt 13 — Functions, Callables, Arity und Type Coercion

```text
Du arbeitest auf dem Ergebnis von Prompt 12 weiter.

Ziel:
Stabilisiere Funktionsaufrufe, Callables, Arity-Handling und Type-Coercion.

Scope:
- user functions
- closures
- arrow functions
- first-class callables
- dynamic calls MVP
- call_user_func
- call_user_func_array
- default parameters
- variadics
- by-ref parameters
- return types
- weak/strict scalar type coercion
- builtin arity/type handling aus Arginfo

Aufgaben:
1. Erzeuge oder aktualisiere Modul:
   `zend.functions`
2. Führe aus:
   `nix develop -c just phpt-module MODULE=zend.functions`
3. Nutze generated arginfo aus Prompt 7 für Builtins.
4. Implementiere keine handgeschriebenen Sonderfälle, wenn Arginfo nutzbar ist.
5. Ergänze PHPTs für:
   - too few args
   - too many args
   - by-ref mismatch
   - weak coercion
   - strict_types
   - variadic packing
   - callable invocation

Akzeptanzkriterien:
- `nix develop -c just verify-runtime`
- `nix develop -c just verify-stdlib`
- `nix develop -c just phpt-module MODULE=zend.functions`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib aus:
- welche Callables funktionieren
- welche Type-Coercion-Regeln funktionieren
- welche Gaps bleiben
```

---

## Prompt 14 — Objects, Classes, Magic, Traits, Enums

```text
Du arbeitest auf dem Ergebnis von Prompt 13 weiter.

Ziel:
Stabilisiere das Objektmodell für PHPTs.

Scope:
- class table
- object identity
- constructors
- properties
- methods
- visibility
- static methods/properties
- magic methods
- traits MVP
- enums MVP
- clone
- clone-with
- property hooks MVP

Aufgaben:
1. Erzeuge oder aktualisiere Modul:
   `zend.objects`
2. Führe aus:
   `nix develop -c just phpt-module MODULE=zend.objects`
3. Teile Failures nach:
   - construction
   - property read/write
   - method dispatch
   - visibility
   - magic
   - trait composition
   - enum behavior
   - clone/clone-with
4. Implementiere in dieser Reihenfolge:
   1. constructor/property/method basics
   2. visibility
   3. static access
   4. magic methods
   5. clone/clone-with
   6. traits/enums
   7. property hooks
5. Aktualisiere Reflection-relevante Gaps, aber löse Reflection erst später.

Akzeptanzkriterien:
- `nix develop -c just verify-runtime`
- `nix develop -c just phpt-module MODULE=zend.objects`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib aus:
- Objektmodell-Abdeckung
- explizite Gaps
- betroffene Full-Baseline-Reduktion
```

---

## Prompt 15 — Filesystem, Streams, Resources, Include

```text
Du arbeitest auf dem Ergebnis von Prompt 14 weiter.

Ziel:
Stabilisiere lokale Filesystem-, Stream-, Resource- und Include-Semantik.

Scope:
- local files
- directories
- cwd
- include_path
- include/require
- file resources
- stream metadata
- php://memory/temp MVP
- warnings for missing/invalid files
- deterministic filesystem capabilities

Aufgaben:
1. Erzeuge oder aktualisiere Modul:
   `filesystem.streams`
2. Führe aus:
   `nix develop -c just phpt-module MODULE=filesystem.streams`
3. Nutze root-constrained deterministic filesystem policy.
4. Implementiere keine Netzwerkstreams.
5. Implementiere keine PHAR-Streams in diesem Prompt.
6. Fixe BuiltinContext-Persistenz für CWD, include_path, Ressourcen und last-error State, falls nötig.
7. Ergänze PHPTs für lokale Dateioperationen und Include-Scope.

Akzeptanzkriterien:
- `nix develop -c just verify-runtime`
- `nix develop -c just verify-stdlib`
- `nix develop -c just phpt-module MODULE=filesystem.streams`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib aus:
- Filesystem-Funktionen, die PHPT-kompatibel sind
- Stream-Gaps
- Include-Gaps
```

---

## Prompt 16 — `ext/standard` Kernmodule grün machen

```text
Du arbeitest auf dem Ergebnis von Prompt 15 weiter.

Ziel:
Bearbeite die wichtigsten `ext/standard`-Module nach der stabilisierten Runtime-Basis.

Module in dieser Reihenfolge:
1. standard.arrays
2. standard.strings
3. standard.math
4. standard.variables
5. standard.output
6. standard.serialization
7. standard.url-html

Aufgaben pro Modul:
1. Erzeuge oder aktualisiere Modulmanifest.
2. Führe aus:
   `nix develop -c just phpt-module MODULE=<module>`
3. Nutze php-src Arginfo für Signaturen.
4. Nutze Original-PHPTs und Reference PHP als Oracle.
5. Implementiere nur Modulscope.
6. Erzeuge minimierte Generated PHPTs für Regressionen.
7. Aktualisiere Modulreport.
8. Nach jedem Modul:
   `nix develop -c just phpt-full-regression`

Akzeptanzkriterien am Ende:
- `nix develop -c just verify-stdlib`
- Alle genannten Module sind grün oder haben nur explizit dokumentierte Policy-Gaps.
- Full-PHPT erzeugt keine neuen Failure-Fingerprints.

Am Ende:
Gib Tabelle aus:
- Modul
- vorher FAIL/BORK
- nachher FAIL/BORK
- reduzierte Fingerprints
- verbleibende Gaps
```

---

## Prompt 17 — JSON grün machen

```text
Du arbeitest auf dem Ergebnis von Prompt 16 weiter.

Ziel:
Mache JSON-PHPTs grün.

Scope:
- json_encode
- json_decode
- json_last_error
- json_last_error_msg
- common flags
- invalid UTF-8 policy
- depth handling
- exceptions with JSON_THROW_ON_ERROR
- arrays/objects interaction
- JsonSerializable, soweit Runtime-Objektmodell bereit ist

Aufgaben:
1. Erzeuge oder aktualisiere Modul:
   `json`
2. Führe aus:
   `nix develop -c just phpt-module MODULE=json`
3. Fixe request-local JSON last-error persistence.
4. Nutze Reference PHP für Flag-Kombinationen.
5. Dokumentiere bewusst nicht implementierte Edge Cases.

Akzeptanzkriterien:
- `nix develop -c just diff-json-pcre-date`
- `nix develop -c just phpt-module MODULE=json`
- `nix develop -c just verify-stdlib`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib JSON-Coverage und Gaps aus.
```

---

## Prompt 18 — PCRE grün machen

```text
Du arbeitest auf dem Ergebnis von Prompt 17 weiter.

Ziel:
Mache PCRE-PHPTs grün, soweit die vorhandene PCRE2-Basis reicht.

Scope:
- preg_match
- preg_match_all
- preg_replace
- preg_replace_callback, soweit callable dispatch bereit ist
- preg_split
- preg_grep
- preg_quote
- preg_last_error
- preg_last_error_msg
- offsets
- captures
- common modifiers

Aufgaben:
1. Erzeuge oder aktualisiere Modul:
   `pcre`
2. Führe aus:
   `nix develop -c just phpt-module MODULE=pcre`
3. Fixe request-local PCRE last-error persistence.
4. Nutze Reference PHP für modifier behavior und warnings.
5. Keine vollständige PCRE-Neuimplementierung; PCRE2 nutzen.
6. Dokumentiere Callout/JIT/backtracking-limit Gaps.

Akzeptanzkriterien:
- `nix develop -c just diff-json-pcre-date`
- `nix develop -c just phpt-module MODULE=pcre`
- `nix develop -c just verify-stdlib`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib PCRE-Coverage und Gaps aus.
```

---

## Prompt 19 — Date/Time grün machen

```text
Du arbeitest auf dem Ergebnis von Prompt 18 weiter.

Ziel:
Mache Date/Time-PHPTs grün, soweit ohne vollständige timelib-Parität möglich.

Scope:
- date
- time
- strtotime MVP
- DateTime
- DateTimeImmutable
- DateTimeZone
- DateInterval MVP
- default timezone persistence
- formatting
- common parsing cases

Aufgaben:
1. Erzeuge oder aktualisiere Modul:
   `date`
2. Führe aus:
   `nix develop -c just phpt-module MODULE=date`
3. Fixe default timezone persistence im VM/BuiltinContext.
4. Nutze Reference PHP für Format- und Parse-Erwartungen.
5. Dokumentiere timelib-natural-language-Gaps ausdrücklich.

Akzeptanzkriterien:
- `nix develop -c just diff-json-pcre-date`
- `nix develop -c just phpt-module MODULE=date`
- `nix develop -c just verify-stdlib`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib Date/Time-Coverage und Gaps aus.
```

---

## Prompt 20 — SPL grün machen

```text
Du arbeitest auf dem Ergebnis von Prompt 19 weiter.

Ziel:
Mache SPL-PHPTs grün, soweit sie für Composer- und Framework-Kompatibilität relevant sind.

Scope:
- Countable
- Iterator
- IteratorAggregate
- ArrayAccess
- ArrayIterator
- RecursiveArrayIterator
- IteratorIterator
- LimitIterator
- EmptyIterator
- AppendIterator
- ArrayObject
- SplFixedArray
- SplObjectStorage
- SplDoublyLinkedList
- SplStack
- SplQueue
- SplFileInfo
- SplFileObject MVP

Aufgaben:
1. Erzeuge oder aktualisiere Modul:
   `spl`
2. Führe aus:
   `nix develop -c just phpt-module MODULE=spl`
3. Nutze generated arginfo.
4. Baue auf dem Objekt-, Array-, Filesystem- und Iterator-Modell auf.
5. Implementiere nur stabile, testbare SPL-MVP-Semantik.
6. Dokumentiere vollständige SPL-API-Gaps.

Akzeptanzkriterien:
- `nix develop -c just diff-spl-reflection`
- `nix develop -c just phpt-module MODULE=spl`
- `nix develop -c just verify-stdlib`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib SPL-Coverage und Gaps aus.
```

---

## Prompt 21 — Reflection grün machen

```text
Du arbeitest auf dem Ergebnis von Prompt 20 weiter.

Ziel:
Mache Reflection-PHPTs grün, soweit Metadaten aus Frontend, Runtime und generated arginfo verfügbar sind.

Scope:
- ReflectionFunction
- ReflectionParameter
- ReflectionClass
- ReflectionMethod
- ReflectionProperty
- ReflectionAttribute
- ReflectionExtension MVP
- Internal function metadata
- Userland function/class metadata
- Attributes metadata
- Enum metadata, falls verfügbar

Aufgaben:
1. Erzeuge oder aktualisiere Modul:
   `reflection`
2. Führe aus:
   `nix develop -c just phpt-module MODULE=reflection`
3. Nutze generated arginfo.
4. Nutze Runtime-/Semantic-Metadaten, nicht ad-hoc Parsing.
5. Implementiere keine fake Reflection-Daten.
6. Dokumentiere bewusst fehlende Reflection-APIs.

Akzeptanzkriterien:
- `nix develop -c just diff-spl-reflection`
- `nix develop -c just phpt-module MODULE=reflection`
- `nix develop -c just verify-stdlib`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib Reflection-Coverage und Gaps aus.
```

---

## Prompt 22 — Extension-Policy statt verrauschter Failures

```text
Du arbeitest auf dem Ergebnis von Prompt 21 weiter.

Ziel:
Klassifiziere nicht-core Extensions sauber, damit Full-PHPT zwischen must-fix, optional und out-of-scope unterscheiden kann.

Extensions:
- dom
- xml
- simplexml
- pdo
- mysqli
- soap
- intl
- mbstring
- gd
- phar
- opcache
- session
- sapi

Aufgaben:
1. Erzeuge `docs/phpt/extension-policy.md`.
2. Pro Extension erfassen:
   - PHPT count
   - aktueller PASS/SKIP/FAIL/BORK
   - required für Core?
   - required für Composer?
   - optional?
   - out-of-scope?
   - braucht Stub?
   - braucht echte Implementierung?
3. Implementiere keine großen Extensions in diesem Prompt.
4. Passe PHPT-Triage so an, dass Extension-Policy im Report sichtbar wird.
5. Extension-PHPTs dürfen nicht stillschweigend verschwinden.

Akzeptanzkriterien:
- `nix develop -c just phpt-triage`
- `nix develop -c just verify-phpt`
- `nix develop -c just phpt-full-regression`

Am Ende:
Gib eine Tabelle:
Extension | Policy | PHPT count | Next action.
```

---

## Prompt 23 — Full-Regression-Integration und neuer Baseline-Stand

```text
Du arbeitest auf dem Ergebnis von Prompt 22 weiter.

Ziel:
Erzeuge einen neuen konsistenten Full-PHPT-Stand nach den bisherigen Modul-Fixes.

Aufgaben:
1. Führe aus:
   - `nix develop -c just check`
   - `nix develop -c just verify-frontend`
   - `nix develop -c just verify-runtime`
   - `nix develop -c just verify-stdlib`
   - `nix develop -c just verify-performance`
   - `nix develop -c just verify-phpt`
2. Führe aus:
   `nix develop -c just phpt-full-regression`
3. Aktualisiere:
   - `docs/phpt/reports/full-baseline.md`
   - maschinenlesbare known-failure baseline
   - `docs/phpt/reports/triage.md`
   - `docs/phpt/modules/README.md`
4. Kein `PHPT_ACCEPT_BASELINE=1`, außer du erklärst exakt, welche neuen Failure-Fingerprints akzeptiert werden sollen.
5. Keine `target/`-Artefakte committen.
6. Keine Original-PHPTs committen.

Akzeptanzkriterien:
- Full run zeigt keine rejected new regression fingerprints.
- PASS-Zahl ist gestiegen oder non-green-Zahl ist gesunken.
- Alle neuen Gaps sind dokumentiert.
- Alle resolved Fingerprints sind im Bericht sichtbar.

Am Ende:
Gib einen finalen Bericht:
- vorher PASS/SKIP/FAIL/BORK
- nachher PASS/SKIP/FAIL/BORK
- resolved known failures
- neue bekannte Gaps
- nächste 10 Module nach Priorität
```

---

## Prompt 24 — Wiederholungsschleife bis Full Green

```text
Du arbeitest auf dem Ergebnis von Prompt 23 weiter.

Ziel:
Wiederhole die PHPT-Modularbeit systematisch, bis die Full-PHPT-Baseline final green oder policy-green ist.

Vorgehen:
1. Führe `nix develop -c just phpt-triage` aus.
2. Wähle das höchste priorisierte Modul mit:
   - vielen Failures
   - geringer Abhängigkeit von out-of-scope Extensions
   - hoher Core-/Composer-Relevanz
3. Erzeuge oder aktualisiere Modulmanifest.
4. Führe `nix develop -c just phpt-module MODULE=<module>` aus.
5. Implementiere nur dieses Modul.
6. Nutze php-src als Oracle:
   - PHPTs
   - Reference PHP output
   - arginfo/stubs
   - source lookup
7. Erzeuge Generated/Minimized PHPTs mit Provenance.
8. Aktualisiere Modulreport.
9. Führe Full Regression aus.
10. Wiederhole.

Harte Regeln:
- Kein Original php-src editieren.
- Kein target committen.
- Keine neuen Baseline-Failures ohne explizite Akzeptanz.
- Keine Modul-Fixes ohne PHPT oder minimierte Regression.
- Keine handgeschriebenen Signaturen, wenn Arginfo verfügbar ist.
- Jede known gap braucht:
  - ID
  - Referenzverhalten
  - aktuelles Rust-Verhalten
  - Fixture oder PHPT-Beispiel
  - geplante Lösungsschicht

Abbruchkriterien:
- Full-PHPT ist strict green.
Oder:
- Full-PHPT ist policy green, und alle verbleibenden non-green PHPTs gehören dokumentierten out-of-scope Extensions oder legitimen platform skips.

Am Ende jedes Zyklus:
Gib aus:
- bearbeitetes Modul
- PASS-Zuwachs
- non-green-Reduktion
- neue/gelöste Gaps
- nächstes Modul
```


<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_STDLIB
// PHP reference: standard-library array helpers are routed through the engine registry.
var_export(array_column([["id" => 7]], "id"));
echo "\n";

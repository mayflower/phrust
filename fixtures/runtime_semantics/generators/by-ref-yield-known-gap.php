<?php
// runtime-semantics: category=generators expect=known_gap known_gap=E_PHP_RUNTIME_GENERATOR_BY_REF_YIELD_GAP
function &gen() {
    $value = 1;
    yield $value;
}

foreach (gen() as &$value) {
    $value = 9;
}

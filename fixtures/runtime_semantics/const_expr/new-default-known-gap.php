<?php
// runtime-semantics: category=const_expr expect=known_gap known_gap=E_PHP_RUNTIME_CONST_EXPR_MATRIX
function new_default_fixture($value = new stdClass()): void
{
    echo gettype($value), "\n";
}
new_default_fixture();

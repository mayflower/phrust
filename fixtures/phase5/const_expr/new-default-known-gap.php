<?php
// phase5-runtime: category=const_expr expect=known_gap known_gap=E_PHP_RUNTIME_CONST_EXPR_MATRIX
function prompt44_new_default($value = new stdClass()): void
{
    echo gettype($value), "\n";
}
prompt44_new_default();

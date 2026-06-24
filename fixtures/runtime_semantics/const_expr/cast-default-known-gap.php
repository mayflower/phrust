<?php
// runtime-semantics: category=const_expr expect=known_gap known_gap=E_PHP_RUNTIME_CONST_EXPR_MATRIX
function cast_default_fixture($value = (int) "42"): void
{
    echo $value, "\n";
}
cast_default_fixture();

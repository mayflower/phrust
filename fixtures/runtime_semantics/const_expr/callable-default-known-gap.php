<?php
// runtime-semantics: category=const_expr expect=known_gap known_gap=E_PHP_RUNTIME_CONST_EXPR_MATRIX
function callable_default_fixture($callable = strlen(...)): void
{
    echo $callable("abcd"), "\n";
}
callable_default_fixture();

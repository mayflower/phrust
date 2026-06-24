<?php
// runtime-semantics: category=const_expr expect=known_gap known_gap=E_PHP_RUNTIME_CONST_EXPR_MATRIX
function closure_default_fixture($callback = static function (): string {
    return "closure";
}): void {
    echo $callback(), "\n";
}
closure_default_fixture();

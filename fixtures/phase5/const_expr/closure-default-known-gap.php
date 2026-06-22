<?php
// phase5-runtime: category=const_expr expect=known_gap known_gap=E_PHP_RUNTIME_CONST_EXPR_MATRIX
function prompt44_closure_default($callback = static function (): string {
    return "closure";
}): void {
    echo $callback(), "\n";
}
prompt44_closure_default();

<?php
function const_expr_closure_fixture($value = static function (): int {
    return 1;
}): void {}

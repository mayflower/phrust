<?php
// phase5-runtime: category=void_cast expect=known_gap known_gap=E_PHP_INVALID_VOID_CAST
function prompt44_void_side_effect(): int
{
    echo "side|";
    return 7;
}
(void) prompt44_void_side_effect();
echo "done\n";

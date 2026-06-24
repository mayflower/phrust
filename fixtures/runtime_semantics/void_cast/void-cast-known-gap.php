<?php
// runtime-semantics: category=void_cast expect=known_gap known_gap=E_PHP_INVALID_VOID_CAST
function void_cast_side_effect_fixture(): int
{
    echo "side|";
    return 7;
}
(void) void_cast_side_effect_fixture();
echo "done\n";

<?php
// runtime-semantics: category=types expect=known_gap known_gap=E_PHP_RUNTIME_TYPEERROR_TEXT_COMPAT
declare(strict_types=1);

function add_one(int $value): int {
    return $value + 1;
}

echo add_one("41"), "\n";

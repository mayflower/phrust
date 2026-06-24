<?php
// runtime-semantics: category=types expect=known_gap known_gap=E_PHP_RUNTIME_UNION_TYPEERROR_TEXT_COMPAT
function label(int|string $value): string {
    return "ok";
}

echo label([]), "\n";

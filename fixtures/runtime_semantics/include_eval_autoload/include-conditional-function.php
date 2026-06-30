<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_VM_CONDITIONAL_FUNCTION_DECLARATION_GAP
$enable = false;
include __DIR__ . "/_data/lib/conditional-function.php";
echo function_exists("include_conditional_declared_function") ? "declared" : "missing";
$enable = true;
include __DIR__ . "/_data/lib/conditional-function.php";
echo "|", include_conditional_declared_function(), "\n";

<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_RUNTIME_CONSTANT_REDECLARATION_WARNING_COMPAT
include __DIR__ . "/_data/lib/redeclare-constant.php";
include __DIR__ . "/_data/lib/redeclare-constant.php";

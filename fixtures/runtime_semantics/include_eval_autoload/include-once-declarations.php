<?php
$count = 0;
include_once __DIR__ . "/_data/lib/once-declarations.php";
include_once __DIR__ . "/_data/lib/once-declarations.php";
require_once __DIR__ . "/_data/lib/once-declarations.php";
echo include_once_declared_symbol_function(), "|";
echo IncludeOnceDeclaredSymbolClass::VALUE, "|";
echo INCLUDE_ONCE_DECLARED_SYMBOL_CONST, "|";
echo $count, "\n";

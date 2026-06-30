<?php
include __DIR__ . "/_data/lib/declarations.php";
echo include_declared_symbol_function(), "|";
echo IncludeDeclaredSymbolClass::VALUE, "|";
echo (new IncludeDeclaredSymbolClass())->value(), "|";
echo INCLUDE_DECLARED_SYMBOL_CONST, "\n";

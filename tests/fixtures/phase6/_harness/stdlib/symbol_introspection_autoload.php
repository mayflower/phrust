<?php
// phase6-diff: id=PHASE6_STDLIB_SYMBOL_INTROSPECTION_AUTOLOAD area=stdlib expect=pass
function phase6_symbol_autoload($name) { echo "autoload:", $name, "\n"; }
spl_autoload_register('phase6_symbol_autoload');
echo class_exists('MissingSymbol', false) ? "T\n" : "F\n";
echo interface_exists('MissingInterface', false) ? "T\n" : "F\n";
echo enum_exists('MissingEnum', false) ? "T\n" : "F\n";
echo class_exists('MissingSymbol', true) ? "T\n" : "F\n";
echo interface_exists('MissingInterface', true) ? "T\n" : "F\n";
echo enum_exists('MissingEnum', true) ? "T\n" : "F\n";

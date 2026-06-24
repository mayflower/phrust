<?php
// stdlib-diff: id=STDLIB_SYMBOL_INTROSPECTION_AUTOLOAD area=stdlib expect=pass
function stdlib_symbol_autoload($name) { echo "autoload:", $name, "\n"; }
spl_autoload_register('stdlib_symbol_autoload');
echo class_exists('MissingSymbol', false) ? "T\n" : "F\n";
echo interface_exists('MissingInterface', false) ? "T\n" : "F\n";
echo enum_exists('MissingEnum', false) ? "T\n" : "F\n";
echo class_exists('MissingSymbol', true) ? "T\n" : "F\n";
echo interface_exists('MissingInterface', true) ? "T\n" : "F\n";
echo enum_exists('MissingEnum', true) ? "T\n" : "F\n";

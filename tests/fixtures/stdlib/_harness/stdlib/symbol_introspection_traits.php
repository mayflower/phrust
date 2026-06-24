<?php
// stdlib-diff: id=STDLIB_SYMBOL_TRAIT_INTROSPECTION area=stdlib expect=known_gap known_gap=STDLIB-GAP-SYMBOL-TRAIT-INTROSPECTION
trait T {}
echo trait_exists('T', false) ? "T\n" : "F\n";
echo in_array('T', get_declared_traits(), true) ? "T\n" : "F\n";

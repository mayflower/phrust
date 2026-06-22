<?php
// phase5-runtime: expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_ARRAY_ELEMENT_REFERENCE
function &array_slot(&$items) {
    return $items[0];
}

$items = [1];
$x =& array_slot($items);
echo $x;

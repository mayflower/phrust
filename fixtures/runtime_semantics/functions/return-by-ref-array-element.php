<?php
// runtime-semantics: expect=pass
function &array_slot(&$items) {
    return $items[0];
}

$items = [1];
$x =& array_slot($items);
$x = 7;
echo $items[0];

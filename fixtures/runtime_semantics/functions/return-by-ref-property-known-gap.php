<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_PROPERTY_REFERENCE
function &property_slot($object) {
    return $object->value;
}

$object = new stdClass();
$object->value = 1;
$x =& property_slot($object);
echo $x;

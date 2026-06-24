<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_PROPERTY_REFERENCE
class Box {
    public int $value;
}

$box = new Box();
$box->value = 1;
$ref =& $box->value;
$ref = 2;
echo $box->value, "\n";

<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_PROPERTY_REFERENCE regression_category=refs-objects reference_behavior=stdout:2 regression_case=48
class Box {
    public $p = 1;
}
$box = new Box();
$alias =& $box->p;
$alias = 2;
echo $box->p;

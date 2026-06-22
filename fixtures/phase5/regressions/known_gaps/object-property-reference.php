<?php
// phase5-runtime: expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_PROPERTY_REFERENCE regression_category=refs-objects reference_behavior=stdout:2 fix_prompt=48
class Box {
    public $p = 1;
}
$box = new Box();
$alias =& $box->p;
$alias = 2;
echo $box->p;

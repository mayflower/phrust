<?php
class Phase7Box {
    public const SCALE = 2;
    public static $label = "props";
    public $value = 1;
}

$box = new Phase7Box();
for ($i = 0; $i < 4; $i++) {
    $box->value = $box->value * Phase7Box::SCALE;
    Phase7Box::$label;
}
echo Phase7Box::$label, ":", $box->value, "\n";

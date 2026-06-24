<?php
class PerfBox {
    public const SCALE = 2;
    public static $label = "props";
    public $value = 1;
}

$box = new PerfBox();
for ($i = 0; $i < 4; $i++) {
    $box->value = $box->value * PerfBox::SCALE;
    PerfBox::$label;
}
echo PerfBox::$label, ":", $box->value, "\n";

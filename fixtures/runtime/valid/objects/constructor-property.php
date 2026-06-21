<?php
class Box {
    public $value;

    public function __construct($value) {
        $this->value = $value;
    }
}

$box = new Box(7);
echo $box->value, "\n";

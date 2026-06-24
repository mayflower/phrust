<?php
// expect=skip
class Box {
    public $value;

    public function __construct() {
        $this->value = 4;
    }

    public function reader() {
        return function () {
            return $this->value;
        };
    }
}

$f = (new Box())->reader();
echo $f();

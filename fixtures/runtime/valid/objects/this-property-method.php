<?php
class ThisPropertyMethodFixture {
    public $value;

    function __construct($value) {
        $this->value = $value;
    }

    function get() {
        return $this->value;
    }

    function plus($value) {
        return $this->get() + $value;
    }
}

$box = new ThisPropertyMethodFixture(7);
echo $box->get(), "|", $box->plus(5), "\n";

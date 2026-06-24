<?php
class MethodCallCounterFixture {
    function add_one($value) {
        return $value + 1;
    }
}

$counter = new MethodCallCounterFixture();
echo $counter->add_one(4), "\n";

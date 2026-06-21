<?php
class Prompt27Counter {
    function add_one($value) {
        return $value + 1;
    }
}

$counter = new Prompt27Counter();
echo $counter->add_one(4), "\n";

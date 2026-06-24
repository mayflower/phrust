<?php
class CloneBoxFixture {
    public $value;
}

$original = new CloneBoxFixture();
$original->value = 1;
$copy = clone $original;
echo $original->value, "|", $copy->value, "\n";

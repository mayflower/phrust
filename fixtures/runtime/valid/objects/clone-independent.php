<?php
class CloneCellFixture {
    public $value;
}

$original = new CloneCellFixture();
$original->value = 1;
$copy = clone $original;
$copy->value = 2;
echo $original->value, "|", $copy->value, "\n";

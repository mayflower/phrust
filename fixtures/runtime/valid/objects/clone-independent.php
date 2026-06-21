<?php
class Prompt28CloneCell {
    public $value;
}

$original = new Prompt28CloneCell();
$original->value = 1;
$copy = clone $original;
$copy->value = 2;
echo $original->value, "|", $copy->value, "\n";

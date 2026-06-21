<?php
class Prompt28CloneBox {
    public $value;
}

$original = new Prompt28CloneBox();
$original->value = 1;
$copy = clone $original;
echo $original->value, "|", $copy->value, "\n";

<?php
class Prompt28CloneWithBox {
    public $name;
    public $count;
}

$original = new Prompt28CloneWithBox();
$original->name = "old";
$original->count = 1;
$copy = clone($original, ["name" => "new", "count" => 2]);
echo $original->name, ":", $original->count, "|", $copy->name, ":", $copy->count, "\n";

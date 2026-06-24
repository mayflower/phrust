<?php
class CloneWithBoxFixture {
    public $name;
    public $count;
}

$original = new CloneWithBoxFixture();
$original->name = "old";
$original->count = 1;
$copy = clone($original, ["name" => "new", "count" => 2]);
echo $original->name, ":", $original->count, "|", $copy->name, ":", $copy->count, "\n";

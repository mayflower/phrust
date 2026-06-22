<?php
// phase5-runtime: category=clone_with expect=pass
class CloneWithNoOriginalMutation {
    public string $name = "old";
    public int $count = 1;
}

$original = new CloneWithNoOriginalMutation();
$copy = clone($original, ["name" => "new", "count" => 2]);
echo $original->name . ":" . $original->count . "|" . $copy->name . ":" . $copy->count;

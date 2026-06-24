<?php
// runtime-semantics: category=clone_with expect=pass
class CloneShallowBox {
    public string $name = "old";
}

$original = new CloneShallowBox();
$copy = clone $original;
$copy->name = "new";
echo $original->name . "|" . $copy->name;

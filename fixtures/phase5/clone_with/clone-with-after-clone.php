<?php
// phase5-runtime: category=clone_with expect=pass
class CloneWithAfterClone {
    public string $name = "old";

    public function __clone(): void {
        $this->name = "cloned";
    }
}

$original = new CloneWithAfterClone();
$copy = clone($original, ["name" => "new"]);
echo $original->name . "|" . $copy->name;

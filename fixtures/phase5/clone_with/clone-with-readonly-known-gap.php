<?php
// phase5-runtime: category=clone_with expect=known_gap known_gap=E_PHP_VM_UNSUPPORTED_PROPERTY_MODIFIER
class CloneWithReadonlyGap {
    public readonly string $name;

    public function __construct() {
        $this->name = "old";
    }
}

$original = new CloneWithReadonlyGap();
$copy = clone($original, ["name" => "new"]);

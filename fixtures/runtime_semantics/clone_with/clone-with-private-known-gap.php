<?php
// runtime-semantics: category=clone_with expect=known_gap known_gap=E_PHP_VM_UNSUPPORTED_PROPERTY_MODIFIER
class CloneWithPrivateGap {
    private string $name = "old";
}

$original = new CloneWithPrivateGap();
$copy = clone($original, ["name" => "new"]);

<?php
// phase5-runtime: category=clone_with expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_PROPERTY_HOOKS
class CloneWithPropertyHookGap {
    public string $name {
        get { return "old"; }
        set { }
    }
}

$original = new CloneWithPropertyHookGap();
$copy = clone($original, ["name" => "new"]);

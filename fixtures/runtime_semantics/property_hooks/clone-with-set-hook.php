<?php
// runtime-semantics: category=property_hooks expect=pass
class HookCloneWith {
    public string $name {
        set { $this->name = strtoupper($value); }
        get { return $this->name; }
    }

    public function __construct() {
        $this->name = "old";
    }
}

$original = new HookCloneWith();
$copy = clone($original, ["name" => "new"]);
echo $original->name . "|" . $copy->name;

<?php
// runtime-semantics: category=property_hooks expect=pass
class HookSetBacked {
    public string $name {
        set { $this->name = strtoupper($value); }
        get { return $this->name; }
    }
}

$box = new HookSetBacked();
$box->name = "ada";
echo $box->name;

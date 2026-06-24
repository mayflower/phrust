<?php
// runtime-semantics: category=property_hooks expect=pass
class HookSetVirtual {
    public string $name {
        set { echo "set:" . $value; }
    }
}

$box = new HookSetVirtual();
$box->name = "virtual";

<?php
// phase5-runtime: category=property_hooks expect=known_gap known_gap=E_PHP_VM_PRIVATE_PROPERTY_SET_ACCESS
class HookPrivateSet {
    public private(set) string $name {
        set { $this->name = $value; }
        get { return $this->name; }
    }
}

$box = new HookPrivateSet();
$box->name = "new";

<?php
// runtime-semantics: category=property_hooks expect=known_gap known_gap=E_PHP_VM_PROPERTY_HOOK_RECURSION
class HookRecursion {
    public string $name {
        get { return $this->other; }
    }
    public string $other {
        get { return $this->name; }
    }
}

$box = new HookRecursion();
echo $box->name;

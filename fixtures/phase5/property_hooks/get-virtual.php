<?php
// phase5-runtime: category=property_hooks expect=pass
class HookGetVirtual {
    public string $name {
        get { return "virtual"; }
    }
}

$box = new HookGetVirtual();
echo $box->name;

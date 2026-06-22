<?php
// phase5-runtime: category=property_hooks expect=pass
class HookGetBacked {
    public string $name {
        set { $this->name = $value; }
        get { return $this->name . "!"; }
    }
}

$box = new HookGetBacked();
$box->name = "Ada";
echo $box->name;

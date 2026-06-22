<?php
// phase5-runtime: category=property_hooks expect=pass
class HookTypeCheck {
    public int $number {
        set { $this->number = $value; }
        get { return $this->number; }
    }
}

$box = new HookTypeCheck();
$box->number = 7;
echo $box->number;

<?php
// phase4: kind=known_gap id=E_PHP_IR_UNSUPPORTED_PROPERTY_HOOKS
class Phase4PropertyHookGap
{
    public string $name {
        get {
            return "hook";
        }
    }
}

echo (new Phase4PropertyHookGap())->name;

<?php
// phase5: kind=pass id=property-hook-get
class Phase4PropertyHookGap
{
    public string $name {
        get {
            return "hook";
        }
    }
}

echo (new Phase4PropertyHookGap())->name;

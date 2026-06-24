<?php
// runtime-fixture: kind=pass id=property-hook-get
class RuntimePropertyHookGap
{
    public string $name {
        get {
            return "hook";
        }
    }
}

echo (new RuntimePropertyHookGap())->name;

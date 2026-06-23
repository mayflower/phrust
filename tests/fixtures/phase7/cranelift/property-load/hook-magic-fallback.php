<?php
class Phase7PropertyLoadMagicFallback {
    public function __get(string $name): int {
        return strlen($name);
    }
}

class Phase7PropertyLoadHookFallback {
    public int $value {
        get {
            return 11;
        }
    }
}

function phase7_property_load_magic(Phase7PropertyLoadMagicFallback $object): int {
    return $object->value;
}

function phase7_property_load_hook(Phase7PropertyLoadHookFallback $object): int {
    return $object->value;
}

echo phase7_property_load_magic(new Phase7PropertyLoadMagicFallback()), "|";
echo phase7_property_load_hook(new Phase7PropertyLoadHookFallback()), "\n";

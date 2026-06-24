<?php
class PerfPropertyLoadMagicFallback {
    public function __get(string $name): int {
        return strlen($name);
    }
}

class PerfPropertyLoadHookFallback {
    public int $value {
        get {
            return 11;
        }
    }
}

function perf_property_load_magic(PerfPropertyLoadMagicFallback $object): int {
    return $object->value;
}

function perf_property_load_hook(PerfPropertyLoadHookFallback $object): int {
    return $object->value;
}

echo perf_property_load_magic(new PerfPropertyLoadMagicFallback()), "|";
echo perf_property_load_hook(new PerfPropertyLoadHookFallback()), "\n";

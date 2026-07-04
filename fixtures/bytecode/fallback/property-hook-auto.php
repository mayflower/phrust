<?php
class BytecodePropertyHookTarget {
    public string $value {
        get {
            return "hook";
        }
    }
}

// Dynamic instantiation keeps this a rich-planned auto-fallback probe.
$class = 'BytecodePropertyHookTarget';
echo (new $class())->value, "\n";

<?php
class BytecodePropertyHookTarget {
    public string $value {
        get {
            return "hook";
        }
    }
}

echo (new BytecodePropertyHookTarget())->value, "\n";

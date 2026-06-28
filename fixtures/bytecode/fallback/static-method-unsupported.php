<?php
class BytecodeStaticTarget {
    public static function value() {
        return 1;
    }
}

function bytecode_static_method_call_target() {
    return BytecodeStaticTarget::value();
}

echo "static-method-fenced\n";

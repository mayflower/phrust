<?php
class BytecodeStaticAutoTarget {
    public static function value($input) {
        return $input + 5;
    }
}

echo BytecodeStaticAutoTarget::value(3), "\n";

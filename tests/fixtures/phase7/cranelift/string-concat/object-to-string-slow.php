<?php
class Phase7CraneliftConcatObject {
    public function __toString(): string {
        return "object";
    }
}

function phase7_cranelift_concat_object($lhs, $rhs): string {
    return $lhs . $rhs;
}

echo phase7_cranelift_concat_object("value:", new Phase7CraneliftConcatObject()), "\n";

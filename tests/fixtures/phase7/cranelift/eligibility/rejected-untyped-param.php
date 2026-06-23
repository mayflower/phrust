<?php
function phase7_cranelift_rejected_untyped($value): int {
    return $value + 1;
}

echo phase7_cranelift_rejected_untyped(4), "\n";

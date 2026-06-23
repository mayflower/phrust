<?php
function phase7_cranelift_rejected_call(string $value): int {
    return strlen($value);
}

echo phase7_cranelift_rejected_call("abcd"), "\n";

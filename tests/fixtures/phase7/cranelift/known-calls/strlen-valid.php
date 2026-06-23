<?php
function phase7_cranelift_strlen_valid(string $value): int {
    return strlen($value);
}

echo phase7_cranelift_strlen_valid("hello"), "\n";

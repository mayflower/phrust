<?php
function phase7_cranelift_strlen_wrong_arity(string $value): int {
    return strlen($value, 1);
}

try {
    echo phase7_cranelift_strlen_wrong_arity("hello"), "\n";
} catch (ArgumentCountError $e) {
    echo "arity-error\n";
}

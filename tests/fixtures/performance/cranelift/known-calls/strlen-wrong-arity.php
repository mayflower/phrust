<?php
function perf_cranelift_strlen_wrong_arity(string $value): int {
    return strlen($value, 1);
}

try {
    echo perf_cranelift_strlen_wrong_arity("hello"), "\n";
} catch (ArgumentCountError $e) {
    echo "arity-error\n";
}

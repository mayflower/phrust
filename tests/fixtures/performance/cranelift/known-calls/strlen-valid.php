<?php
function perf_cranelift_strlen_valid(string $value): int {
    return strlen($value);
}

echo perf_cranelift_strlen_valid("hello"), "\n";

<?php
function perf_cranelift_count_packed(array $value): int {
    return count($value);
}

echo perf_cranelift_count_packed([10, 20, 30]), "\n";

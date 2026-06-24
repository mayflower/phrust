<?php
function perf_cranelift_count_mixed(array $value): int {
    return count($value);
}

echo perf_cranelift_count_mixed(["a" => 10, 4 => 20, "b" => 30]), "\n";

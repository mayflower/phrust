<?php
function perf_cranelift_rejected_array(int $value): int {
    $items = [$value];
    return $items[0];
}

echo perf_cranelift_rejected_array(7), "\n";

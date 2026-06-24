<?php
function perf_cranelift_eligible_int_leaf(int $a, int $b): int {
    $sum = $a + $b;
    if ($sum > 10) {
        return $sum - 1;
    }
    return $sum * 2;
}

echo perf_cranelift_eligible_int_leaf(4, 5), "\n";

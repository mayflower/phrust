<?php
function perf_cranelift_packed_foreach_sum_by_ref_non_eligible(array $xs): int {
    $sum = 0;
    foreach ($xs as &$x) {
        $sum += $x;
    }
    return $sum;
}

echo perf_cranelift_packed_foreach_sum_by_ref_non_eligible([10, 20, 30]), "\n";

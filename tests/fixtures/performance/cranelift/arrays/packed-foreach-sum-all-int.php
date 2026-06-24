<?php
function perf_cranelift_packed_foreach_sum_all_int(array $xs): int {
    $sum = 0;
    foreach ($xs as $x) {
        $sum += $x;
    }
    return $sum;
}

echo perf_cranelift_packed_foreach_sum_all_int([10, 20, 30]), "\n";

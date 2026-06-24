<?php
function perf_cranelift_packed_foreach_sum_overflow(array $xs): int {
    $sum = 0;
    foreach ($xs as $x) {
        $sum += $x;
    }
    return $sum;
}

echo perf_cranelift_packed_foreach_sum_overflow([9223372036854775807, 1]), "\n";

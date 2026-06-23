<?php
function phase7_cranelift_packed_foreach_sum_empty(array $xs): int {
    $s = 0;
    foreach ($xs as $x) {
        $s += $x;
    }
    return $s;
}

echo phase7_cranelift_packed_foreach_sum_empty([]), "\n";

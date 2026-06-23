<?php
function phase7_cranelift_packed_foreach_sum_mixed_element(array $xs): int {
    $total = 0;
    foreach ($xs as $value) {
        $total += $value;
    }
    return $total;
}

echo phase7_cranelift_packed_foreach_sum_mixed_element([10, "20", 30]), "\n";

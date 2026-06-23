<?php
function phase7_cranelift_packed_foreach_sum_body_mutation_non_eligible(array $xs): int {
    $sum = 0;
    foreach ($xs as $x) {
        $xs[] = 1;
        $sum += $x;
    }
    return $sum;
}

echo phase7_cranelift_packed_foreach_sum_body_mutation_non_eligible([10, 20, 30]), "\n";

<?php
function phase7_cranelift_sum_1_to_n(int $n): int
{
    $sum = 0;
    for ($i = 1; $i <= $n; $i = $i + 1) {
        $sum = $sum + $i;
    }
    return $sum;
}

echo phase7_cranelift_sum_1_to_n(10), "\n";

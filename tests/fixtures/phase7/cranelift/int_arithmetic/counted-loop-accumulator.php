<?php
function phase7_cranelift_counted_loop(int $n): int
{
    $sum = 0;
    for ($i = 0; $i < $n; $i = $i + 1) {
        $sum = $sum + $i;
    }
    return $sum;
}

echo phase7_cranelift_counted_loop(5), "\n";

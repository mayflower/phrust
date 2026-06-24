<?php
function perf_cranelift_branchy_max_min(int $a, int $b): int
{
    if ($a > $b) {
        $max = $a;
        $min = $b;
    } else {
        $max = $b;
        $min = $a;
    }
    return $max - $min;
}

echo perf_cranelift_branchy_max_min(9, 4), "\n";
echo perf_cranelift_branchy_max_min(-3, 8), "\n";

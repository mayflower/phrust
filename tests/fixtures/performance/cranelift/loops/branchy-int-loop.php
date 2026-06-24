<?php
function perf_cranelift_branchy_int_loop(int $n): int
{
    $acc = 0;
    for ($i = 0; $i < $n; $i = $i + 1) {
        if ($i < 5) {
            $acc = $acc + $i;
        } else {
            $acc = $acc - $i;
        }
    }
    return $acc;
}

echo perf_cranelift_branchy_int_loop(10), "\n";

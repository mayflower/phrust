<?php
function perf_cranelift_fib_iterative(int $n): int
{
    $a = 0;
    $b = 1;
    for ($i = 0; $i < $n; $i = $i + 1) {
        $next = $a + $b;
        $a = $b;
        $b = $next;
    }
    return $a;
}

echo perf_cranelift_fib_iterative(10), "\n";

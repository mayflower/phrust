<?php
function perf_cranelift_int_add_repeat(int $a, int $b): int
{
    return $a + $b;
}

echo perf_cranelift_int_add_repeat(1, 2), "\n";
echo perf_cranelift_int_add_repeat(3, 4), "\n";
echo perf_cranelift_int_add_repeat(5, 6), "\n";
echo perf_cranelift_int_add_repeat(7, 8), "\n";
echo perf_cranelift_int_add_repeat(9, 10), "\n";

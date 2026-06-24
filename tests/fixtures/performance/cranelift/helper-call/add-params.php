<?php
function perf_cranelift_helper_add(int $a, int $b): int
{
    return $a + $b;
}

echo perf_cranelift_helper_add(20, 22), "\n";

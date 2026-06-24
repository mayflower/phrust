<?php
function perf_cranelift_negative_ints(int $a, int $b, int $c): int
{
    return ($a - $b) * $c;
}

echo perf_cranelift_negative_ints(-7, 5, -3), "\n";

<?php
function perf_cranelift_overflow_correctness(int $a): int
{
    return $a + 1;
}

echo perf_cranelift_overflow_correctness(9223372036854775807), "\n";

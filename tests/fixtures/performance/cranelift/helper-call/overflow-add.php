<?php
function perf_cranelift_helper_overflow(int $a): int
{
    return $a + 1;
}

echo perf_cranelift_helper_overflow(9223372036854775807), "\n";

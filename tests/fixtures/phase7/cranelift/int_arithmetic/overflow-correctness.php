<?php
function phase7_cranelift_overflow_correctness(int $a): int
{
    return $a + 1;
}

echo phase7_cranelift_overflow_correctness(9223372036854775807), "\n";

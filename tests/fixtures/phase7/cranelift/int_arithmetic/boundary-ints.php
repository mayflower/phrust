<?php
function phase7_cranelift_boundary_ints(int $a, int $b): int
{
    return $a + $b;
}

echo phase7_cranelift_boundary_ints(9223372036854775806, 1), "\n";

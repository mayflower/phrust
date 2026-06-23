<?php
function phase7_cranelift_helper_add_mul(int $a): int
{
    return ($a + 2) * 3;
}

echo phase7_cranelift_helper_add_mul(4), "\n";

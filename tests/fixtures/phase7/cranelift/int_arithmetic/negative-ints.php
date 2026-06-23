<?php
function phase7_cranelift_negative_ints(int $a, int $b, int $c): int
{
    return ($a - $b) * $c;
}

echo phase7_cranelift_negative_ints(-7, 5, -3), "\n";

<?php
function phase7_cranelift_factorial_like(int $n): int
{
    $result = 1;
    for ($i = 1; $i <= $n; $i = $i + 1) {
        $result = $result * $i;
    }
    return $result;
}

echo phase7_cranelift_factorial_like(5), "\n";

<?php
function phase7_cranelift_loop_helper($x)
{
    return $x + 1;
}

function phase7_cranelift_loop_with_call(int $n): int
{
    $sum = 0;
    for ($i = 0; $i < $n; $i = $i + 1) {
        $sum = $sum + phase7_cranelift_loop_helper($i);
    }
    return $sum;
}

echo phase7_cranelift_loop_with_call(5), "\n";

<?php
function perf_cranelift_loop_helper($x)
{
    return $x + 1;
}

function perf_cranelift_loop_with_call(int $n): int
{
    $sum = 0;
    for ($i = 0; $i < $n; $i = $i + 1) {
        $sum = $sum + perf_cranelift_loop_helper($i);
    }
    return $sum;
}

echo perf_cranelift_loop_with_call(5), "\n";

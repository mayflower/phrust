<?php
function perf_cranelift_int_chain(int $a): int
{
    return (($a + 2) * 3) + 4;
}

echo perf_cranelift_int_chain(5), "\n";

<?php
function perf_cranelift_unstable_type_switch(int $a): int
{
    return $a + 1;
}

echo perf_cranelift_unstable_type_switch(1), "\n";
echo perf_cranelift_unstable_type_switch("2"), "\n";
echo perf_cranelift_unstable_type_switch("3"), "\n";
echo perf_cranelift_unstable_type_switch(4), "\n";

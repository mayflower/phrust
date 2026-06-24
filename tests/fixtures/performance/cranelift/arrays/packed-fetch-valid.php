<?php
function perf_cranelift_packed_fetch_valid(array $xs, int $i): int {
    return $xs[$i];
}

$xs = [10, 20, 30];
echo perf_cranelift_packed_fetch_valid($xs, 1), "\n";

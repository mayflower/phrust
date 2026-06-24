<?php
function perf_cranelift_packed_fetch_out_of_bounds(array $xs, int $i): int {
    return $xs[$i];
}

echo perf_cranelift_packed_fetch_out_of_bounds([10], 4), "\n";

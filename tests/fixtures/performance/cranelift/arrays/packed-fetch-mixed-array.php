<?php
function perf_cranelift_packed_fetch_mixed_array(array $xs, int $i): int {
    return $xs[$i];
}

$xs = [0 => 11, "name" => 12];
echo perf_cranelift_packed_fetch_mixed_array($xs, 0), "\n";
